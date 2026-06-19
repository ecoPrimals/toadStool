// SPDX-License-Identifier: AGPL-3.0-or-later
//! Protocol configuration types for Coordination communication

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// WebSocket removed — use JSON-RPC 2.0 (biomeOS/coordination)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationTransport {
    /// JSON-RPC or REST-style HTTP transport.
    HTTP,
    /// Brokered message-queue transport.
    MessageQueue,
}

/// Protocol configuration for Coordination communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    /// Active transport for this connection.
    pub protocol: CoordinationTransport,
    /// Settings when using HTTP.
    pub http: HttpProtocolConfig,
    /// Settings when using a message queue.
    pub message_queue: MessageQueueProtocolConfig,
}

/// HTTP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProtocolConfig {
    /// Per-request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum retry attempts for transient failures.
    pub max_retries: u32,
    /// Extra HTTP headers to send on each request.
    pub headers: HashMap<String, String>,
}

/// Message queue protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueueProtocolConfig {
    /// Primary queue name for consumer bindings.
    pub queue_name: String,
    /// AMQP-style exchange name (or broker-specific equivalent).
    pub exchange: String,
    /// Routing key for directed delivery.
    pub routing_key: String,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_coordination_protocol_variants() {
        let _http = CoordinationTransport::HTTP;
        let _mq = CoordinationTransport::MessageQueue;
    }

    #[test]
    fn test_coordination_protocol_serialization_roundtrip() {
        for protocol in [
            CoordinationTransport::HTTP,
            CoordinationTransport::MessageQueue,
        ] {
            let json = serde_json::to_string(&protocol).unwrap();
            let parsed: CoordinationTransport = serde_json::from_str(&json).unwrap();
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
            protocol: CoordinationTransport::HTTP,
            http: HttpProtocolConfig {
                timeout_ms: 3000,
                max_retries: 5,
                headers: HashMap::new(),
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
