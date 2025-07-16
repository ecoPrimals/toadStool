---
title: ToadStool Execution Environments Specification
description: Comprehensive specification for multi-runtime execution environments
version: 1.0.0
date: 2025-01-26
author: ToadStool Development Team
priority: CRITICAL
status: CORE_SPEC
---

# 🏃 Execution Environments Specification

## Executive Summary

ToadStool provides **universal execution environments** that can run any workload, in any language, on any platform. The execution layer abstracts away runtime complexity while providing consistent security, monitoring, and resource management.

---

## 🎯 **Design Principles**

### **Universal Compatibility**
```yaml
compatibility_matrix:
  languages: ["any_language", "runtime_agnostic"]
  platforms: ["linux", "macos", "windows", "container_platforms"]
  architectures: ["x86_64", "aarch64", "arm64", "configurable"]
  runtimes: ["wasm", "container", "native", "gpu", "extensible"]
```

### **Configuration-Driven Architecture**
```rust
// Everything is configurable, nothing is hardcoded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Runtime selection strategy
    pub runtime_strategy: RuntimeStrategy,
    /// Platform-specific optimizations
    pub platform_config: PlatformConfig,
    /// Resource allocation policies
    pub resource_policy: ResourcePolicy,
    /// Security isolation level
    pub isolation_level: IsolationLevel,
    /// Performance optimization flags
    pub optimization_flags: OptimizationFlags,
}
```

---

## 🛠️ **Runtime Architecture**

### **Unified Runtime Interface**
```rust
#[async_trait::async_trait]
pub trait RuntimeEngine: Send + Sync + Debug {
    /// Initialize the runtime with configuration
    async fn initialize(&mut self, config: RuntimeConfig) -> Result<()>;
    
    /// Execute a workload with specified context
    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResponse>;
    
    /// Get runtime capabilities and metadata
    fn get_capabilities(&self) -> RuntimeCapabilities;
    
    /// Check if runtime supports the given workload type
    fn supports_workload(&self, workload_type: &WorkloadType) -> bool;
    
    /// Get runtime health and performance metrics
    async fn get_metrics(&self) -> Result<RuntimeMetrics>;
    
    /// Shutdown runtime gracefully
    async fn shutdown(&mut self) -> Result<()>;
}

/// Universal execution request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Unique execution identifier
    pub execution_id: Uuid,
    /// Workload specification
    pub workload: WorkloadSpec,
    /// Runtime preferences (auto-selected if None)
    pub runtime_hint: Option<RuntimeType>,
    /// Resource requirements and limits
    pub resources: ResourceRequirements,
    /// Security and isolation context
    pub security_context: SecurityContext,
    /// Execution timeout
    pub timeout: Option<Duration>,
    /// Environment variables (runtime-agnostic)
    pub environment: HashMap<String, String>,
    /// Input data and parameters
    pub input_data: ExecutionInput,
    /// Callback configuration for async results
    pub callback_config: Option<CallbackConfig>,
}
```

---

## 🐳 **Container Runtime**

### **Multi-Engine Support**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container engine selection
    pub engine: ContainerEngine,
    /// Image registry configuration
    pub registry_config: RegistryConfig,
    /// Network isolation settings
    pub network_policy: NetworkPolicy,
    /// Volume mounting rules
    pub volume_policy: VolumePolicy,
    /// Resource limits
    pub resource_limits: ContainerResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerEngine {
    /// Docker engine with custom socket path
    Docker { socket_path: PathBuf },
    /// Containerd with custom address
    Containerd { address: String },
    /// Podman engine
    Podman { socket_path: PathBuf },
    /// Custom engine implementation
    Custom { 
        name: String,
        config: HashMap<String, Value>
    },
}
```

### **Language-Agnostic Container Execution**
```rust
impl RuntimeEngine for ContainerRuntime {
    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResponse> {
        let container_spec = match &request.workload {
            WorkloadSpec::Container { image, command, args, .. } => {
                ContainerSpec {
                    image: image.clone(),
                    command: command.clone(),
                    args: args.clone(),
                    environment: request.environment,
                    resource_limits: self.calculate_limits(&request.resources)?,
                    security_context: self.map_security_context(&request.security_context)?,
                    network_config: self.resolve_network_config()?,
                    volume_mounts: self.resolve_volumes(&request.workload)?,
                }
            }
            _ => return Err(ExecutionError::UnsupportedWorkload),
        };

        let execution_result = self.engine
            .create_and_run_container(container_spec)
            .await?;

        Ok(ExecutionResponse::from_container_result(execution_result))
    }
}
```

---

## 🕸️ **WebAssembly Runtime**

### **Universal WASM Execution**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    /// WASM engine selection
    pub engine: WasmEngine,
    /// WASI capability configuration
    pub wasi_config: WasiConfig,
    /// Memory and resource limits
    pub limits: WasmLimits,
    /// Host function imports
    pub host_functions: Vec<HostFunctionBinding>,
    /// Module caching strategy
    pub caching_policy: CachingPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmEngine {
    /// Wasmtime engine with custom config
    Wasmtime { 
        config: WasmtimeConfig,
        features: Vec<WasmFeature>
    },
    /// Wasmer engine
    Wasmer {
        compiler: WasmerCompiler,
        features: Vec<WasmFeature>
    },
    /// Custom WASM engine
    Custom {
        name: String,
        config: HashMap<String, Value>
    },
}
```

### **WASI Integration**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasiConfig {
    /// Filesystem access permissions
    pub filesystem_access: FilesystemPolicy,
    /// Network access configuration
    pub network_access: NetworkPolicy,
    /// Environment variable exposure
    pub environment_policy: EnvironmentPolicy,
    /// Standard I/O configuration
    pub stdio_config: StdioConfig,
    /// Clock and time access
    pub time_access: TimePolicy,
    /// Random number generation access
    pub random_access: RandomPolicy,
}

impl RuntimeEngine for WasmRuntime {
    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResponse> {
        let wasm_module = match &request.workload {
            WorkloadSpec::Wasm { module_source, entry_point, .. } => {
                self.load_and_validate_module(module_source).await?
            }
            _ => return Err(ExecutionError::UnsupportedWorkload),
        };

        // Configure WASI context based on security requirements
        let wasi_context = self.create_wasi_context(
            &request.security_context,
            &request.environment,
            &request.input_data
        )?;

        // Execute with resource monitoring
        let execution_result = self.engine
            .execute_with_context(wasm_module, wasi_context)
            .await?;

        Ok(ExecutionResponse::from_wasm_result(execution_result))
    }
}
```

---

## ⚡ **Native Runtime**

### **Secure Native Execution**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeConfig {
    /// Executable validation policy
    pub validation_policy: ExecutableValidationPolicy,
    /// Dynamic library loading rules
    pub library_policy: LibraryPolicy,
    /// Process isolation configuration
    pub isolation_config: ProcessIsolationConfig,
    /// System call filtering
    pub syscall_policy: SyscallPolicy,
    /// Performance monitoring
    pub monitoring_config: NativeMonitoringConfig,
}

impl RuntimeEngine for NativeRuntime {
    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResponse> {
        let native_spec = match &request.workload {
            WorkloadSpec::Native { executable, args, working_dir, .. } => {
                // Validate executable before execution
                self.validate_executable(executable).await?;
                
                NativeExecutionSpec {
                    executable: executable.clone(),
                    arguments: args.clone(),
                    working_directory: working_dir.clone(),
                    environment: request.environment,
                    resource_limits: self.map_resource_limits(&request.resources)?,
                    security_context: self.create_security_context(&request.security_context)?,
                }
            }
            _ => return Err(ExecutionError::UnsupportedWorkload),
        };

        // Create isolated execution environment
        let execution_context = self.create_isolated_context(native_spec).await?;
        
        // Execute with comprehensive monitoring
        let execution_result = self.execute_in_context(execution_context).await?;

        Ok(ExecutionResponse::from_native_result(execution_result))
    }
}
```

---

## 🎮 **GPU Compute Runtime**

### **Multi-Platform GPU Support**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// GPU compute backend
    pub backend: GpuBackend,
    /// Memory allocation strategy
    pub memory_strategy: GpuMemoryStrategy,
    /// Compute queue configuration
    pub queue_config: ComputeQueueConfig,
    /// Performance optimization flags
    pub optimization_flags: GpuOptimizations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuBackend {
    /// NVIDIA CUDA backend
    Cuda {
        version: String,
        devices: Vec<CudaDevice>,
        memory_pool_size: Option<usize>,
    },
    /// OpenCL backend
    OpenCL {
        platform: String,
        devices: Vec<OpenCLDevice>,
        context_config: OpenCLContextConfig,
    },
    /// Vulkan compute backend
    Vulkan {
        instance_config: VulkanInstanceConfig,
        device_selection: VulkanDeviceSelection,
    },
    /// WebGPU backend for web compatibility
    WebGPU {
        adapter_options: WebGPUAdapterOptions,
        limits: WebGPULimits,
    },
}
```

---

## 🔄 **Runtime Selection & Auto-Detection**

### **Intelligent Runtime Selection**
```rust
#[derive(Debug, Clone)]
pub struct RuntimeSelector {
    available_runtimes: Vec<Box<dyn RuntimeEngine>>,
    selection_strategy: SelectionStrategy,
    capability_cache: Arc<RwLock<HashMap<RuntimeType, RuntimeCapabilities>>>,
}

impl RuntimeSelector {
    /// Select optimal runtime for workload
    pub async fn select_runtime(
        &self, 
        workload: &WorkloadSpec,
        requirements: &ResourceRequirements,
        preferences: &RuntimePreferences
    ) -> Result<&dyn RuntimeEngine> {
        match preferences.strategy {
            SelectionStrategy::Performance => {
                self.select_by_performance(workload, requirements).await
            }
            SelectionStrategy::Security => {
                self.select_by_security(workload, requirements).await
            }
            SelectionStrategy::ResourceEfficiency => {
                self.select_by_efficiency(workload, requirements).await
            }
            SelectionStrategy::Automatic => {
                self.select_automatically(workload, requirements).await
            }
            SelectionStrategy::Specific(runtime_type) => {
                self.get_runtime_by_type(runtime_type)
            }
        }
    }
}
```

### **Workload Type Detection**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadSpec {
    /// Container-based workload
    Container {
        image: String,
        command: Option<Vec<String>>,
        args: Option<Vec<String>>,
        registry_auth: Option<RegistryAuth>,
    },
    
    /// WebAssembly module
    Wasm {
        module_source: WasmModuleSource,
        entry_point: Option<String>,
        imports: Vec<WasmImport>,
    },
    
    /// Native executable
    Native {
        executable: ExecutableSource,
        args: Vec<String>,
        working_dir: Option<PathBuf>,
        library_deps: Vec<LibraryDependency>,
    },
    
    /// GPU compute workload
    GpuCompute {
        kernel_source: GpuKernelSource,
        compute_type: GpuComputeType,
        data_bindings: Vec<GpuDataBinding>,
    },
    
    /// Multi-stage workflow
    Workflow {
        stages: Vec<WorkloadStage>,
        dependencies: Vec<StageDependency>,
        coordination: WorkflowCoordination,
    },
}
```

---

## 📊 **Performance & Monitoring**

### **Runtime Metrics Collection**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    /// Execution timing information
    pub timing: ExecutionTiming,
    /// Resource utilization
    pub resource_usage: ResourceUsage,
    /// Performance characteristics
    pub performance: PerformanceMetrics,
    /// Error and warning counts
    pub diagnostics: DiagnosticMetrics,
    /// Runtime-specific metrics
    pub runtime_specific: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTiming {
    /// Time to initialize runtime
    pub initialization_time: Duration,
    /// Time to start execution
    pub startup_time: Duration,
    /// Total execution time
    pub execution_time: Duration,
    /// Cleanup and shutdown time
    pub cleanup_time: Duration,
    /// End-to-end total time
    pub total_time: Duration,
}
```

---

## 🔧 **Configuration Management**

### **Hierarchical Configuration**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolConfig {
    /// Global runtime settings
    pub global: GlobalRuntimeConfig,
    /// Per-runtime configurations
    pub runtimes: HashMap<RuntimeType, RuntimeConfig>,
    /// Platform-specific overrides
    pub platform_overrides: HashMap<Platform, PlatformConfig>,
    /// Environment-specific settings
    pub environment_configs: HashMap<String, EnvironmentConfig>,
    /// Feature flags and experimental options
    pub feature_flags: FeatureFlags,
}

// Configuration loading with precedence
impl ToadStoolConfig {
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        
        // Load in order of precedence (lowest to highest)
        config.merge_from_file("/etc/toadstool/config.toml")?;
        config.merge_from_file(&format!("{}/.config/toadstool/config.toml", env::var("HOME")?))?;
        config.merge_from_file("./toadstool.toml")?;
        config.merge_from_env("TOADSTOOL_")?;
        config.merge_from_args()?;
        
        config.validate()?;
        Ok(config)
    }
}
```

---

## 🎛️ **Extension System**

### **Runtime Plugin Architecture**
```rust
#[async_trait::async_trait]
pub trait RuntimeExtension: Send + Sync {
    /// Extension metadata
    fn metadata(&self) -> ExtensionMetadata;
    
    /// Initialize extension with runtime context
    async fn initialize(&mut self, context: &RuntimeContext) -> Result<()>;
    
    /// Hook into execution lifecycle
    async fn on_execution_start(&self, request: &ExecutionRequest) -> Result<()>;
    async fn on_execution_complete(&self, response: &ExecutionResponse) -> Result<()>;
    
    /// Provide additional capabilities
    fn additional_capabilities(&self) -> Vec<Capability>;
    
    /// Custom workload type support
    fn supports_custom_workload(&self, workload_type: &str) -> bool;
}

// Extension registration system
pub struct ExtensionRegistry {
    extensions: Vec<Box<dyn RuntimeExtension>>,
    capability_index: HashMap<Capability, Vec<usize>>,
}
```

This specification establishes ToadStool as a truly universal, configurable execution platform that can handle any workload type while maintaining strong security and performance characteristics. Every aspect is designed to be configurable rather than hardcoded, ensuring maximum flexibility and adaptability. 