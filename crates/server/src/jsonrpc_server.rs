//! # ToadStool JSON-RPC 2.0 Server
//!
//! Universal, language-agnostic RPC access to ToadStool compute capabilities.
//! Following Songbird's proven jsonrpsee pattern.
//!
//! ## Design Principles (from Songbird)
//!
//! - **Universal Protocol**: Works with any language (Python, JS, Go, etc.)
//! - **Standard Compliant**: JSON-RPC 2.0 specification
//! - **Type-Safe**: Rust types with serde serialization
//! - **Async Native**: Built on tokio
//! - **Self-Describing**: Capabilities query for runtime discovery
//!
//! ## Usage
//!
//! ```json
//! // Request
//! {
//!   "jsonrpc": "2.0",
//!   "method": "toadstool.submit_workload",
//!   "params": {
//!     "workload_id": "work-123",
//!     "workload_type": "gpu_compute",
//!     "data": "base64_encoded_data",
//!     "priority": "Normal"
//!   },
//!   "id": 1
//! }
//!
//! // Response
//! {
//!   "jsonrpc": "2.0",
//!   "result": {
//!     "workload_id": "work-123",
//!     "status": "Pending",
//!     "metrics": { ... }
//!   },
//!   "id": 1
//! }
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use jsonrpsee::{
    core::async_trait,
    proc_macros::rpc,
    server::{Server, ServerHandle},
    types::{ErrorObjectOwned, error::ErrorCode},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use toadstool_integration_protocols::tarpc_service::{
    ComputeCapabilities, HealthStatus, WorkloadResult,
    WorkloadSubmission as TarpcWorkloadSubmission,
    WorkloadPriority, ResourceRequirements,
};

/// JSON-RPC server configuration
#[derive(Debug, Clone)]
pub struct JsonRpcConfig {
    /// Bind address
    pub addr: SocketAddr,
    /// Enable request logging
    pub log_requests: bool,
    /// Maximum request size (bytes)
    pub max_request_size: u32,
    /// Maximum response size (bytes)
    pub max_response_size: u32,
}

impl Default for JsonRpcConfig {
    fn default() -> Self {
        use std::net::{IpAddr, Ipv6Addr};
        Self {
            addr: SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8080),
            log_requests: true,
            max_request_size: 10 * 1024 * 1024,  // 10 MB
            max_response_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// JSON-friendly workload submission (base64 encoding for binary data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWorkloadSubmission {
    pub workload_id: String,
    pub workload_type: String,
    /// Base64-encoded binary data
    pub data: String,
    pub metadata: HashMap<String, String>,
    pub priority: WorkloadPriority,
    pub requirements: ResourceRequirements,
}

impl JsonWorkloadSubmission {
    /// Convert to tarpc submission (decode base64)
    fn to_tarpc(&self) -> Result<TarpcWorkloadSubmission, String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        let data = STANDARD.decode(&self.data)
            .map_err(|e| format!("Invalid base64 data: {}", e))?;

        Ok(TarpcWorkloadSubmission {
            workload_id: self.workload_id.clone(),
            workload_type: self.workload_type.clone(),
            data,
            metadata: self.metadata.clone(),
            priority: self.priority,
            requirements: self.requirements.clone(),
        })
    }
}

/// JSON-RPC 2.0 method definitions
/// 
/// Following Songbird's pattern:
/// - Namespace: "toadstool.*"
/// - Self-describing via capabilities
/// - Standard error codes
#[rpc(server)]
pub trait ToadStoolJsonRpc {
    /// Submit workload for execution
    #[method(name = "toadstool.submit_workload")]
    async fn submit_workload(
        &self,
        submission: JsonWorkloadSubmission,
    ) -> Result<WorkloadResult, ErrorObjectOwned>;

    /// Query workload status
    #[method(name = "toadstool.query_status")]
    async fn query_status(
        &self,
        workload_id: String,
    ) -> Result<WorkloadResult, ErrorObjectOwned>;

    /// Cancel workload
    #[method(name = "toadstool.cancel_workload")]
    async fn cancel_workload(
        &self,
        workload_id: String,
    ) -> Result<(), ErrorObjectOwned>;

    /// List workloads
    #[method(name = "toadstool.list_workloads")]
    async fn list_workloads(
        &self,
        filter: Option<HashMap<String, String>>,
    ) -> Result<Vec<WorkloadResult>, ErrorObjectOwned>;

    /// Query capabilities (SELF-KNOWLEDGE)
    /// 
    /// This is how external clients discover what ToadStool can do.
    /// No hardcoded knowledge - runtime discovery!
    #[method(name = "toadstool.query_capabilities")]
    async fn query_capabilities(&self) -> Result<ComputeCapabilities, ErrorObjectOwned>;

    /// Health check
    #[method(name = "toadstool.health")]
    async fn health(&self) -> Result<HealthStatus, ErrorObjectOwned>;

    /// Get server version and info
    #[method(name = "toadstool.version")]
    async fn version(&self) -> Result<HashMap<String, String>, ErrorObjectOwned>;
}

/// JSON-RPC server state
pub struct JsonRpcServerImpl {
    /// Workload executor (real implementation, not mock)
    executor: Arc<dyn super::tarpc_server::WorkloadExecutor + Send + Sync>,
    /// Server version
    version: String,
    /// Service start time
    start_time: std::time::Instant,
}

impl JsonRpcServerImpl {
    /// Create new JSON-RPC server with real executor
    pub fn new(
        executor: Arc<dyn super::tarpc_server::WorkloadExecutor + Send + Sync>,
        version: String,
    ) -> Self {
        Self {
            executor,
            version,
            start_time: std::time::Instant::now(),
        }
    }
}

#[async_trait]
impl ToadStoolJsonRpcServer for JsonRpcServerImpl {
    async fn submit_workload(
        &self,
        submission: JsonWorkloadSubmission,
    ) -> Result<WorkloadResult, ErrorObjectOwned> {
        info!("JSON-RPC: submit_workload {}", submission.workload_id);

        // Convert and execute
        let tarpc_submission = submission.to_tarpc()
            .map_err(|e| ErrorObjectOwned::owned(
                ErrorCode::InvalidParams.code(),
                e,
                None::<()>,
            ))?;

        self.executor.execute(tarpc_submission).await
            .map_err(|e| ErrorObjectOwned::owned(
                ErrorCode::InternalError.code(),
                e,
                None::<()>,
            ))
    }

    async fn query_status(
        &self,
        workload_id: String,
    ) -> Result<WorkloadResult, ErrorObjectOwned> {
        info!("JSON-RPC: query_status {}", workload_id);
        
        // Implementation would query actual workload status
        Err(ErrorObjectOwned::owned(
            ErrorCode::MethodNotFound.code(),
            "Not yet implemented".to_string(),
            None::<()>,
        ))
    }

    async fn cancel_workload(
        &self,
        workload_id: String,
    ) -> Result<(), ErrorObjectOwned> {
        info!("JSON-RPC: cancel_workload {}", workload_id);

        self.executor.cancel(&workload_id).await
            .map_err(|e| ErrorObjectOwned::owned(
                ErrorCode::InternalError.code(),
                e,
                None::<()>,
            ))
    }

    async fn list_workloads(
        &self,
        _filter: Option<HashMap<String, String>>,
    ) -> Result<Vec<WorkloadResult>, ErrorObjectOwned> {
        info!("JSON-RPC: list_workloads");
        
        // Implementation would list actual workloads
        Ok(vec![])
    }

    async fn query_capabilities(&self) -> Result<ComputeCapabilities, ErrorObjectOwned> {
        info!("JSON-RPC: query_capabilities (self-knowledge)");

        self.executor.query_capabilities().await
            .map_err(|e| ErrorObjectOwned::owned(
                ErrorCode::InternalError.code(),
                e,
                None::<()>,
            ))
    }

    async fn health(&self) -> Result<HealthStatus, ErrorObjectOwned> {
        let uptime = self.start_time.elapsed();
        
        Ok(HealthStatus {
            healthy: true,
            version: self.version.clone(),
            uptime_secs: uptime.as_secs(),
            active_workloads: 0, // TODO: Track actual workloads
            resource_utilization: 0.0,
        })
    }

    async fn version(&self) -> Result<HashMap<String, String>, ErrorObjectOwned> {
        let mut info = HashMap::new();
        info.insert("version".to_string(), self.version.clone());
        info.insert("protocol".to_string(), "JSON-RPC 2.0".to_string());
        info.insert("service".to_string(), "ToadStool Compute".to_string());
        Ok(info)
    }
}

/// Start JSON-RPC 2.0 server
pub async fn start_jsonrpc_server(
    config: JsonRpcConfig,
    executor: Arc<dyn super::tarpc_server::WorkloadExecutor + Send + Sync>,
    version: String,
) -> Result<ServerHandle, Box<dyn std::error::Error>> {
    info!("Starting JSON-RPC server on: {}", config.addr);

    let server = Server::builder()
        .max_request_body_size(config.max_request_size)
        .max_response_body_size(config.max_response_size)
        .build(config.addr)
        .await?;

    let impl_server = JsonRpcServerImpl::new(executor, version);
    let module = impl_server.into_rpc();

    let handle = server.start(module);
    
    info!("JSON-RPC server started successfully");
    Ok(handle)
}

/// Start JSON-RPC server on Unix socket (fallback - deprecated for production)
///
/// # ⚠️ DEPRECATED - Use ManualJsonRpcServer instead
///
/// This function starts a JSON-RPC server but uses TCP with hardcoded port (127.0.0.1:9944)
/// due to jsonrpsee library limitations. This violates deep debt principles.
///
/// **Modern Alternative**: Use `ManualJsonRpcServer::serve()` which supports Unix sockets.
///
/// ## Why Deprecated?
///
/// - Hardcoded TCP port `127.0.0.1:9944` (deep debt violation)
/// - No multi-instance support (port conflicts)
/// - jsonrpsee library limitation (doesn't support Unix sockets)
///
/// ## Migration Path
///
/// ```rust,ignore
/// // Old (deprecated):
/// start_jsonrpc_unix_server(socket_path, executor, version, max_req, max_resp).await?;
///
/// // New (recommended):
/// let server = ManualJsonRpcServer::new(executor, version);
/// server.serve(socket_path).await?;
/// ```
#[deprecated(
    since = "2.2.0",
    note = "Use ManualJsonRpcServer::serve() instead - supports Unix sockets, no TCP hardcoding"
)]
pub async fn start_jsonrpc_unix_server(
    socket_path: PathBuf,
    executor: Arc<dyn super::tarpc_server::WorkloadExecutor + Send + Sync>,
    version: String,
    max_request_size: u32,
    max_response_size: u32,
) -> Result<ServerHandle, Box<dyn std::error::Error>> {
    info!("Starting JSON-RPC server (TCP fallback for socket: {:?})", socket_path);
    
    // ✅ RESOLVED: jsonrpsee Unix socket limitation addressed
    // Solution: ManualJsonRpcServer provides pure Rust HTTP/1.1 + JSON-RPC over Unix sockets
    // See: crates/server/src/manual_jsonrpc.rs
    // This deprecated function remains for backward compatibility only
    let addr = "127.0.0.1:9944".parse::<SocketAddr>()?;
    
    info!("JSON-RPC server listening on: {} (socket path logged for reference: {:?})", addr, socket_path);
    
    // Build JSON-RPC module
    let impl_server = JsonRpcServerImpl::new(executor, version);
    let module = impl_server.into_rpc();
    
    // Create server with configuration
    let server = Server::builder()
        .max_request_body_size(max_request_size)
        .max_response_body_size(max_response_size)
        .build(addr)
        .await?;
    
    let handle = server.start(module);
    
    info!("JSON-RPC server ready at: {}", addr);
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_workload_submission() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        let data = vec![1, 2, 3, 4];
        let encoded = STANDARD.encode(&data);
        
        let submission = JsonWorkloadSubmission {
            workload_id: "work-123".to_string(),
            workload_type: "gpu_compute".to_string(),
            data: encoded,
            metadata: HashMap::new(),
            priority: WorkloadPriority::Normal,
            requirements: ResourceRequirements {
                cpu_cores: Some(4),
                memory_bytes: Some(1024 * 1024 * 1024),
                gpu_memory_bytes: None,
                timeout_secs: Some(300),
            },
        };

        let tarpc = submission.to_tarpc().expect("Conversion failed");
        assert_eq!(tarpc.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_config_default() {
        let config = JsonRpcConfig::default();
        assert_eq!(config.max_request_size, 10 * 1024 * 1024);
        assert!(config.log_requests);
    }
}

