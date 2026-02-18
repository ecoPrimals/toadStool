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
