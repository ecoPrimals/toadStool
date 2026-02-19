//! # Pure Rust JSON-RPC 2.0 Implementation
//!
//! **Pattern**: BearDog's proven ~150 line manual implementation  
//! **Dependencies**: ZERO! Only `serde_json` (already in workspace)  
//! **Status**: 100% Pure Rust - NO `jsonrpsee`, NO `ring`!  
//!
//! ## Why Manual Implementation?
//!
//! - ✅ **Pure Rust**: Zero C dependencies (no `ring` via `rustls`)
//! - ✅ **Simple**: JSON-RPC 2.0 spec is ~3 structs (~150 lines total)
//! - ✅ **Fast**: No heavy dependencies, faster compile
//! - ✅ **Full Control**: Custom routing, error handling
//! - ✅ **Proven**: BearDog uses this in production
//!
//! ## Architecture
//!
//! ```text
//! Request → Parse JSON → Route Method → Execute → Response JSON
//! ```
//!
//! That's it! No complicated middleware, no heavy abstractions.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{error, info};

// Re-use existing types
use crate::rpc_types::{
    HealthStatus, ResourceRequirements, WorkloadPriority,
    WorkloadSubmission as TarpcWorkloadSubmission,
};

/// JSON-RPC 2.0 Request
///
/// Standard compliant: <https://www.jsonrpc.org/specification>
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (must be "2.0")
    pub jsonrpc: String,

    /// Method name (e.g., "toadstool.submit_workload")
    pub method: String,

    /// Optional parameters (can be object or array)
    #[serde(default)]
    pub params: Option<serde_json::Value>,

    /// Request ID (for matching request/response)
    pub id: Option<serde_json::Value>,
}

/// Zero-copy JSON-RPC version for responses
const JSONRPC_VERSION: &str = toadstool_common::constants::jsonrpc::VERSION;

/// JSON-RPC 2.0 Response
///
/// Either contains `result` (success) or `error` (failure), never both.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    /// Protocol version ("2.0")
    pub jsonrpc: Cow<'static, str>,

    /// Success result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error object (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    /// Request ID (from original request)
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 Error Object
///
/// Standard error codes: <https://www.jsonrpc.org/specification#error_object>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code (standard or application-defined)
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    // Standard JSON-RPC 2.0 error codes -- from shared ecosystem constants
    pub const PARSE_ERROR: i32 = toadstool_common::constants::jsonrpc::error_codes::PARSE_ERROR;
    pub const INVALID_REQUEST: i32 =
        toadstool_common::constants::jsonrpc::error_codes::INVALID_REQUEST;
    pub const METHOD_NOT_FOUND: i32 =
        toadstool_common::constants::jsonrpc::error_codes::METHOD_NOT_FOUND;
    pub const INVALID_PARAMS: i32 =
        toadstool_common::constants::jsonrpc::error_codes::INVALID_PARAMS;
    pub const INTERNAL_ERROR: i32 =
        toadstool_common::constants::jsonrpc::error_codes::INTERNAL_ERROR;

    /// Create parse error
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self {
            code: Self::PARSE_ERROR,
            message: msg.into(),
            data: None,
        }
    }

    /// Create invalid request error
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: msg.into(),
            data: None,
        }
    }

    /// Create method not found error
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    /// Create invalid params error
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: msg.into(),
            data: None,
        }
    }

    /// Create internal error
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INTERNAL_ERROR,
            message: msg.into(),
            data: None,
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
    /// Convert into tarpc submission (decode base64, consumes self)
    fn into_tarpc(self) -> Result<TarpcWorkloadSubmission, String> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        let data = STANDARD
            .decode(&self.data)
            .map_err(|e| format!("Invalid base64 data: {}", e))?;

        Ok(TarpcWorkloadSubmission {
            workload_id: self.workload_id,
            workload_type: self.workload_type,
            data,
            metadata: self.metadata,
            priority: self.priority,
            requirements: self.requirements,
        })
    }
}

/// Pure Rust JSON-RPC Handler
///
/// This is the core - routes requests to appropriate methods.
pub struct JsonRpcHandler {
    executor: Arc<dyn super::tarpc_server::WorkloadExecutor + Send + Sync>,
    version: String,
    start_time: std::time::Instant,
    job_queue: crate::gpu_job_queue::GpuJobQueue,
    error_count: Arc<AtomicU64>,
}

impl JsonRpcHandler {
    /// Create new handler with executor
    ///
    /// Pass `error_count` to share the counter with other servers for unified monitoring.
    pub fn new(
        executor: Arc<dyn super::tarpc_server::WorkloadExecutor + Send + Sync>,
        version: String,
        error_count: Option<Arc<AtomicU64>>,
    ) -> Self {
        Self {
            executor,
            version,
            start_time: std::time::Instant::now(),
            job_queue: crate::gpu_job_queue::GpuJobQueue::new(
                crate::gpu_job_queue::JobQueueConfig::default(),
            ),
            error_count: error_count.unwrap_or_else(|| Arc::new(AtomicU64::new(0))),
        }
    }

    /// Handle JSON-RPC request (main entry point)
    ///
    /// This is BearDog's proven pattern: parse → validate → route → execute → respond
    pub async fn handle_request(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        // Validate JSON-RPC version
        if request.jsonrpc != JSONRPC_VERSION {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return JsonRpcResponse {
                jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                result: None,
                error: Some(JsonRpcError::invalid_request(
                    "Invalid JSON-RPC version (must be '2.0')",
                )),
                id: request.id.clone().unwrap_or(serde_json::Value::Null),
            };
        }

        info!("JSON-RPC request: {}", request.method);

        // Route to appropriate method handler
        match self
            .handle_method(&request.method, request.params.as_ref())
            .await
        {
            Ok(result) => JsonRpcResponse {
                jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                result: Some(result),
                error: None,
                id: request.id.clone().unwrap_or(serde_json::Value::Null),
            },
            Err(err) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                error!("JSON-RPC error for {}: {}", request.method, err.message);
                JsonRpcResponse {
                    jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                    result: None,
                    error: Some(err),
                    id: request.id.clone().unwrap_or(serde_json::Value::Null),
                }
            }
        }
    }

    /// Route method to handler
    ///
    /// This is where all ToadStool methods are registered.
    async fn handle_method(
        &self,
        method: &str,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match method {
            // Submit workload
            "toadstool.submit_workload" => self.submit_workload(params).await,

            // Query status
            "toadstool.query_status" => self.query_status(params).await,

            // Cancel workload
            "toadstool.cancel_workload" => self.cancel_workload(params).await,

            // List workloads
            "toadstool.list_workloads" => self.list_workloads(params).await,

            // Query capabilities (SELF-KNOWLEDGE!)
            "toadstool.query_capabilities" => self.query_capabilities().await,

            // Health check
            "toadstool.health" => self.health().await,

            // Version info
            "toadstool.version" => self.version_info().await,

            // GPU job-queue methods (`compute.*` namespace).
            // These address the low-level GPU batch queue (JobType + priority).
            // `toadstool.*` methods address the high-level workload executor
            // (WorkloadSpec). Both namespaces are intentional and serve
            // different clients — they are not aliases of each other.
            "compute.submit" => self.compute_submit(params).await,
            "compute.status" => self.compute_status(params).await,
            "compute.result" => self.compute_result(params).await,
            "compute.cancel" => self.compute_cancel(params).await,
            "compute.list" => self.compute_list(params).await,

            // Unknown method
            _ => Err(JsonRpcError::method_not_found(method)),
        }
    }

    /// Submit workload method
    async fn submit_workload(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let submission: JsonWorkloadSubmission = serde_json::from_value(params.clone())
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {}", e)))?;

        info!("Submitting workload: {}", submission.workload_id);

        let tarpc_submission = submission
            .into_tarpc()
            .map_err(JsonRpcError::invalid_params)?;

        let result = self
            .executor
            .execute(tarpc_submission)
            .await
            .map_err(JsonRpcError::internal_error)?;

        serde_json::to_value(result)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Query workload status
    async fn query_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let workload_id: String = serde_json::from_value(params.clone())
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {}", e)))?;

        info!("Querying status: {}", workload_id);

        // Parse as UUID for job queue lookup
        let job_id = uuid::Uuid::parse_str(&workload_id)
            .map_err(|_| JsonRpcError::invalid_params("Invalid job ID format"))?;

        match self.job_queue.status(job_id).await {
            Ok(job) => serde_json::to_value(job)
                .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}"))),
            Err(e) => Err(JsonRpcError::internal_error(e.to_string())),
        }
    }

    /// Cancel workload
    async fn cancel_workload(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let workload_id: String = serde_json::from_value(params.clone())
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {}", e)))?;

        info!("Canceling workload: {}", workload_id);

        self.executor
            .cancel(&workload_id)
            .await
            .map_err(JsonRpcError::internal_error)?;

        Ok(serde_json::json!({"success": true}))
    }

    /// List workloads
    async fn list_workloads(
        &self,
        _params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        info!("Listing workloads");

        let jobs = self.job_queue.list(None).await;
        let counts = self.job_queue.counts().await;

        Ok(serde_json::json!({
            "jobs": jobs,
            "counts": counts,
        }))
    }

    /// Query capabilities (SELF-KNOWLEDGE!)
    async fn query_capabilities(&self) -> Result<serde_json::Value, JsonRpcError> {
        info!("Querying capabilities (self-knowledge)");

        let caps = self
            .executor
            .query_capabilities()
            .await
            .map_err(JsonRpcError::internal_error)?;

        serde_json::to_value(caps)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Health check
    async fn health(&self) -> Result<serde_json::Value, JsonRpcError> {
        let uptime = self.start_time.elapsed();

        let status = HealthStatus {
            healthy: true,
            version: self.version.clone(),
            uptime_secs: uptime.as_secs(),
            active_workloads: 0,
            queued_workloads: 0,
            error_count: self.error_count.load(Ordering::Relaxed) as usize,
            resource_utilization: 0.0,
        };

        serde_json::to_value(status)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Version info
    async fn version_info(&self) -> Result<serde_json::Value, JsonRpcError> {
        let mut info = HashMap::new();
        info.insert("version".to_string(), self.version.clone());
        info.insert("protocol".to_string(), "JSON-RPC 2.0".to_string());
        info.insert("service".to_string(), "ToadStool Compute".to_string());
        info.insert(
            "implementation".to_string(),
            "Pure Rust (BearDog pattern)".to_string(),
        );

        Ok(serde_json::json!(info))
    }

    // ---- GPU Compute Job Queue (compute.*) ----

    async fn compute_submit(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let job_type: crate::gpu_job_queue::JobType = serde_json::from_value(params.clone())
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid job type: {e}")))?;

        let priority = params.get("priority").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        match self.job_queue.submit(job_type, priority).await {
            Ok(job_id) => Ok(serde_json::json!({"job_id": job_id})),
            Err(e) => Err(JsonRpcError::internal_error(e.to_string())),
        }
    }

    async fn compute_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let job_id = self.extract_job_id(params)?;
        match self.job_queue.status(job_id).await {
            Ok(job) => serde_json::to_value(job)
                .map_err(|e| JsonRpcError::internal_error(format!("Serialization: {e}"))),
            Err(e) => Err(JsonRpcError::internal_error(e.to_string())),
        }
    }

    async fn compute_result(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let job_id = self.extract_job_id(params)?;
        self.job_queue
            .result(job_id)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn compute_cancel(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let job_id = self.extract_job_id(params)?;
        self.job_queue
            .cancel(job_id)
            .await
            .map(|()| serde_json::json!({"cancelled": true}))
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn compute_list(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let state_filter = params
            .and_then(|p| p.get("state"))
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let jobs = self.job_queue.list(state_filter).await;
        let counts = self.job_queue.counts().await;
        Ok(serde_json::json!({"jobs": jobs, "counts": counts}))
    }

    fn extract_job_id(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<uuid::Uuid, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let id_str = params
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'job_id'"))?;
        uuid::Uuid::parse_str(id_str)
            .map_err(|_| JsonRpcError::invalid_params("Invalid job_id UUID"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_handler() -> JsonRpcHandler {
        let executor = Arc::new(crate::tarpc_server::StandaloneExecutor::new());
        JsonRpcHandler::new(executor, "test-1.0.0".to_string(), None)
    }

    fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(serde_json::json!(id)),
        }
    }

    #[test]
    fn test_parse_request() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "toadstool.health",
            "id": 1
        }"#;

        let req: JsonRpcRequest = serde_json::from_str(json).expect("Parse failed");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "toadstool.health");
    }

    #[test]
    fn test_error_response() {
        let err = JsonRpcError::method_not_found("foo.bar");
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("foo.bar"));
    }

    #[test]
    fn test_json_workload_submission() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

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

        let tarpc = submission.into_tarpc().expect("Conversion failed");
        assert_eq!(tarpc.data, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_health_via_handle_request() {
        let handler = test_handler();
        let request = mk_request("toadstool.health", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result["healthy"].as_bool().unwrap());
        assert_eq!(result["version"], "test-1.0.0");
    }

    #[tokio::test]
    async fn test_handle_method_dispatch_version() {
        let handler = test_handler();
        let request = mk_request("toadstool.version", None, 2);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["version"], "test-1.0.0");
        assert_eq!(result["protocol"], "JSON-RPC 2.0");
    }

    #[tokio::test]
    async fn test_handle_method_dispatch_unknown() {
        let handler = test_handler();
        let request = mk_request("unknown.method", None, 99);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_invalid_jsonrpc_version() {
        let handler = test_handler();
        let request = JsonRpcRequest {
            jsonrpc: "3.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_REQUEST);
    }

    #[tokio::test]
    async fn test_compute_submit() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": {
                "model": "tinyllama",
                "prompt": "Hello",
                "params": {}
            }
        });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result["job_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_compute_list() {
        let handler = test_handler();
        let request = mk_request("compute.list", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result["jobs"].is_array());
        assert!(result["counts"].is_object());
    }

    #[tokio::test]
    async fn test_compute_status_missing_job_id() {
        let handler = test_handler();
        let request = mk_request("compute.status", Some(serde_json::json!({})), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_query_capabilities() {
        let handler = test_handler();
        let request = mk_request("toadstool.query_capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result["service_id"].as_str().is_some());
        assert!(result["compute_units"].is_array());
    }

    #[test]
    fn test_jsonrpc_error_constructors() {
        let err = JsonRpcError::parse_error("bad json");
        assert_eq!(err.code, JsonRpcError::PARSE_ERROR);
        assert!(err.message.contains("bad json"));

        let err = JsonRpcError::invalid_request("wrong version");
        assert_eq!(err.code, JsonRpcError::INVALID_REQUEST);

        let err = JsonRpcError::invalid_params("missing field");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);

        let err = JsonRpcError::internal_error("panic");
        assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    }

    #[test]
    fn test_json_workload_submission_invalid_base64() {
        let submission = JsonWorkloadSubmission {
            workload_id: "work-1".to_string(),
            workload_type: "gpu_compute".to_string(),
            data: "!!!not-valid-base64!!!".to_string(),
            metadata: HashMap::new(),
            priority: WorkloadPriority::Normal,
            requirements: ResourceRequirements {
                cpu_cores: Some(4),
                memory_bytes: Some(1024 * 1024 * 1024),
                gpu_memory_bytes: None,
                timeout_secs: Some(300),
            },
        };
        let result = submission.into_tarpc();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid base64"));
    }

    #[tokio::test]
    async fn test_submit_workload_missing_params() {
        let handler = test_handler();
        let request = mk_request("toadstool.submit_workload", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_submit_workload_success() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let handler = test_handler();
        let params = serde_json::json!({
            "workload_id": "work-submit-1",
            "workload_type": "cpu_compute",
            "data": STANDARD.encode([1u8, 2, 3, 4]),
            "metadata": {},
            "priority": "Normal",
            "requirements": {
                "cpu_cores": 2,
                "memory_bytes": 1024,
                "timeout_secs": 60
            }
        });
        let request = mk_request("toadstool.submit_workload", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["workload_id"], "work-submit-1");
        assert!(result["status"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_submit_workload_invalid_base64() {
        let handler = test_handler();
        let params = serde_json::json!({
            "workload_id": "work-1",
            "workload_type": "cpu_compute",
            "data": "!!!invalid!!!",
            "metadata": {},
            "priority": "Normal",
            "requirements": {}
        });
        let request = mk_request("toadstool.submit_workload", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_query_status_invalid_uuid() {
        let handler = test_handler();
        let params = serde_json::json!("not-a-uuid");
        let request = mk_request("toadstool.query_status", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_query_status_job_not_found() {
        let handler = test_handler();
        let job_id = uuid::Uuid::new_v4();
        let params = serde_json::json!(job_id.to_string());
        let request = mk_request("toadstool.query_status", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    }

    #[tokio::test]
    async fn test_cancel_workload_missing_params() {
        let handler = test_handler();
        let request = mk_request("toadstool.cancel_workload", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_cancel_workload_success() {
        let handler = test_handler();
        let params = serde_json::json!("some-workload-id");
        let request = mk_request("toadstool.cancel_workload", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["success"], true);
    }

    #[tokio::test]
    async fn test_compute_result_missing_job_id() {
        let handler = test_handler();
        let request = mk_request("compute.result", Some(serde_json::json!({})), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_compute_result_job_not_found() {
        let handler = test_handler();
        let job_id = uuid::Uuid::new_v4();
        let params = serde_json::json!({ "job_id": job_id.to_string() });
        let request = mk_request("compute.result", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    }

    #[tokio::test]
    async fn test_compute_cancel_missing_job_id() {
        let handler = test_handler();
        let request = mk_request("compute.cancel", Some(serde_json::json!({})), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_compute_list_with_state_filter() {
        let handler = test_handler();
        let params = serde_json::json!({ "state": "pending" });
        let request = mk_request("compute.list", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result["jobs"].is_array());
        assert!(result["counts"].is_object());
    }

    #[tokio::test]
    async fn test_health_error_count_incremented() {
        let error_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let executor = Arc::new(crate::tarpc_server::StandaloneExecutor::new());
        let handler = JsonRpcHandler::new(executor, "1.0".to_string(), Some(error_count));

        let bad_request = mk_request("unknown.method", None, 1);
        let _ = handler.handle_request(&bad_request).await;

        let health_request = mk_request("toadstool.health", None, 2);
        let response = handler.handle_request(&health_request).await;
        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result["error_count"].as_u64().unwrap_or(0) >= 1);
    }

    #[tokio::test]
    async fn test_request_id_null_when_missing() {
        let handler = test_handler();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "toadstool.health".to_string(),
            params: None,
            id: None,
        };
        let response = handler.handle_request(&request).await;
        assert_eq!(response.id, serde_json::Value::Null);
    }

    #[tokio::test]
    async fn test_compute_submit_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({ "invalid": "job_type" });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_submit_workload_invalid_params_structure() {
        let handler = test_handler();
        let params = serde_json::json!({
            "workload_type": "cpu_compute",
            "data": ""
        });
        let request = mk_request("toadstool.submit_workload", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_compute_cancel_success() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": { "model": "test", "prompt": "x", "params": {} }
        });
        let submit_req = mk_request("compute.submit", Some(params.clone()), 1);
        let submit_resp = handler.handle_request(&submit_req).await;
        let job_id = submit_resp
            .result
            .as_ref()
            .and_then(|r| r.get("job_id"))
            .and_then(|v| v.as_str())
            .expect("submit should return job_id");

        let cancel_params = serde_json::json!({ "job_id": job_id });
        let cancel_req = mk_request("compute.cancel", Some(cancel_params), 2);
        let cancel_resp = handler.handle_request(&cancel_req).await;

        assert!(cancel_resp.error.is_none());
        let result = cancel_resp.result.expect("result present");
        assert_eq!(result["cancelled"], true);
    }

    #[test]
    fn test_jsonrpc_response_serialization() {
        let success = JsonRpcResponse {
            jsonrpc: std::borrow::Cow::Borrowed("2.0"),
            result: Some(serde_json::json!({"ok": true})),
            error: None,
            id: serde_json::json!(1),
        };
        let json = serde_json::to_string(&success).expect("Serialize failed");
        assert!(json.contains("\"result\""));
        assert!(json.contains("\"ok\""));
        assert!(!json.contains("\"error\""));

        let failure = JsonRpcResponse {
            jsonrpc: std::borrow::Cow::Borrowed("2.0"),
            result: None,
            error: Some(JsonRpcError::method_not_found("foo")),
            id: serde_json::json!(2),
        };
        let json_err = serde_json::to_string(&failure).expect("Serialize failed");
        assert!(json_err.contains("\"error\""));
    }

    #[test]
    fn test_jsonrpc_request_with_params_array() {
        let json = r#"{"jsonrpc":"2.0","method":"foo","params":[1,2],"id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).expect("Parse failed");
        assert!(req.params.is_some());
    }
}
