use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

/// HTTP to Stratum proxy that can extract path information
/// This handles miners that connect with URLs like stratum+tcp://host:port/order-1
pub struct HttpStratumProxy {
    bind_addr: SocketAddr,
    stratum_addr: SocketAddr,
}

impl HttpStratumProxy {
    pub fn new(bind_addr: SocketAddr, stratum_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            stratum_addr,
        }
    }

    pub async fn start(&self) -> io::Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("HTTP-Stratum proxy listening on {}", self.bind_addr);

        loop {
            match listener.accept().await {
                Ok((client_stream, client_addr)) => {
                    let stratum_addr = self.stratum_addr;
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(client_stream, client_addr, stratum_addr).await {
                            error!("Error handling connection from {}: {}", client_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        client_stream: TcpStream,
        client_addr: SocketAddr,
        stratum_addr: SocketAddr,
    ) -> io::Result<()> {
        debug!("New connection from {}", client_addr);

        // Try to parse the first line to see if it's HTTP or direct Stratum
        let mut buf_reader = BufReader::new(&client_stream);
        let mut first_line = String::new();

        // Peek at the first line without consuming the stream
        match buf_reader.read_line(&mut first_line).await {
            Ok(0) => {
                debug!("Connection closed by client {}", client_addr);
                return Ok(());
            }
            Ok(_) => {
                let path = Self::extract_path_from_request(&first_line);
                if let Some(ref p) = path {
                    info!("Client {} - Path extracted: {}", client_addr, p);
                } else {
                    debug!("Client {} - No path found in request", client_addr);
                }

                // Connect to the actual Stratum server
                let stratum_stream = TcpStream::connect(stratum_addr).await?;
                info!("Connected to Stratum server {} for client {}", stratum_addr, client_addr);

                // If it's an HTTP request, convert it to Stratum
                if first_line.trim().starts_with("GET ") || first_line.trim().starts_with("POST ") {
                    Self::handle_http_to_stratum(client_stream, stratum_stream, client_addr, path).await
                } else {
                    // It's already Stratum protocol, just proxy it
                    // But we need to forward the first line we already read
                    let mut stratum_writer = BufWriter::new(stratum_stream);
                    stratum_writer.write_all(first_line.as_bytes()).await?;
                    stratum_writer.flush().await?;

                    // Now proxy the rest
                    Self::proxy_bidirectional(client_stream, stratum_writer.into_inner(), client_addr).await
                }
            }
            Err(e) => {
                error!("Failed to read from client {}: {}", client_addr, e);
                return Err(e);
            }
        }
    }

    fn extract_path_from_request(request_line: &str) -> Option<String> {
        // Parse HTTP GET request: "GET /order-1 HTTP/1.1"
        if request_line.trim().starts_with("GET ") {
            let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                let path = parts[1];
                if path.starts_with('/') && path.len() > 1 {
                    return Some(path[1..].to_string()); // Remove leading slash
                }
            }
        }

        // Try to parse from JSON if it contains URL info
        if request_line.trim().starts_with('{') {
            // This is probably a Stratum JSON message, try to extract URL from it
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(request_line.trim()) {
                if let Some(params) = json.get("params").and_then(|p| p.as_array()) {
                    for param in params {
                        if let Some(param_str) = param.as_str() {
                            if param_str.contains("stratum+tcp://") {
                                if let Some(path_start) = param_str.rfind('/') {
                                    let path = &param_str[path_start + 1..];
                                    if !path.is_empty() && path != param_str {
                                        return Some(path.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    async fn handle_http_to_stratum(
        client_stream: TcpStream,
        stratum_stream: TcpStream,
        client_addr: SocketAddr,
        path: Option<String>,
    ) -> io::Result<()> {
        info!("Converting HTTP to Stratum for client {} (path: {:?})", client_addr, path);

        // Send HTTP response headers to switch to raw TCP
        let mut client_writer = BufWriter::new(&client_stream);
        client_writer.write_all(b"HTTP/1.1 200 OK\r\n").await?;
        client_writer.write_all(b"Connection: Upgrade\r\n").await?;
        client_writer.write_all(b"Upgrade: stratum\r\n").await?;
        client_writer.write_all(b"\r\n").await?;
        client_writer.flush().await?;

        // Now proxy the connection
        Self::proxy_bidirectional(client_stream, stratum_stream, client_addr).await
    }

    async fn proxy_bidirectional(
        client_stream: TcpStream,
        stratum_stream: TcpStream,
        client_addr: SocketAddr,
    ) -> io::Result<()> {
        let (client_read, client_write) = client_stream.into_split();
        let (stratum_read, stratum_write) = stratum_stream.into_split();

        let client_to_stratum = Self::proxy_data(client_read, stratum_write, "client->stratum", client_addr);
        let stratum_to_client = Self::proxy_data(stratum_read, client_write, "stratum->client", client_addr);

        // Run both directions concurrently
        let (result1, result2) = tokio::join!(client_to_stratum, stratum_to_client);

        if let Err(e) = result1 {
            debug!("Client->Stratum proxy error for {}: {}", client_addr, e);
        }
        if let Err(e) = result2 {
            debug!("Stratum->Client proxy error for {}: {}", client_addr, e);
        }

        info!("Connection closed for {}", client_addr);
        Ok(())
    }

    async fn proxy_data(
        mut reader: tokio::net::tcp::OwnedReadHalf,
        mut writer: tokio::net::tcp::OwnedWriteHalf,
        direction: &str,
        client_addr: SocketAddr,
    ) -> io::Result<()> {
        let mut buf = [0; 4096];
        loop {
            match tokio::io::AsyncReadExt::read(&mut reader, &mut buf).await {
                Ok(0) => {
                    debug!("{} connection closed for {}", direction, client_addr);
                    break;
                }
                Ok(n) => {
                    tokio::io::AsyncWriteExt::write_all(&mut writer, &buf[..n]).await?;
                    tokio::io::AsyncWriteExt::flush(&mut writer).await?;
                }
                Err(e) => {
                    debug!("{} read error for {}: {}", direction, client_addr, e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}