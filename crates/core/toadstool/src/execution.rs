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
