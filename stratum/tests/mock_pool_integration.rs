#[cfg(feature = "mock-pool")]
mod mock_tests {
    use anyhow::Result;
    use loka_stratum::mock::{MockConfig, MockPool};
    use serde_json::json;
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;
    use tokio::time::sleep;
    

    async fn send_and_receive(
        stream: &mut TcpStream,
        msg: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Send message
        let mut msg_str = msg.to_string();
        msg_str.push('\n');
        stream.write_all(msg_str.as_bytes()).await?;
        stream.flush().await?;
        
        // Read response
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        Ok(serde_json::from_str(&line)?)
    }

    #[tokio::test]
    async fn test_mock_pool_lifecycle() -> Result<()> {
        let config = MockConfig::default();
        let pool = MockPool::new(config);
        let handle = pool.start("127.0.0.1:0").await?;
        
        sleep(Duration::from_millis(100)).await;
        
        handle.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_mining_subscribe() -> Result<()> {
        let config = MockConfig {
            extranonce_size: 4,
            ..Default::default()
        };
        
        let pool = MockPool::new(config);
        let handle = pool.start("127.0.0.1:0").await?;
        let addr = handle.local_addr;
        
        sleep(Duration::from_millis(100)).await;
        
        let mut stream = TcpStream::connect(addr).await?;
        let subscribe_msg = json!({
            "id": 1,
            "method": "mining.subscribe",
            "params": []
        });
        
        let response = send_and_receive(&mut stream, &subscribe_msg).await?;
        
        assert_eq!(response["id"], 1);
        assert!(response["result"].is_array());
        assert!(response["error"].is_null());
        
        let result = &response["result"];
        assert!(result[0].is_array());
        assert!(result[1].is_string());
        assert_eq!(result[2], 4);
        
        handle.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_mining_authorize() -> Result<()> {
        let config = MockConfig::default();
        let pool = MockPool::new(config);
        let handle = pool.start("127.0.0.1:0").await?;
        let addr = handle.local_addr;
        
        sleep(Duration::from_millis(100)).await;
        
        let mut stream = TcpStream::connect(addr).await?;
        
        let subscribe_msg = json!({
            "id": 1,
            "method": "mining.subscribe",
            "params": []
        });
        let _subscribe_response = send_and_receive(&mut stream, &subscribe_msg).await?;
        
        let authorize_msg = json!({
            "id": 2,
            "method": "mining.authorize",
            "params": ["worker1", "password"]
        });
        let auth_response = send_and_receive(&mut stream, &authorize_msg).await?;
        assert_eq!(auth_response["id"], 2);
        assert_eq!(auth_response["result"], true);
        assert!(auth_response["error"].is_null());
        
        handle.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_share_submission() -> Result<()> {
        let config = MockConfig {
            accept_rate: 1.0,
            ..Default::default()
        };
        
        let pool = MockPool::new(config);
        let handle = pool.start("127.0.0.1:0").await?;
        let addr = handle.local_addr;
        
        sleep(Duration::from_millis(100)).await;
        
        let mut stream = TcpStream::connect(addr).await?;
        
        let subscribe_msg = json!({
            "id": 1,
            "method": "mining.subscribe",
            "params": []
        });
        let _subscribe_response = send_and_receive(&mut stream, &subscribe_msg).await?;
        
        let authorize_msg = json!({
            "id": 2,
            "method": "mining.authorize",
            "params": ["worker1", "password"]
        });
        let _auth_response = send_and_receive(&mut stream, &authorize_msg).await?;
        
        let submit_msg = json!({
            "id": 3,
            "method": "mining.submit",
            "params": ["worker1", "job123", "00000000", "5f000000", "12345678"]
        });
        let submit_response = send_and_receive(&mut stream, &submit_msg).await?;
        assert_eq!(submit_response["id"], 3);
        
        handle.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_difficulty_adjustment() -> Result<()> {
        let config = MockConfig {
            vardiff_enabled: true,
            vardiff_target_time_secs: 1,
            initial_difficulty: 128,
            ..Default::default()
        };
        
        let pool = MockPool::new(config);
        let handle = pool.start("127.0.0.1:0").await?;
        let addr = handle.local_addr;
        
        sleep(Duration::from_millis(100)).await;
        
        let mut stream = TcpStream::connect(addr).await?;
        
        let subscribe_msg = json!({
            "id": 1,
            "method": "mining.subscribe",
            "params": []
        });
        let _subscribe_response = send_and_receive(&mut stream, &subscribe_msg).await?;
        
        let authorize_msg = json!({
            "id": 2,
            "method": "mining.authorize",
            "params": ["worker1", "password"]
        });
        let _auth_response = send_and_receive(&mut stream, &authorize_msg).await?;
        
        for i in 0..10 {
            let submit_msg = json!({
                "id": 10 + i,
                "method": "mining.submit",
                "params": [
                    "worker1",
                    format!("job{}", i),
                    format!("{:08x}", i),
                    "5f000000",
                    format!("{:08x}", i * 1000)
                ]
            });
            let _response = send_and_receive(&mut stream, &submit_msg).await?;
            sleep(Duration::from_millis(200)).await;
        }
        
        handle.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_error_injection() -> Result<()> {
        let config = MockConfig {
            error_rate: 0.5,
            ..Default::default()
        };
        
        let pool = MockPool::new(config);
        let handle = pool.start("127.0.0.1:0").await?;
        let addr = handle.local_addr;
        
        sleep(Duration::from_millis(100)).await;
        
        let mut stream = TcpStream::connect(addr).await?;
        
        let mut error_count = 0;
        for i in 0..10 {
            let msg = json!({
                "id": i,
                "method": "mining.ping",
                "params": []
            });
            
            let response = send_and_receive(&mut stream, &msg).await?;
            if !response["error"].is_null() {
                error_count += 1;
            }
        }
        
        assert!(error_count >= 2 && error_count <= 8, "Expected ~50% error rate, got {}/10", error_count);
        
        handle.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_latency_simulation() -> Result<()> {
        let config = MockConfig {
            latency_ms: 100,
            ..Default::default()
        };
        
        let pool = MockPool::new(config);
        let handle = pool.start("127.0.0.1:0").await?;
        let addr = handle.local_addr;
        
        sleep(Duration::from_millis(100)).await;
        
        let mut stream = TcpStream::connect(addr).await?;
        
        let start = tokio::time::Instant::now();
        
        let ping_msg = json!({
            "id": 1,
            "method": "mining.ping",
            "params": []
        });
        
        let _response = send_and_receive(&mut stream, &ping_msg).await?;
        let elapsed = start.elapsed();
        
        assert!(elapsed >= Duration::from_millis(100), "Latency simulation failed");
        
        handle.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_connections() -> Result<()> {
        let config = MockConfig {
            max_connections: 10,
            ..Default::default()
        };
        
        let pool = MockPool::new(config);
        let handle = pool.start("127.0.0.1:0").await?;
        let addr = handle.local_addr;
        
        sleep(Duration::from_millis(100)).await;
        
        let mut handles = Vec::new();
        
        for i in 0..5 {
            let handle = tokio::spawn(async move {
                let mut stream = TcpStream::connect(addr).await?;
                
                let subscribe_msg = json!({
                    "id": 1,
                    "method": "mining.subscribe",
                    "params": []
                });
                let response = send_and_receive(&mut stream, &subscribe_msg).await?;
                
                assert!(response["error"].is_null());
                
                let authorize_msg = json!({
                    "id": 2,
                    "method": "mining.authorize",
                    "params": [format!("worker{}", i), "password"]
                });
                let auth_response = send_and_receive(&mut stream, &authorize_msg).await?;
                assert_eq!(auth_response["result"], true);
                
                Ok::<(), anyhow::Error>(())
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await??;
        }
        
        handle.shutdown().await?;
        Ok(())
    }
}