---
description: ENFORCE universal compute platform with recursive hosting and multi-runtime execution
globs: ["toadstool/src/**/*.rs", "toadstool/crates/**/*.rs"]
---

# Enhanced Compute Platform Specification

## Context
- When implementing universal compute orchestration
- When providing multi-runtime execution environments
- When supporting recursive ToadStool hosting
- When integrating with ecosystem-wide compute needs

## Requirements

### Universal Compute Execution
- Implement multiple runtime environments simultaneously
- Support container, WASM, native, and GPU execution
- Enable dynamic runtime selection based on workload
- Provide unified compute abstraction layer

### Recursive Hosting Capabilities
- Support ToadStool instances hosting other ToadStool instances
- Enable nested compute environment management
- Provide recursive resource allocation and monitoring
- Support distributed compute coordination

### Real-Time Execution Monitoring
- Implement real-time execution progress tracking
- Support bidirectional execution event streaming
- Enable resource usage monitoring and optimization
- Provide execution analytics and insights

## Architecture

### Universal Compute Engine
```rust
pub struct UniversalComputeEngine {
    runtime_manager: Arc<RuntimeManager>,
    resource_coordinator: Arc<ResourceCoordinator>,
    execution_monitor: Arc<ExecutionMonitor>,
    scheduler: Arc<ComputeScheduler>,
    sandbox_manager: Arc<SandboxManager>,
}

impl UniversalComputeEngine {
    pub async fn new(config: ComputeEngineConfig) -> Result<Self>;
    pub async fn start(&self) -> Result<()>;
    pub async fn execute_workload(&self, workload: Workload) -> Result<ExecutionResult>;
    pub async fn stream_execution(&self, workload: Workload) -> Result<ExecutionStream>;
    pub async fn monitor_execution(&self, execution_id: &str) -> Result<ExecutionMonitor>;
}
```

### Runtime Manager
```rust
pub struct RuntimeManager {
    runtime_registry: Arc<RuntimeRegistry>,
    runtime_pools: HashMap<RuntimeType, Arc<RuntimePool>>,
    capability_matcher: Arc<CapabilityMatcher>,
    performance_optimizer: Arc<PerformanceOptimizer>,
}

#[async_trait]
pub trait Runtime {
    async fn execute(&self, workload: Workload) -> Result<ExecutionResult>;
    async fn stream_execution(&self, workload: Workload) -> Result<ExecutionStream>;
    async fn monitor_resources(&self) -> Result<ResourceUsage>;
    fn capabilities(&self) -> Vec<RuntimeCapability>;
    fn runtime_type(&self) -> RuntimeType;
}

#[derive(Debug, Clone)]
pub enum RuntimeType {
    Container,
    WebAssembly,
    Native,
    GPU,
    Recursive,
}

#[derive(Debug, Clone)]
pub enum RuntimeCapability {
    Sandboxing,
    NetworkAccess,
    FileSystemAccess,
    GPUAccess,
    RecursiveHosting,
    StreamingExecution,
}
```

### Resource Coordinator
```rust
pub struct ResourceCoordinator {
    resource_allocator: Arc<ResourceAllocator>,
    load_balancer: Arc<ComputeLoadBalancer>,
    quota_manager: Arc<QuotaManager>,
    optimization_engine: Arc<OptimizationEngine>,
}

impl ResourceCoordinator {
    pub async fn allocate_resources(&self, request: ResourceRequest) -> Result<ResourceAllocation>;
    pub async fn deallocate_resources(&self, allocation_id: &str) -> Result<()>;
    pub async fn balance_load(&self, workloads: Vec<Workload>) -> Result<LoadBalancingResult>;
    pub async fn optimize_placement(&self, workload: &Workload) -> Result<PlacementDecision>;
}

#[derive(Debug, Clone)]
pub struct ResourceRequest {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub storage_gb: u64,
    pub gpu_units: u32,
    pub network_bandwidth_mbps: u32,
    pub duration: Duration,
    pub priority: Priority,
}
```

### Execution Monitor
```rust
pub struct ExecutionMonitor {
    active_executions: Arc<RwLock<HashMap<String, ExecutionSession>>>,
    metrics_collector: Arc<MetricsCollector>,
    event_broadcaster: Arc<EventBroadcaster>,
    analytics_engine: Arc<AnalyticsEngine>,
}

impl ExecutionMonitor {
    pub async fn start_monitoring(&self, execution_id: &str) -> Result<MonitorHandle>;
    pub async fn stop_monitoring(&self, execution_id: &str) -> Result<()>;
    pub async fn get_execution_metrics(&self, execution_id: &str) -> Result<ExecutionMetrics>;
    pub async fn stream_execution_events(&self, execution_id: &str) -> Result<EventStream>;
}

#[derive(Debug, Clone)]
pub struct ExecutionSession {
    pub execution_id: String,
    pub workload: Workload,
    pub runtime_type: RuntimeType,
    pub resource_allocation: ResourceAllocation,
    pub status: ExecutionStatus,
    pub metrics: ExecutionMetrics,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Recursive Hosting System
```rust
pub struct RecursiveHostingSystem {
    host_manager: Arc<HostManager>,
    nesting_controller: Arc<NestingController>,
    resource_partitioner: Arc<ResourcePartitioner>,
    communication_bridge: Arc<CommunicationBridge>,
}

impl RecursiveHostingSystem {
    pub async fn create_nested_instance(&self, config: NestedInstanceConfig) -> Result<NestedInstance>;
    pub async fn manage_nesting_level(&self, instance_id: &str, level: u32) -> Result<NestingResult>;
    pub async fn bridge_communication(&self, source: &str, target: &str) -> Result<CommunicationChannel>;
    pub async fn partition_resources(&self, parent_allocation: &ResourceAllocation, child_requests: Vec<ResourceRequest>) -> Result<Vec<ResourceAllocation>>;
}

#[derive(Debug, Clone)]
pub struct NestedInstance {
    pub instance_id: String,
    pub parent_id: Option<String>,
    pub nesting_level: u32,
    pub resource_allocation: ResourceAllocation,
    pub configuration: NestedInstanceConfig,
    pub status: InstanceStatus,
}
```

### Sandbox Manager
```rust
pub struct SandboxManager {
    sandbox_registry: Arc<SandboxRegistry>,
    security_policy_engine: Arc<SecurityPolicyEngine>,
    isolation_controller: Arc<IsolationController>,
    capability_manager: Arc<CapabilityManager>,
}

impl SandboxManager {
    pub async fn create_sandbox(&self, spec: SandboxSpec) -> Result<Sandbox>;
    pub async fn execute_in_sandbox(&self, sandbox_id: &str, workload: Workload) -> Result<ExecutionResult>;
    pub async fn monitor_sandbox_security(&self, sandbox_id: &str) -> Result<SecurityStatus>;
    pub async fn destroy_sandbox(&self, sandbox_id: &str) -> Result<()>;
}
```

## Implementation Tasks

### Phase 1: Core Compute Infrastructure
1. **Universal Runtime Framework**
   - Implement multi-runtime execution engine
   - Create unified workload abstraction
   - Build runtime capability matching
   - Enable dynamic runtime selection

2. **Resource Management**
   - Implement resource allocation system
   - Create quota management
   - Build load balancing algorithms
   - Enable resource optimization

### Phase 2: Advanced Execution Features
1. **Streaming Execution**
   - Implement real-time execution monitoring
   - Create execution event streaming
   - Build progress tracking
   - Enable execution analytics

2. **Recursive Hosting**
   - Implement nested ToadStool hosting
   - Create resource partitioning
   - Build communication bridging
   - Enable nesting level management

### Phase 3: Security and Sandboxing
1. **Sandbox System**
   - Implement secure execution sandboxing
   - Create security policy enforcement
   - Build isolation mechanisms
   - Enable capability management

2. **Security Integration**
   - Implement secure workload execution
   - Create encrypted communication
   - Build access control
   - Enable audit logging

## Workload Specifications

### Workload Definition
```rust
#[derive(Debug, Clone)]
pub struct Workload {
    pub id: String,
    pub name: String,
    pub workload_type: WorkloadType,
    pub runtime_requirements: RuntimeRequirements,
    pub resource_requirements: ResourceRequest,
    pub execution_context: ExecutionContext,
    pub security_context: SecurityContext,
}

#[derive(Debug, Clone)]
pub enum WorkloadType {
    Container { image: String, command: Vec<String> },
    WebAssembly { module: Vec<u8>, function: String },
    Native { binary: Vec<u8>, arguments: Vec<String> },
    GPU { kernel: Vec<u8>, parameters: HashMap<String, String> },
    Recursive { nested_config: NestedInstanceConfig },
}

#[derive(Debug, Clone)]
pub struct RuntimeRequirements {
    pub preferred_runtime: Option<RuntimeType>,
    pub required_capabilities: Vec<RuntimeCapability>,
    pub performance_hints: PerformanceHints,
    pub compatibility_constraints: Vec<CompatibilityConstraint>,
}
```

### Execution Context
```rust
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub environment_variables: HashMap<String, String>,
    pub working_directory: Option<String>,
    pub input_data: Option<Vec<u8>>,
    pub output_configuration: OutputConfiguration,
    pub timeout: Option<Duration>,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub isolation_level: IsolationLevel,
    pub network_policy: NetworkPolicy,
    pub filesystem_policy: FilesystemPolicy,
    pub capability_set: CapabilitySet,
}
```

### gRPC Compute Services
```rust
// Compute Service
service ComputeService {
    rpc ExecuteWorkload(ExecuteRequest) returns (stream ExecutionResponse);
    rpc MonitorExecution(MonitorRequest) returns (stream ExecutionEvent);
    rpc AllocateResources(ResourceRequest) returns (ResourceResponse);
    rpc CreateSandbox(SandboxRequest) returns (SandboxResponse);
    rpc HostRecursive(RecursiveRequest) returns (stream RecursiveResponse);
}

// Resource Management Service
service ResourceService {
    rpc AllocateResources(AllocationRequest) returns (AllocationResponse);
    rpc DeallocateResources(DeallocationRequest) returns (DeallocationResponse);
    rpc MonitorResources(MonitorRequest) returns (stream ResourceMetrics);
    rpc OptimizeResources(OptimizationRequest) returns (OptimizationResponse);
}
```

## Configuration

### Compute Engine Configuration
```rust
pub struct ComputeEngineConfig {
    pub runtime_config: RuntimeConfig,
    pub resource_config: ResourceConfig,
    pub monitoring_config: MonitoringConfig,
    pub security_config: ComputeSecurityConfig,
}

pub struct RuntimeConfig {
    pub enabled_runtimes: Vec<RuntimeType>,
    pub runtime_pools: HashMap<RuntimeType, PoolConfig>,
    pub default_timeout: Duration,
    pub max_concurrent_executions: usize,
}

pub struct ResourceConfig {
    pub total_cpu_cores: u32,
    pub total_memory_mb: u64,
    pub total_storage_gb: u64,
    pub total_gpu_units: u32,
    pub allocation_strategy: AllocationStrategy,
}
```

### Sandbox Configuration
```rust
pub struct SandboxConfig {
    pub default_isolation_level: IsolationLevel,
    pub network_isolation: bool,
    pub filesystem_isolation: bool,
    pub capability_filtering: bool,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone)]
pub enum IsolationLevel {
    None,
    Process,
    Container,
    VirtualMachine,
}
```

## Integration Points

### Primal Integration
- **Songbird**: Register compute services and coordinate execution
- **Squirrel**: Execute AI workloads and plugin processing
- **NestGate**: Access storage for compute workloads
- **BearDog**: Secure execution environments and encrypted communication
- **BiomeOS**: Provide compute services to universal UI

### Event Integration
- Broadcast execution events to ecosystem
- Subscribe to resource allocation events
- Handle workload coordination requests
- Coordinate distributed compute tasks

## Performance Requirements

### Latency Targets
- Workload startup: < 1s
- Resource allocation: < 200ms
- Execution monitoring: < 50ms
- Sandbox creation: < 500ms

### Throughput Targets
- Concurrent executions: 1K executions
- Workload throughput: 10K workloads/hour
- Resource operations: 5K operations/second
- Monitoring events: 50K events/second

## Security Considerations

### Execution Security
- Implement secure sandboxing
- Use capability-based security
- Monitor execution behavior
- Prevent privilege escalation

### Resource Security
- Implement resource isolation
- Use secure resource allocation
- Monitor resource usage
- Prevent resource exhaustion

### Communication Security
- Encrypt all execution communication
- Use authenticated channels
- Implement audit logging
- Support secure bridging

## Testing Strategy

### Unit Testing
- Runtime implementations
- Resource allocation logic
- Execution monitoring
- Sandbox security

### Integration Testing
- Cross-runtime compatibility
- Resource coordination
- Execution workflows
- Security validation

### Performance Testing
- Execution latency
- Resource utilization
- Concurrent execution
- Monitoring overhead

## Examples

### Workload Execution
```rust
let compute_engine = UniversalComputeEngine::new(config).await?;

let workload = Workload {
    id: "task-001".to_string(),
    name: "Data Processing".to_string(),
    workload_type: WorkloadType::Container {
        image: "data-processor:latest".to_string(),
        command: vec!["process".to_string(), "--input".to_string(), "/data".to_string()],
    },
    runtime_requirements: RuntimeRequirements {
        preferred_runtime: Some(RuntimeType::Container),
        required_capabilities: vec![RuntimeCapability::Sandboxing],
        performance_hints: PerformanceHints::default(),
        compatibility_constraints: vec![],
    },
    resource_requirements: ResourceRequest {
        cpu_cores: 2,
        memory_mb: 1024,
        storage_gb: 10,
        gpu_units: 0,
        network_bandwidth_mbps: 100,
        duration: Duration::from_secs(3600),
        priority: Priority::Medium,
    },
    execution_context: ExecutionContext::default(),
    security_context: SecurityContext::default(),
};

let execution_result = compute_engine.execute_workload(workload).await?;
```

### Recursive Hosting
```rust
let nested_config = NestedInstanceConfig {
    parent_instance_id: Some("parent-001".to_string()),
    nesting_level: 1,
    resource_allocation: ResourceAllocation {
        cpu_cores: 1,
        memory_mb: 512,
        storage_gb: 5,
        gpu_units: 0,
        network_bandwidth_mbps: 50,
    },
    configuration: InstanceConfiguration::default(),
};

let nested_instance = recursive_hosting_system.create_nested_instance(nested_config).await?;
```

### Sandbox Execution
```rust
let sandbox_spec = SandboxSpec {
    isolation_level: IsolationLevel::Container,
    network_policy: NetworkPolicy::Isolated,
    filesystem_policy: FilesystemPolicy::ReadOnly,
    capability_set: CapabilitySet::minimal(),
    resource_limits: ResourceLimits::default(),
};

let sandbox = sandbox_manager.create_sandbox(sandbox_spec).await?;
let result = sandbox_manager.execute_in_sandbox(&sandbox.id, workload).await?;
```

## Best Practices

1. **Resource Optimization**
   - Use efficient resource allocation
   - Implement resource pooling
   - Enable dynamic scaling
   - Support resource sharing

2. **Security First**
   - Implement secure sandboxing
   - Use least privilege principles
   - Enable execution monitoring
   - Support audit logging

3. **Performance Optimization**
   - Use asynchronous execution
   - Implement efficient scheduling
   - Enable parallel processing
   - Support caching strategies

4. **Monitoring and Observability**
   - Implement comprehensive monitoring
   - Use real-time metrics
   - Enable performance analytics
   - Support debugging tools

## Version History

- v1.0.0: Initial enhanced compute specification
- v1.1.0: Added recursive hosting support
- v1.2.0: Enhanced sandbox security
- v1.3.0: Multi-runtime optimization

<version>1.3.0</version> 