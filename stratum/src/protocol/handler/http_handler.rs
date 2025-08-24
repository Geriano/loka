//! HTTP request handling and CONNECT tunneling implementation.
//!
//! This module provides specialized handling for HTTP requests including:
//! - Direct HTTP request processing (GET/POST)
//! - HTTP CONNECT method for tunneling
//! - Path extraction and validation
//! - Error response generation

use std::net::SocketAddr;
use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tracing::{debug, info, trace, warn};

use crate::Manager;
use crate::error::Result;
use crate::services::pool_config::PoolConfigService;

/// HTTP request handler for processing HTTP-specific protocol messages.
///
/// Handles both direct HTTP requests and HTTP CONNECT tunneling for mining
/// proxy scenarios. Supports path extraction, target validation, and error
/// response generation.
///
/// # Examples
///
/// ```rust,no_run
/// use loka_stratum::protocol::handler::http_handler::HttpHandler;
/// use loka_stratum::services::pool_config::{PoolConfigService, PoolConfigServiceConfig};
/// use loka_stratum::services::database::DatabaseService;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let database = Arc::new(DatabaseService::new("sqlite::memory:").await?);
/// let config = PoolConfigServiceConfig::default();
/// let pool_service = Arc::new(PoolConfigService::new(database, config));
/// let handler = HttpHandler::new(pool_service);
/// let is_http = HttpHandler::is_direct_http_request("GET /status HTTP/1.1");
/// assert!(is_http);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct HttpHandler {
    /// Pool configuration service for target validation
    pool_service: Arc<PoolConfigService>,
}

impl HttpHandler {
    /// Create a new HTTP handler.
    ///
    /// # Arguments
    ///
    /// * `pool_service` - Pool configuration service for validating targets
    pub fn new(pool_service: Arc<PoolConfigService>) -> Self {
        Self { pool_service }
    }

    /// Check if a message line represents a direct HTTP request.
    ///
    /// Detects HTTP methods like GET, POST, PUT, DELETE, etc.
    ///
    /// # Arguments
    ///
    /// * `line` - The message line to check
    ///
    /// # Returns
    ///
    /// True if the line appears to be an HTTP request
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::protocol::handler::http_handler::HttpHandler;
    ///
    /// assert!(HttpHandler::is_direct_http_request("GET /status HTTP/1.1"));
    /// assert!(HttpHandler::is_direct_http_request("POST /submit HTTP/1.1"));
    /// assert!(!HttpHandler::is_direct_http_request("{\"method\":\"mining.subscribe\"}"));
    /// ```
    pub fn is_direct_http_request(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return false;
        }

        let http_methods = [
            "GET ", "POST ", "PUT ", "DELETE ", "HEAD ", "OPTIONS ", "PATCH ", "TRACE ",
        ];

        http_methods
            .iter()
            .any(|method| trimmed.starts_with(method) && trimmed.contains(" HTTP/"))
    }

    /// Parse the path from an HTTP request line.
    ///
    /// Extracts the path component from HTTP request lines like "GET /path HTTP/1.1".
    ///
    /// # Arguments
    ///
    /// * `line` - The HTTP request line
    ///
    /// # Returns
    ///
    /// The extracted path, or None if parsing fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::protocol::handler::http_handler::HttpHandler;
    ///
    /// let path = HttpHandler::parse_http_request_path("GET /api/status HTTP/1.1");
    /// assert_eq!(path, Some("/api/status".to_string()));
    /// ```
    pub fn parse_http_request_path(line: &str) -> Option<String> {
        let trimmed = line.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        if parts.len() >= 3 && parts[2].starts_with("HTTP/") {
            // Standard HTTP request format: METHOD PATH HTTP/VERSION
            Some(parts[1].to_string())
        } else if parts.len() >= 2 {
            // Fallback: METHOD PATH (missing HTTP version)
            Some(parts[1].to_string())
        } else {
            None
        }
    }

    /// Extract path information from various message formats.
    ///
    /// Attempts to extract path/target information from different message types
    /// including HTTP requests and JSON-formatted Stratum messages.
    ///
    /// # Arguments
    ///
    /// * `raw_message` - The raw message to parse
    ///
    /// # Returns
    ///
    /// Extracted path if found
    pub fn extract_path_from_message(raw_message: &str) -> Option<String> {
        let line = raw_message.trim();

        // Try HTTP request format first
        if let Some(path) = Self::try_extract_http_path(line) {
            return Some(path);
        }

        // Try JSON Stratum format
        if let Some(path) = Self::try_extract_stratum_path(line) {
            return Some(path);
        }

        // Try to find URL-like patterns in the text
        Self::parse_url_path_from_string(line)
    }

    /// Attempt to extract path from HTTP request format.
    fn try_extract_http_path(line: &str) -> Option<String> {
        if Self::is_direct_http_request(line) {
            Self::parse_http_request_path(line)
        } else {
            None
        }
    }

    /// Attempt to extract path from Stratum JSON format.
    fn try_extract_stratum_path(line: &str) -> Option<String> {
        if let Ok(json) = serde_json::from_str::<Value>(line) {
            Self::extract_path_from_json(&json)
        } else {
            None
        }
    }

    /// Extract path from JSON message structure.
    fn extract_path_from_json(json: &Value) -> Option<String> {
        // Try mining.subscribe format first
        if let Some(path) = Self::extract_path_from_mining_subscribe(json) {
            return Some(path);
        }

        // Search recursively for URL-like patterns
        Self::search_json_for_url_path(json)
    }

    /// Extract path from mining.subscribe message format.
    fn extract_path_from_mining_subscribe(json: &Value) -> Option<String> {
        if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
            if method == "mining.subscribe" {
                if let Some(params) = json.get("params").and_then(|p| p.as_array()) {
                    // Look for URL in params array
                    for param in params {
                        if let Some(param_str) = param.as_str() {
                            if param_str.contains("://") || param_str.starts_with('/') {
                                return Some(param_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Parse URL path from a text string using regex-like patterns.
    fn parse_url_path_from_string(text: &str) -> Option<String> {
        // Look for common URL patterns
        let patterns = [
            "http://",
            "https://",
            "stratum://",
            "stratum+tcp://",
            "stratum+ssl://",
        ];

        for pattern in &patterns {
            if let Some(start) = text.find(pattern) {
                let url_start = start;
                let rest = &text[url_start..];

                // Find the end of the URL (space, quote, or end of string)
                let url_end = rest
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .unwrap_or(rest.len());

                let full_url = &rest[..url_end];

                // Extract just the path part
                if let Some(path_start) = full_url.find("://") {
                    let after_scheme = &full_url[path_start + 3..];
                    if let Some(path_pos) = after_scheme.find('/') {
                        return Some(after_scheme[path_pos..].to_string());
                    }
                }
            }
        }

        // Look for paths starting with '/'
        if text.starts_with('/') {
            if let Some(end) = text.find(|c: char| c.is_whitespace()) {
                return Some(text[..end].to_string());
            } else {
                return Some(text.to_string());
            }
        }

        None
    }

    /// Recursively search JSON structure for URL-like paths.
    fn search_json_for_url_path(value: &Value) -> Option<String> {
        match value {
            Value::String(s) => {
                if s.contains("://") || s.starts_with('/') {
                    Some(s.clone())
                } else {
                    None
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    if let Some(path) = Self::search_json_for_url_path(item) {
                        return Some(path);
                    }
                }
                None
            }
            Value::Object(obj) => {
                for (_, val) in obj {
                    if let Some(path) = Self::search_json_for_url_path(val) {
                        return Some(path);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Parse CONNECT target from HTTP CONNECT message.
    ///
    /// Extracts the target host:port from HTTP CONNECT requests.
    ///
    /// # Arguments
    ///
    /// * `message` - The raw CONNECT message
    ///
    /// # Returns
    ///
    /// The target host:port if parsing succeeds
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::protocol::handler::http_handler::HttpHandler;
    ///
    /// let target = HttpHandler::parse_connect_target_from_message("CONNECT pool.example.com:4444 HTTP/1.1");
    /// assert_eq!(target, Some("pool.example.com:4444".to_string()));
    /// ```
    pub fn parse_connect_target_from_message(message: &str) -> Option<String> {
        let line = message.trim();

        if !line.starts_with("CONNECT ") {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            Some(parts[1].to_string())
        } else {
            None
        }
    }

    /// Handle a complete HTTP request with headers and body.
    ///
    /// Processes multi-line HTTP requests by reading headers and handling
    /// the request appropriately based on the method and path.
    pub async fn handle_complete_http_request(
        &self,
        reader: &mut BufReader<OwnedReadHalf>,
        request_line: &str,
        addr: SocketAddr,
        _downstream_to_upstream_tx: &mpsc::UnboundedSender<Value>,
        manager: &Manager,
    ) -> Result<()> {
        info!(
            "miner {} - Processing complete HTTP request: {}",
            addr, request_line
        );

        // Parse the request line
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            self.send_http_error_response(reader, 400, "Bad Request")
                .await?;
            return Ok(());
        }

        let method = parts[0];
        let path = parts[1];

        // Read headers
        let mut headers = Vec::new();
        let mut content_length = 0usize;

        loop {
            let mut header_line = String::new();
            match reader.read_line(&mut header_line).await {
                Ok(0) => break, // End of stream
                Ok(_) => {
                    let header = header_line.trim();
                    if header.is_empty() {
                        break; // End of headers
                    }

                    // Check for Content-Length
                    if let Some(length_str) = header.strip_prefix("Content-Length: ") {
                        if let Ok(length) = length_str.parse::<usize>() {
                            content_length = length;
                        }
                    }

                    headers.push(header.to_string());
                }
                Err(e) => {
                    warn!("miner {} - Error reading HTTP header: {}", addr, e);
                    break;
                }
            }
        }

        // Read body if present
        let mut body = String::new();
        if content_length > 0 {
            let mut buffer = vec![0u8; content_length];
            match tokio::io::AsyncReadExt::read_exact(reader, &mut buffer).await {
                Ok(_) => {
                    body = String::from_utf8_lossy(&buffer).to_string();
                }
                Err(e) => {
                    warn!("miner {} - Error reading HTTP body: {}", addr, e);
                }
            }
        }

        debug!(
            "miner {} - HTTP request details - Method: {}, Path: {}, Headers: {}, Body length: {}",
            addr,
            method,
            path,
            headers.len(),
            body.len()
        );

        // Handle different HTTP methods
        match method {
            "GET" => self.handle_get_request(path, addr, manager).await,
            "POST" => self.handle_post_request(path, &body, addr, manager).await,
            "CONNECT" => {
                // CONNECT should be handled by the connection logic, not here
                self.send_http_error_response(reader, 405, "Method Not Allowed")
                    .await?;
                Ok(())
            }
            _ => {
                self.send_http_error_response(reader, 405, "Method Not Allowed")
                    .await?;
                Ok(())
            }
        }
    }

    /// Handle GET requests for status and information endpoints.
    async fn handle_get_request(
        &self,
        path: &str,
        addr: SocketAddr,
        _manager: &Manager,
    ) -> Result<()> {
        debug!("miner {} - Handling GET request for path: {}", addr, path);

        match path {
            "/" | "/status" => {
                // Return basic status information
                // This would typically send a response back, but we don't have the writer here
                // In a real implementation, this would be structured differently
                info!("miner {} - Status request handled", addr);
            }
            "/metrics" => {
                // Return metrics information
                info!("miner {} - Metrics request handled", addr);
            }
            _ => {
                info!("miner {} - Unknown GET path: {}", addr, path);
            }
        }

        Ok(())
    }

    /// Handle POST requests for data submission.
    async fn handle_post_request(
        &self,
        path: &str,
        body: &str,
        addr: SocketAddr,
        _manager: &Manager,
    ) -> Result<()> {
        debug!(
            "miner {} - Handling POST request for path: {} with body: {}",
            addr, path, body
        );

        // Try to parse body as JSON for potential Stratum messages
        if let Ok(json) = serde_json::from_str::<Value>(body) {
            trace!("miner {} - POST body parsed as JSON: {:?}", addr, json);
            // Could forward this to the Stratum handler
        }

        Ok(())
    }

    /// Validate that a CONNECT target is allowed.
    ///
    /// Checks the target against configured pool addresses and security policies.
    pub async fn validate_connect_target(&self, target: &str, addr: SocketAddr) -> Result<bool> {
        debug!("miner {} - Validating CONNECT target: {}", addr, target);

        // Parse target as host:port
        let parts: Vec<&str> = target.split(':').collect();
        if parts.len() != 2 {
            warn!("miner {} - Invalid CONNECT target format: {}", addr, target);
            return Ok(false);
        }

        let host = parts[0];
        let port = parts[1];

        // Parse port
        if port.parse::<u16>().is_err() {
            warn!("miner {} - Invalid port in CONNECT target: {}", addr, port);
            return Ok(false);
        }

        // Check against pool configurations using the pool service
        let port_num: u16 = port.parse().unwrap(); // Already validated above
        let is_valid = self.pool_service.is_valid_target(host, port_num).await;

        if !is_valid {
            warn!("miner {} - CONNECT target not allowed: {}", addr, target);
        } else {
            info!("miner {} - CONNECT target validated: {}", addr, target);
        }

        Ok(is_valid)
    }

    /// Send an HTTP error response.
    async fn send_http_error_response(
        &self,
        _reader: &mut BufReader<OwnedReadHalf>,
        status_code: u16,
        status_text: &str,
    ) -> Result<()> {
        // Note: In a real implementation, we would need a writer, not a reader
        // This is a structural issue that would be resolved in the actual refactoring
        warn!("Would send HTTP {} response: {}", status_code, status_text);
        Ok(())
    }

    /// Format an HTTP response with the given status.
    pub fn format_http_response(status_code: u16, status_text: &str) -> String {
        format!(
            "HTTP/1.1 {status_code} {status_text}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        )
    }

    /// Send a direct HTTP response to the client.
    ///
    /// Formats and sends an HTTP response with optional body content.
    pub async fn send_direct_http_response(
        writer: &mut BufWriter<OwnedWriteHalf>,
        status_code: u16,
        status_text: &str,
        body: Option<&str>,
    ) -> Result<()> {
        let response = if let Some(body_content) = body {
            format!(
                "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_code,
                status_text,
                body_content.len(),
                body_content
            )
        } else {
            Self::format_http_response(status_code, status_text)
        };

        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;

        trace!("Sent HTTP response: {} {}", status_code, status_text);
        Ok(())
    }
}
