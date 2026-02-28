//! Protocol configuration types for Songbird communication

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// WebSocket removed — use JSON-RPC 2.0 (biomeOS/songbird)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdProtocol {
    HTTP,
    GRPC,
    MessageQueue,
}

/// Protocol configuration for Songbird communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    pub protocol: SongbirdProtocol,
    pub http: HttpProtocolConfig,
    pub grpc: GrpcProtocolConfig,
    pub message_queue: MessageQueueProtocolConfig,
}

/// HTTP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProtocolConfig {
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub headers: HashMap<String, String>,
}

/// gRPC protocol configuration
///
/// # Deprecation
///
/// gRPC is deprecated per wateringHole UNIVERSAL_IPC_STANDARD_V3.
/// Migrate to JSON-RPC 2.0 over Unix sockets.
#[deprecated(
    since = "0.3.0",
    note = "gRPC deprecated per UNIVERSAL_IPC_STANDARD_V3. Use JSON-RPC over Unix socket."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcProtocolConfig {
    pub timeout_ms: u64,
    pub max_message_size: usize,
    pub compression: bool,
}

/// Message queue protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueueProtocolConfig {
    pub queue_name: String,
    pub exchange: String,
    pub routing_key: String,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_songbird_protocol_variants() {
        let _http = SongbirdProtocol::HTTP;
        let _grpc = SongbirdProtocol::GRPC;
        let _mq = SongbirdProtocol::MessageQueue;
    }

    #[test]
    fn test_songbird_protocol_serialization_roundtrip() {
        for protocol in [SongbirdProtocol::HTTP, SongbirdProtocol::GRPC] {
            let json = serde_json::to_string(&protocol).unwrap();
            let parsed: SongbirdProtocol = serde_json::from_str(&json).unwrap();
            assert!(std::mem::discriminant(&protocol) == std::mem::discriminant(&parsed));
        }
    }

    #[test]
    fn test_http_protocol_config_construction() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());
        let config = HttpProtocolConfig {
            timeout_ms: 5000,
            max_retries: 3,
            headers,
        };
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_grpc_protocol_config_construction() {
        let config = GrpcProtocolConfig {
            timeout_ms: 10_000,
            max_message_size: 4 * 1024 * 1024,
            compression: true,
        };
        assert_eq!(config.max_message_size, 4 * 1024 * 1024);
        assert!(config.compression);
    }

    #[test]
    fn test_message_queue_protocol_config_construction() {
        let config = MessageQueueProtocolConfig {
            queue_name: "jobs".to_string(),
            exchange: "toadstool".to_string(),
            routing_key: "compute".to_string(),
        };
        assert_eq!(config.queue_name, "jobs");
        assert_eq!(config.routing_key, "compute");
    }

    #[test]
    fn test_protocol_config_serialization_roundtrip() {
        let config = ProtocolConfig {
            protocol: SongbirdProtocol::HTTP,
            http: HttpProtocolConfig {
                timeout_ms: 3000,
                max_retries: 5,
                headers: HashMap::new(),
            },
            grpc: GrpcProtocolConfig {
                timeout_ms: 5000,
                max_message_size: 1024 * 1024,
                compression: false,
            },
            message_queue: MessageQueueProtocolConfig {
                queue_name: "test".to_string(),
                exchange: "ex".to_string(),
                routing_key: "key".to_string(),
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ProtocolConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.http.timeout_ms, config.http.timeout_ms);
    }
}
