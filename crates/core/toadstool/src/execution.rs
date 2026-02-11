//! Execution types and runtime engine interface

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

use crate::ToadStoolResult;

/// Execution request containing all information needed to run a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Unique identifier for this execution
    pub execution_id: Uuid,
    /// The workload to execute
    pub workload: crate::WorkloadSpec,
    /// Preferred runtime (hint for scheduling)
    pub runtime_hint: Option<RuntimeType>,
    /// Resource requirements
    pub resources: crate::resources::ResourceRequirements,
    /// Security context
    pub security_context: crate::SecurityContext,
    /// Maximum execution time
    pub timeout: Option<Duration>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Input data for the workload
    pub input_data: ExecutionInput,
    /// Callback configuration
    pub callback_config: Option<CallbackConfig>,
    /// Encryption configuration (for secure distributed workloads)
    pub encryption_config: Option<crate::encryption::EncryptionConfig>,
}

impl Default for ExecutionRequest {
    fn default() -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            workload: crate::WorkloadSpec::default(),
            runtime_hint: None,
            resources: crate::resources::ResourceRequirements::default(),
            security_context: crate::SecurityContext::default(),
            timeout: Some(Duration::from_secs(300)),
            environment: HashMap::new(),
            input_data: ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        }
    }
}

/// Response from an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    /// Execution identifier
    pub execution_id: Uuid,
    /// Execution status
    pub status: ExecutionStatus,
    /// Output from the execution
    pub output: ExecutionOutput,
    /// Runtime metrics
    pub metrics: crate::RuntimeMetrics,
    /// Total execution duration
    pub duration: Duration,
    /// Runtime that was used
    pub runtime_used: RuntimeType,
    /// Warnings generated during execution
    pub warnings: Vec<String>,
}

impl Default for ExecutionResponse {
    fn default() -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            output: ExecutionOutput::default(),
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_secs(0),
            runtime_used: RuntimeType::Native,
            warnings: Vec::new(),
        }
    }
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Execution completed successfully
    Success,
    /// Execution failed
    Failed { error: String },
    /// Execution was cancelled
    Cancelled,
    /// Execution timed out
    TimedOut,
    /// Execution is still running
    Running,
    /// Execution is pending
    Pending,
}

/// Input data for execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionInput {
    /// Binary data
    pub data: Vec<u8>,
    /// Input format
    pub format: Option<String>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Output from execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionOutput {
    /// Binary output data
    pub data: Vec<u8>,
    /// Standard output (text)
    pub stdout: Option<String>,
    /// Standard error (text)
    pub stderr: Option<String>,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Output format
    pub format: Option<String>,
    /// Result metadata
    pub result: HashMap<String, String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Callback configuration for execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackConfig {
    /// Callback URL
    pub url: String,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Events to callback on
    pub events: Vec<CallbackEvent>,
}

/// Events that can trigger callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackEvent {
    /// Execution started
    Started,
    /// Execution completed
    Completed,
    /// Execution failed
    Failed,
    /// Progress update
    Progress,
}

/// Types of runtime engines
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RuntimeType {
    /// Native process execution
    Native,
    /// WebAssembly execution
    Wasm,
    /// Container execution
    Container,
    /// GPU acceleration
    Gpu,
    /// Python runtime
    Python,
    /// Custom runtime
    Custom(String),
}

/// Runtime engine capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    /// Supported workload types
    pub supported_workloads: Vec<crate::WorkloadType>,
    /// Maximum concurrent executions
    pub max_concurrent_executions: Option<u32>,
    /// Supported architectures
    pub supported_architectures: Vec<String>,
    /// Platform-specific features
    pub platform_features: HashMap<String, bool>,
    /// Runtime version
    pub version: String,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    /// Runtime-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Resource limits
    pub resource_limits: Option<crate::resources::ResourceLimits>,
    /// Security settings
    pub security_settings: Option<crate::SecuritySettings>,
    /// Logging configuration
    pub logging: Option<LoggingConfig>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format
    pub format: String,
    /// Log destination
    pub destination: String,
}

/// Core trait for all runtime execution engines in the ToadStool universal compute platform.
///
/// This trait defines the interface that all runtime engines must implement to participate
/// in workload execution. Implementations can support various execution environments:
/// Native binaries, WASM modules, containers, GPU compute, or custom runtimes.
///
/// # Design Principles
///
/// - **Clear lifecycle management**: Initialize → Execute → Shutdown pattern
/// - **Capability declaration**: Self-describe supported workload types
/// - **Comprehensive monitoring**: Real-time metrics and health checks
/// - **Thread-safe execution**: Full `Send + Sync` for concurrent workloads
/// - **Polymorphic selection**: Used via `Box<dyn RuntimeEngine>` for runtime dispatch
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────┐
/// │         RuntimeOrchestrator                 │
/// │  (Selects appropriate engine for workload)  │
/// └─────────────────────────────────────────────┘
///                    │
///        ┌───────────┴───────────┐
///        ▼                       ▼
///   Box<dyn RuntimeEngine>  Box<dyn RuntimeEngine>
///        │                       │
///   NativeEngine           WasmEngine
/// ```
///
/// # Example Implementation
///
/// ```ignore
/// use toadstool::execution::{RuntimeEngine, ExecutionRequest, ExecutionResponse};
/// use toadstool::{ToadStoolResult, RuntimeMetrics};
/// use async_trait::async_trait;
///
/// pub struct MyCustomRuntime {
///     initialized: bool,
///     workload_count: u64,
/// }
///
/// #[async_trait]
/// impl RuntimeEngine for MyCustomRuntime {
///     async fn initialize(&mut self, config: RuntimeConfig) -> ToadStoolResult<()> {
///         // Setup runtime environment (one-time initialization)
///         self.initialized = true;
///         tracing::info!("MyCustomRuntime initialized");
///         Ok(())
///     }
///     
///     async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
///         // Validate we're initialized
///         if !self.initialized {
///             return Err(ToadStoolError::Execution(
///                 ExecutionError::RuntimeNotInitialized
///             ));
///         }
///
///         // Execute the workload
///         let start = std::time::Instant::now();
///         let output = self.run_workload(&request).await?;
///         let duration = start.elapsed();
///
///         // Return response with metrics
///         Ok(ExecutionResponse {
///             execution_id: request.execution_id,
///             status: ExecutionStatus::Success,
///             output,
///             duration,
///             runtime_used: RuntimeType::Custom("my-runtime".to_string()),
///             metrics: self.collect_metrics(),
///             warnings: Vec::new(),
///         })
///     }
///     
///     fn get_capabilities(&self) -> RuntimeCapabilities {
///         RuntimeCapabilities {
///             runtime_type: RuntimeType::Custom("my-runtime".to_string()),
///             supported_workload_types: vec![WorkloadType::Custom],
///             max_concurrent_executions: 10,
///             supports_gpu: false,
///             supports_networking: true,
///         }
///     }
///     
///     fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
///         matches!(workload_type, WorkloadType::Custom)
///     }
///     
///     async fn get_metrics(&self) -> ToadStoolResult<RuntimeMetrics> {
///         Ok(self.collect_metrics())
///     }
///     
///     async fn shutdown(&mut self) -> ToadStoolResult<()> {
///         // Clean up resources before dropping
///         tracing::info!("Shutting down MyCustomRuntime");
///         self.initialized = false;
///         Ok(())
///     }
/// }
/// ```
///
/// # Trait Invariants
///
/// Implementations MUST maintain these invariants:
///
/// 1. **Initialization**: `initialize()` must be called exactly once before any `execute()` calls
/// 2. **Thread Safety**: All methods must be safe to call from multiple threads simultaneously
/// 3. **Idempotency**: `execute()` should be idempotent where possible (same input → same output)
/// 4. **Resource Cleanup**: `shutdown()` must release all held resources
/// 5. **Error Propagation**: Errors must be propagated clearly with context
/// 6. **Timeout Respect**: Implementations should respect `ExecutionRequest::timeout`
/// 7. **Metrics Accuracy**: `get_metrics()` must return current, accurate data
///
/// # Performance Characteristics
///
/// - **Initialization**: O(1) - Called once at startup
/// - **Execute**: Variable - Depends on workload complexity
/// - **get_capabilities**: O(1) - Should be fast (called frequently)
/// - **supports_workload**: O(1) - Should be fast (called frequently)
/// - **get_metrics**: O(1) - Should be fast (called for health checks)
/// - **Shutdown**: O(1) - Called once at teardown
///
/// # Concurrency
///
/// The `execute()` method takes `&self` (shared reference), enabling concurrent execution:
///
/// ```ignore
/// let engine: Arc<dyn RuntimeEngine> = Arc::new(MyRuntime::new());
///
/// // Multiple concurrent executions are allowed
/// let (result1, result2) = tokio::join!(
///     engine.execute(request1),
///     engine.execute(request2)
/// );
/// ```
///
/// Implementations must use internal synchronization (Mutex, RwLock, etc.) if needed.
///
/// # Error Handling
///
/// Implementations should return specific errors using `ToadStoolError::Execution`:
///
/// ```ignore
/// // Good: Specific error with context
/// return Err(ToadStoolError::Execution(ExecutionError::WorkloadFailed {
///     reason: format!("Binary '{}' not found", binary_name),
///     exit_code: Some(127),
/// }));
///
/// // Avoid: Generic errors without context
/// return Err(ToadStoolError::Unknown("Something failed".into()));
/// ```
///
/// # Testing
///
/// For testing, use the mock runtime from `toadstool_testing`:
///
/// ```ignore
/// use toadstool_testing::mocks::MockRuntimeEngine;
///
/// let mut mock = MockRuntimeEngine::new();
/// mock.expect_execute()
///     .returning(|_| Ok(ExecutionResponse::default()));
///
/// let result = mock.execute(request).await?;
/// ```
///
/// # Common Pitfalls
///
/// 1. **Blocking in async**: Don't use blocking I/O in async methods
///    ```ignore
///    // ❌ Bad: Blocks async runtime
///    std::fs::read("file.txt")?;
///    
///    // ✅ Good: Uses async I/O
///    tokio::fs::read("file.txt").await?;
///    ```
///
/// 2. **Resource leaks**: Always clean up in `shutdown()` or `Drop`
///    ```ignore
///    // ✅ Good: Cleanup in shutdown
///    async fn shutdown(&mut self) -> ToadStoolResult<()> {
///        self.cleanup_temp_files().await?;
///        self.close_connections().await?;
///        Ok(())
///    }
///    ```
///
/// 3. **Panics**: Never panic in trait methods (return errors instead)
///    ```ignore
///    // ❌ Bad: Panics
///    let value = map.get(key).unwrap();
///    
///    // ✅ Good: Returns error
///    let value = map.get(key)
///        .ok_or_else(|| ToadStoolError::Execution(...))?;
///    ```
///
/// # Performance Note: Native Async Traits
///
/// This trait uses native async Rust (`Pin<Box<dyn Future<...>>>`) instead of the async_trait macro.
/// This provides zero-cost abstraction and improved performance.
///
/// **Benefits**: Zero macro overhead, faster compilation, more efficient stack usage
/// **Trade-off**: Slightly more verbose type signatures
///
/// # See Also
///
/// - [`RuntimeOrchestrator`] - Selects and manages runtime engines
/// - [`ExecutionRequest`] - Input to execution
/// - [`ExecutionResponse`] - Output from execution
/// - [`RuntimeCapabilities`] - Capability declaration
/// - `RuntimeMetrics` - Performance metrics (planned)
pub trait RuntimeEngine: Send + Sync {
    /// Initialize the runtime engine with the provided configuration.
    ///
    /// This method is called once before any workload execution.
    fn initialize(
        &mut self,
        config: RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Execute a workload and return the result.
    ///
    /// This is the primary method for workload execution.
    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>>;

    /// Get the capabilities supported by this runtime.
    ///
    /// Used for workload routing and compatibility checking.
    fn get_capabilities(&self) -> RuntimeCapabilities;

    /// Check if this runtime supports a specific workload type.
    ///
    /// Returns `true` if the runtime can execute the given workload type.
    fn supports_workload(&self, workload_type: &crate::WorkloadType) -> bool;

    /// Get current runtime metrics.
    ///
    /// Returns performance and resource usage metrics.
    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<crate::RuntimeMetrics>> + Send + '_>>;

    /// Shutdown the runtime engine and clean up resources.
    ///
    /// This method is called once before the engine is dropped.
    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;
}

// RuntimeOrchestrator is now defined in runtime.rs module
// Re-export it here for backward compatibility
pub use crate::runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ============== ExecutionRequest tests ==============

    #[test]
    fn execution_request_default_construction() {
        let req = ExecutionRequest::default();
        assert!(req.runtime_hint.is_none());
        assert_eq!(req.environment.len(), 0);
        assert_eq!(req.timeout, Some(Duration::from_secs(300)));
        assert!(req.callback_config.is_none());
        assert!(req.encryption_config.is_none());
    }

    #[test]
    fn execution_request_field_access() {
        let mut req = ExecutionRequest::default();
        let id = Uuid::new_v4();
        req.execution_id = id;
        req.runtime_hint = Some(RuntimeType::Wasm);
        req.timeout = Some(Duration::from_secs(60));
        req.environment.insert("FOO".to_string(), "bar".to_string());

        assert_eq!(req.execution_id, id);
        assert_eq!(req.runtime_hint, Some(RuntimeType::Wasm));
        assert_eq!(req.timeout, Some(Duration::from_secs(60)));
        assert_eq!(req.environment.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn execution_request_with_callback_config() {
        let mut req = ExecutionRequest::default();
        req.callback_config = Some(CallbackConfig {
            url: "https://example.com/callback".to_string(),
            auth_token: Some("secret".to_string()),
            events: vec![CallbackEvent::Started, CallbackEvent::Completed],
        });

        let config = req.callback_config.as_ref().unwrap();
        assert_eq!(config.url, "https://example.com/callback");
        assert_eq!(config.auth_token.as_deref(), Some("secret"));
        assert_eq!(config.events.len(), 2);
    }

    #[test]
    fn execution_request_with_encryption_config() {
        let mut req = ExecutionRequest::default();
        req.encryption_config = Some(crate::encryption::EncryptionConfig::default());

        assert!(req.encryption_config.is_some());
    }

    #[test]
    fn execution_request_clone() {
        let req = ExecutionRequest::default();
        let cloned = req.clone();
        assert_eq!(req.execution_id, cloned.execution_id);
    }

    // ============== ExecutionResponse tests ==============

    #[test]
    fn execution_response_default_construction() {
        let resp = ExecutionResponse::default();
        assert_eq!(resp.status, ExecutionStatus::Success);
        assert_eq!(resp.runtime_used, RuntimeType::Native);
        assert_eq!(resp.duration, Duration::from_secs(0));
        assert!(resp.warnings.is_empty());
    }

    #[test]
    fn execution_response_with_all_fields() {
        let resp = ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Failed {
                error: "oops".to_string(),
            },
            output: ExecutionOutput {
                stdout: Some("hello".to_string()),
                stderr: Some("err".to_string()),
                exit_code: Some(1),
                ..Default::default()
            },
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_millis(1500),
            runtime_used: RuntimeType::Container,
            warnings: vec!["deprecated".to_string()],
        };

        assert!(matches!(resp.status, ExecutionStatus::Failed { .. }));
        assert_eq!(resp.output.stdout.as_deref(), Some("hello"));
        assert_eq!(resp.output.exit_code, Some(1));
        assert_eq!(resp.runtime_used, RuntimeType::Container);
        assert_eq!(resp.warnings, vec!["deprecated"]);
    }

    #[test]
    fn execution_response_clone() {
        let resp = ExecutionResponse::default();
        let cloned = resp.clone();
        assert_eq!(resp.status, cloned.status);
    }

    // ============== ExecutionStatus tests ==============

    #[test]
    fn execution_status_all_variants() {
        let success = ExecutionStatus::Success;
        let failed = ExecutionStatus::Failed {
            error: "test error".to_string(),
        };
        let cancelled = ExecutionStatus::Cancelled;
        let timed_out = ExecutionStatus::TimedOut;
        let running = ExecutionStatus::Running;
        let pending = ExecutionStatus::Pending;

        assert_eq!(success, ExecutionStatus::Success);
        assert!(matches!(failed, ExecutionStatus::Failed { error } if error == "test error"));
        assert_eq!(cancelled, ExecutionStatus::Cancelled);
        assert_eq!(timed_out, ExecutionStatus::TimedOut);
        assert_eq!(running, ExecutionStatus::Running);
        assert_eq!(pending, ExecutionStatus::Pending);
    }

    #[test]
    fn execution_status_comparisons() {
        assert_eq!(ExecutionStatus::Success, ExecutionStatus::Success);
        assert_ne!(
            ExecutionStatus::Success,
            ExecutionStatus::Failed {
                error: "x".to_string(),
            }
        );
        assert_ne!(
            ExecutionStatus::Failed {
                error: "a".to_string(),
            },
            ExecutionStatus::Failed {
                error: "b".to_string(),
            }
        );
        assert_eq!(
            ExecutionStatus::Failed {
                error: "same".to_string(),
            },
            ExecutionStatus::Failed {
                error: "same".to_string(),
            }
        );
    }

    #[test]
    fn execution_status_debug() {
        let status = ExecutionStatus::Success;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Success"));
    }

    // ============== RuntimeType tests ==============

    #[test]
    fn runtime_type_all_variants() {
        let _native = RuntimeType::Native;
        let _wasm = RuntimeType::Wasm;
        let _container = RuntimeType::Container;
        let _gpu = RuntimeType::Gpu;
        let _python = RuntimeType::Python;
        let custom = RuntimeType::Custom("my-runtime".to_string());

        assert_eq!(custom, RuntimeType::Custom("my-runtime".to_string()));
    }

    #[test]
    fn runtime_type_comparisons() {
        assert_eq!(RuntimeType::Native, RuntimeType::Native);
        assert_ne!(RuntimeType::Native, RuntimeType::Wasm);
        assert_eq!(
            RuntimeType::Custom("x".to_string()),
            RuntimeType::Custom("x".to_string())
        );
        assert_ne!(
            RuntimeType::Custom("x".to_string()),
            RuntimeType::Custom("y".to_string())
        );
    }

    #[test]
    fn runtime_type_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let t1 = RuntimeType::Native;
        let t2 = RuntimeType::Native;
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        t1.hash(&mut h1);
        t2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn runtime_type_debug() {
        let rt = RuntimeType::Gpu;
        let debug_str = format!("{:?}", rt);
        assert!(debug_str.contains("Gpu"));
    }

    // ============== RuntimeConfig tests ==============

    #[test]
    fn runtime_config_default() {
        let config = RuntimeConfig::default();
        assert!(config.settings.is_empty());
        assert!(config.resource_limits.is_none());
        assert!(config.security_settings.is_none());
        assert!(config.logging.is_none());
    }

    #[test]
    fn runtime_config_with_settings() {
        let mut config = RuntimeConfig::default();
        config
            .settings
            .insert("foo".to_string(), serde_json::json!("bar"));

        assert_eq!(config.settings.get("foo"), Some(&serde_json::json!("bar")));
    }

    #[test]
    fn runtime_config_with_resource_limits() {
        let mut config = RuntimeConfig::default();
        config.resource_limits = Some(crate::resources::ResourceLimits::default());

        assert!(config.resource_limits.is_some());
    }

    #[test]
    fn runtime_config_clone() {
        let config = RuntimeConfig::default();
        let cloned = config.clone();
        assert_eq!(config.settings.len(), cloned.settings.len());
    }

    // ============== ExecutionInput tests ==============

    #[test]
    fn execution_input_default() {
        let input = ExecutionInput::default();
        assert!(input.data.is_empty());
        assert!(input.format.is_none());
        assert!(input.metadata.is_empty());
    }

    #[test]
    fn execution_input_with_data() {
        let mut input = ExecutionInput::default();
        input.data = vec![1, 2, 3];
        input.format = Some("json".to_string());
        input
            .metadata
            .insert("key".to_string(), "value".to_string());

        assert_eq!(input.data, vec![1, 2, 3]);
        assert_eq!(input.format.as_deref(), Some("json"));
        assert_eq!(input.metadata.get("key"), Some(&"value".to_string()));
    }

    // ============== ExecutionOutput tests ==============

    #[test]
    fn execution_output_default() {
        let output = ExecutionOutput::default();
        assert!(output.data.is_empty());
        assert!(output.stdout.is_none());
        assert!(output.stderr.is_none());
        assert!(output.exit_code.is_none());
        assert!(output.result.is_empty());
    }

    #[test]
    fn execution_output_with_fields() {
        let output = ExecutionOutput {
            data: vec![42u8],
            stdout: Some("out".to_string()),
            stderr: Some("err".to_string()),
            exit_code: Some(0),
            format: Some("binary".to_string()),
            result: HashMap::from([("k".to_string(), "v".to_string())]),
            metadata: HashMap::new(),
        };

        assert_eq!(output.data, vec![42]);
        assert_eq!(output.stdout.as_deref(), Some("out"));
        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.result.get("k"), Some(&"v".to_string()));
    }

    // ============== CallbackConfig and CallbackEvent tests ==============

    #[test]
    fn callback_event_variants() {
        let _started = CallbackEvent::Started;
        let _completed = CallbackEvent::Completed;
        let _failed = CallbackEvent::Failed;
        let _progress = CallbackEvent::Progress;
    }

    #[test]
    fn callback_config_construction() {
        let config = CallbackConfig {
            url: "https://example.com".to_string(),
            auth_token: None,
            events: vec![CallbackEvent::Started, CallbackEvent::Failed],
        };

        assert_eq!(config.url, "https://example.com");
        assert!(config.auth_token.is_none());
        assert_eq!(config.events.len(), 2);
    }

    // ============== RuntimeCapabilities tests ==============

    #[test]
    fn runtime_capabilities_construction() {
        let caps = RuntimeCapabilities {
            supported_workloads: vec![crate::WorkloadType::Native, crate::WorkloadType::Wasm],
            max_concurrent_executions: Some(8),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: HashMap::from([("gpu".to_string(), true)]),
            version: "1.0".to_string(),
        };

        assert_eq!(caps.supported_workloads.len(), 2);
        assert_eq!(caps.max_concurrent_executions, Some(8));
        assert_eq!(caps.supported_architectures, vec!["x86_64"]);
        assert_eq!(caps.platform_features.get("gpu"), Some(&true));
        assert_eq!(caps.version, "1.0");
    }

    // ============== LoggingConfig tests ==============

    #[test]
    fn logging_config_construction() {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            destination: "stderr".to_string(),
        };

        assert_eq!(config.level, "info");
        assert_eq!(config.format, "json");
        assert_eq!(config.destination, "stderr");
    }

    // ============== Serialization round-trip tests ==============

    #[test]
    fn execution_request_serialization_roundtrip() {
        let req = ExecutionRequest::default();
        let json = serde_json::to_string(&req).expect("serialize");
        let deserialized: ExecutionRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(req.execution_id, deserialized.execution_id);
        assert_eq!(req.runtime_hint, deserialized.runtime_hint);
    }

    #[test]
    fn execution_response_serialization_roundtrip() {
        let resp = ExecutionResponse::default();
        let json = serde_json::to_string(&resp).expect("serialize");
        let deserialized: ExecutionResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(resp.execution_id, deserialized.execution_id);
        assert_eq!(resp.status, deserialized.status);
    }

    #[test]
    fn execution_status_serialization_roundtrip() {
        let statuses = [
            ExecutionStatus::Success,
            ExecutionStatus::Failed {
                error: "err".to_string(),
            },
            ExecutionStatus::Cancelled,
            ExecutionStatus::TimedOut,
            ExecutionStatus::Running,
            ExecutionStatus::Pending,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).expect("serialize");
            let deserialized: ExecutionStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn runtime_type_serialization_roundtrip() {
        let types = [
            RuntimeType::Native,
            RuntimeType::Wasm,
            RuntimeType::Container,
            RuntimeType::Gpu,
            RuntimeType::Python,
            RuntimeType::Custom("custom-rt".to_string()),
        ];

        for rt in types {
            let json = serde_json::to_string(&rt).expect("serialize");
            let deserialized: RuntimeType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(rt, deserialized);
        }
    }

    #[test]
    fn runtime_config_serialization_roundtrip() {
        let config = RuntimeConfig::default();
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: RuntimeConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config.settings.len(), deserialized.settings.len());
    }

    #[test]
    fn execution_input_serialization_roundtrip() {
        let mut input = ExecutionInput::default();
        input.data = vec![1, 2, 3];
        input.format = Some("bin".to_string());
        let json = serde_json::to_string(&input).expect("serialize");
        let deserialized: ExecutionInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(input.data, deserialized.data);
        assert_eq!(input.format, deserialized.format);
    }

    #[test]
    fn execution_output_serialization_roundtrip() {
        let output = ExecutionOutput {
            stdout: Some("hello".to_string()),
            exit_code: Some(0),
            ..Default::default()
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: ExecutionOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(output.stdout, deserialized.stdout);
        assert_eq!(output.exit_code, deserialized.exit_code);
    }

    #[test]
    fn callback_config_serialization_roundtrip() {
        let config = CallbackConfig {
            url: "https://cb.example.com".to_string(),
            auth_token: Some("token".to_string()),
            events: vec![CallbackEvent::Completed],
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: CallbackConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config.url, deserialized.url);
        assert_eq!(config.events.len(), deserialized.events.len());
    }

    #[test]
    fn callback_event_serialization_roundtrip() {
        let events = [
            CallbackEvent::Started,
            CallbackEvent::Completed,
            CallbackEvent::Failed,
            CallbackEvent::Progress,
        ];
        for event in events {
            let json = serde_json::to_string(&event).expect("serialize");
            let deserialized: CallbackEvent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(
                std::mem::discriminant(&event),
                std::mem::discriminant(&deserialized)
            );
        }
    }

    #[test]
    fn logging_config_serialization_roundtrip() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            format: "text".to_string(),
            destination: "stdout".to_string(),
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: LoggingConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config.level, deserialized.level);
    }

    #[test]
    fn runtime_capabilities_serialization_roundtrip() {
        let caps = RuntimeCapabilities {
            supported_workloads: vec![crate::WorkloadType::Native],
            max_concurrent_executions: Some(4),
            supported_architectures: vec!["aarch64".to_string()],
            platform_features: HashMap::new(),
            version: "2.0".to_string(),
        };
        let json = serde_json::to_string(&caps).expect("serialize");
        let deserialized: RuntimeCapabilities = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(caps.version, deserialized.version);
    }
}
