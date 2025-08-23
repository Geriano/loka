use loka_stratum::protocol::handler::ProtocolHandler;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_request_path() {
        // Test POST request parsing
        let post_request = "POST /order-1 HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(post_request);
        assert_eq!(path, Some("order-1".to_string()));

        // Test GET request parsing
        let get_request = "GET /status HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(get_request);
        assert_eq!(path, Some("status".to_string()));

        // Test PUT request parsing
        let put_request = "PUT /api/users/123 HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(put_request);
        assert_eq!(path, Some("api/users/123".to_string()));

        // Test DELETE request parsing
        let delete_request = "DELETE /api/sessions HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(delete_request);
        assert_eq!(path, Some("api/sessions".to_string()));

        // Test HTTP/2 protocol support
        let http2_request = "POST /order-2 HTTP/2.0";
        let path = ProtocolHandler::parse_http_request_path(http2_request);
        assert_eq!(path, Some("order-2".to_string()));

        // Test with query parameters (should be stripped)
        let query_request = "GET /status?health=true HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(query_request);
        assert_eq!(path, Some("status".to_string()));

        // Test with fragment (should be stripped)
        let fragment_request = "GET /page#section1 HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(fragment_request);
        assert_eq!(path, Some("page".to_string()));

        // Test root path (should return None as it's not meaningful)
        let root_request = "GET / HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(root_request);
        assert_eq!(path, None);

        // Test invalid requests (should return None)
        let invalid_method = "INVALID /path HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(invalid_method);
        assert_eq!(path, None);

        let invalid_protocol = "GET /path INVALID/1.1";
        let path = ProtocolHandler::parse_http_request_path(invalid_protocol);
        assert_eq!(path, None);

        let malformed_request = "GET";
        let path = ProtocolHandler::parse_http_request_path(malformed_request);
        assert_eq!(path, None);

        // Test CONNECT requests (should return None as they're handled separately)
        let connect_request = "CONNECT example.com:443 HTTP/1.1";
        let path = ProtocolHandler::parse_http_request_path(connect_request);
        assert_eq!(path, None);
    }

    #[test]
    fn test_is_direct_http_request() {
        // Test valid HTTP requests
        assert!(ProtocolHandler::is_direct_http_request("POST /order-1 HTTP/1.1"));
        assert!(ProtocolHandler::is_direct_http_request("GET /status HTTP/1.1"));
        assert!(ProtocolHandler::is_direct_http_request("PUT /api/data HTTP/1.1"));
        assert!(ProtocolHandler::is_direct_http_request("DELETE /resource HTTP/1.1"));
        assert!(ProtocolHandler::is_direct_http_request("POST /test HTTP/2.0"));

        // Test CONNECT requests (should return false as they're handled separately)
        assert!(!ProtocolHandler::is_direct_http_request("CONNECT example.com:443 HTTP/1.1"));

        // Test invalid requests
        assert!(!ProtocolHandler::is_direct_http_request("INVALID /path HTTP/1.1"));
        assert!(!ProtocolHandler::is_direct_http_request("GET /path INVALID/1.1"));
        assert!(!ProtocolHandler::is_direct_http_request("GET"));
        assert!(!ProtocolHandler::is_direct_http_request(""));

        // Test JSON Stratum messages (should return false)
        assert!(!ProtocolHandler::is_direct_http_request(r#"{"method":"mining.subscribe","params":[],"id":1}"#));
    }

    #[test]
    fn test_extract_path_from_message_http_priority() {
        // Test that HTTP request parsing takes priority over JSON parsing
        let http_request = "POST /order-1 HTTP/1.1";
        let path = ProtocolHandler::extract_path_from_message(http_request);
        assert_eq!(path, Some("order-1".to_string()));

        // Test JSON message fallback
        let json_message = r#"{"method":"mining.subscribe","params":["user_agent_with_url/path123"],"id":1}"#;
        let path = ProtocolHandler::extract_path_from_message(json_message);
        assert_eq!(path, Some("path123".to_string()));

        // Test invalid message
        let invalid_message = "invalid message format";
        let path = ProtocolHandler::extract_path_from_message(invalid_message);
        assert_eq!(path, None);
    }

    #[test]
    fn test_path_validation() {
        // Test valid paths
        let valid_paths = vec![
            "POST /order-1 HTTP/1.1",
            "GET /api/status HTTP/1.1", 
            "POST /user_profile HTTP/1.1",
            "DELETE /api/sessions/abc123 HTTP/1.1",
        ];

        for request in valid_paths {
            let path = ProtocolHandler::parse_http_request_path(request);
            assert!(path.is_some(), "Failed to parse valid request: {}", request);
            let extracted = path.unwrap();
            assert!(extracted.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/'));
        }

        // Test invalid character paths
        let invalid_paths = vec![
            "POST /path with spaces HTTP/1.1",
            "GET /path@invalid HTTP/1.1",
            "POST /path!invalid HTTP/1.1",
        ];

        for request in invalid_paths {
            let path = ProtocolHandler::parse_http_request_path(request);
            assert!(path.is_none(), "Should not parse invalid request: {}", request);
        }
    }
}