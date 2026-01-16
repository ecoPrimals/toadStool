use std::collections::HashMap;
use std::sync::Arc;

// PURE RUST: reqwest removed - use Unix sockets via Songbird!
use toadstool::ToadStoolResult;
use tokio::sync::RwLock;

use super::auth::AuthenticationManager;
use super::registry::ServiceRegistry;
use crate::types::UniversalJob;

/// Ecosystem caller for invoking external services
/// 
/// **DEPRECATED**: HTTP-based external calls removed
/// **Use**: Unix socket connections via primal integrations instead
pub struct EcosystemCaller {
    /// HTTP client for REST APIs - REMOVED (use Unix sockets!)
    _stub_marker: String,
    /// gRPC client configurations
    _grpc_clients: HashMap<String, GrpcClientConfig>,
    /// WebSocket connections
    _websocket_connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    /// Message queue connections
    _message_queues: Arc<RwLock<HashMap<String, MessageQueueConnection>>>,
    /// Authentication manager
    _auth_manager: Arc<AuthenticationManager>,
    /// Service registry
    _service_registry: Arc<ServiceRegistry>,
}

/// gRPC client configuration
#[derive(Debug, Clone)]
pub struct GrpcClientConfig {
    pub endpoint: String,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

/// WebSocket connection
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub endpoint: String,
    pub connected: bool,
    pub last_ping: std::time::Instant,
}

/// Message queue connection
#[derive(Debug, Clone)]
pub struct MessageQueueConnection {
    pub endpoint: String,
    pub queue_name: String,
    pub connected: bool,
}

impl EcosystemCaller {
    #[must_use]
    pub fn new() -> Self {
        tracing::warn!("EcosystemCaller: HTTP removed - use Unix socket integrations");
        Self {
            _stub_marker: "HTTP_REMOVED_USE_UNIX_SOCKETS".to_string(),
            _grpc_clients: HashMap::new(),
            _websocket_connections: Arc::new(RwLock::new(HashMap::new())),
            _message_queues: Arc::new(RwLock::new(HashMap::new())),
            _auth_manager: Arc::new(AuthenticationManager::new()),
            _service_registry: Arc::new(ServiceRegistry::new()),
        }
    }

    pub async fn call_service(&self, job: &UniversalJob) -> ToadStoolResult<()> {
        // Simplified service calling
        tracing::info!("Calling ecosystem service for job: {:?}", job.job_id);
        Ok(())
    }
}

impl Default for EcosystemCaller {
    fn default() -> Self {
        Self::new()
    }
}
