# ToadStool Core Implementation Specification

**Status**: Implementation Required  
**Version**: 1.0  
**Date**: December 18, 2025  
**Priority**: P0 - Core Functionality

---

## Executive Summary

ToadStool's universal compute abstraction is architecturally sound, but **core GPU execution is not implemented**. This spec defines the minimal viable implementation needed for ToadStool to function as a standalone compute platform.

### Current State
- ✅ Universal abstraction layer complete
- ✅ Resource pool architecture defined
- ✅ Scheduling logic implemented
- ✅ CPU execution working (Rayon)
- ❌ **GPU execution missing**
- ❌ **Memory management missing**
- ❌ **Result aggregation missing**

### Goal
Enable ToadStool to execute real GPU workloads on actual hardware, independent of other primals, with federation support.

---

## Part 1: Real GPU Execution (P0)

### 1.1 OpenCL Backend Implementation

**File**: `crates/runtime/gpu/src/backends/opencl_impl.rs`

**Requirements**:
```rust
pub struct OpenClBackend {
    context: ocl::Context,
    queue: ocl::Queue,
    program_cache: HashMap<String, ocl::Program>,
}

impl OpenClBackend {
    /// Initialize OpenCL on available device
    pub fn new() -> Result<Self>;
    
    /// Compile kernel source to executable
    pub async fn compile_kernel(
        &mut self,
        source: &str,
        entry_point: &str,
    ) -> Result<OpenClKernel>;
    
    /// Execute kernel with buffers
    pub async fn execute(
        &self,
        kernel: &OpenClKernel,
        buffers: &[GpuBuffer],
        global_work_size: [usize; 3],
        local_work_size: Option<[usize; 3]>,
    ) -> Result<ExecutionResult>;
}
```

**Implementation Steps**:
1. Use `ocl` crate to create context for RTX 2070 SUPER
2. Compile OpenCL C kernels to device programs
3. Launch kernels with proper work group sizing
4. Wait for completion, read back results
5. Handle OpenCL errors gracefully

**Test Case**:
```rust
#[test]
async fn test_opencl_matrix_multiply() {
    let backend = OpenClBackend::new().unwrap();
    let kernel = backend.compile_kernel(MATRIX_MULTIPLY_CL, "matmul").await.unwrap();
    
    let a = vec![1.0f32; 1024 * 1024];
    let b = vec![2.0f32; 1024 * 1024];
    
    let result = backend.execute(&kernel, &[a.into(), b.into()], [1024, 1024, 1], None).await.unwrap();
    
    assert!(result.execution_time < Duration::from_millis(50));
}
```

### 1.2 CUDA Backend Implementation (Optional for RTX 2070)

**File**: `crates/runtime/gpu/src/backends/cuda_impl.rs`

**Requirements**:
```rust
pub struct CudaBackend {
    device: cudarc::driver::CudaDevice,
    module_cache: HashMap<String, cudarc::driver::CudaModule>,
}

impl CudaBackend {
    pub fn new(device_id: usize) -> Result<Self>;
    
    pub async fn compile_ptx(
        &mut self,
        ptx: &str,
        function_name: &str,
    ) -> Result<CudaKernel>;
    
    pub async fn execute(
        &self,
        kernel: &CudaKernel,
        buffers: &[GpuBuffer],
        grid_dim: (u32, u32, u32),
        block_dim: (u32, u32, u32),
    ) -> Result<ExecutionResult>;
}
```

### 1.3 GPU Auto-Detection

**File**: `crates/runtime/gpu/src/detection.rs`

**Requirements**:
```rust
pub struct GpuInfo {
    pub id: DeviceId,
    pub name: String,
    pub vendor: GpuVendor,
    pub memory_bytes: u64,
    pub compute_capability: Option<(u32, u32)>,
    pub frameworks: Vec<GpuFramework>,
}

pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Unknown,
}

pub fn detect_gpus() -> Result<Vec<GpuInfo>> {
    // 1. Try CUDA detection (nvidia-smi, cudarc)
    // 2. Try OpenCL detection (ocl platform query)
    // 3. Try Vulkan detection (ash/vulkano)
    // 4. Return all discovered GPUs with capabilities
}
```

**Expected Output**:
```
Detected GPUs:
  [0] NVIDIA GeForce RTX 2070 SUPER
      - Memory: 8192 MiB
      - Compute: 7.5
      - Frameworks: CUDA, OpenCL, Vulkan
```

---

## Part 2: GPU Memory Management (P0)

### 2.1 Device Memory Allocator

**File**: `crates/runtime/gpu/src/memory/allocator.rs`

**Requirements**:
```rust
pub struct GpuMemoryAllocator {
    device: DeviceHandle,
    allocations: HashMap<BufferId, Allocation>,
    memory_pool: MemoryPool,
    total_bytes: u64,
    used_bytes: AtomicU64,
}

impl GpuMemoryAllocator {
    /// Allocate device memory
    pub fn allocate(&mut self, size: usize) -> Result<DeviceBuffer>;
    
    /// Free device memory
    pub fn free(&mut self, buffer: DeviceBuffer) -> Result<()>;
    
    /// Copy host → device
    pub async fn upload(
        &self,
        host_data: &[u8],
        device_buffer: &DeviceBuffer,
    ) -> Result<()>;
    
    /// Copy device → host
    pub async fn download(
        &self,
        device_buffer: &DeviceBuffer,
        host_data: &mut [u8],
    ) -> Result<()>;
    
    /// Get memory usage stats
    pub fn usage(&self) -> MemoryUsage;
}

pub struct MemoryUsage {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub fragmentation: f32,
}
```

### 2.2 Memory Pool for Reuse

**File**: `crates/runtime/gpu/src/memory/pool.rs`

**Requirements**:
```rust
pub struct MemoryPool {
    free_blocks: BTreeMap<usize, Vec<DeviceBuffer>>,
    allocated_blocks: HashMap<BufferId, BufferMetadata>,
}

impl MemoryPool {
    /// Get buffer from pool or allocate new
    pub fn acquire(&mut self, size: usize) -> Result<DeviceBuffer>;
    
    /// Return buffer to pool for reuse
    pub fn release(&mut self, buffer: DeviceBuffer);
    
    /// Clear unused buffers
    pub fn gc(&mut self);
}
```

**Benefits**:
- Reduce allocation overhead
- Minimize memory fragmentation
- Faster workload execution

### 2.3 Pinned Host Memory

**File**: `crates/runtime/gpu/src/memory/pinned.rs`

**Requirements**:
```rust
pub struct PinnedMemory {
    ptr: *mut u8,
    size: usize,
}

impl PinnedMemory {
    /// Allocate page-locked memory for fast transfers
    pub fn new(size: usize) -> Result<Self>;
    
    /// Get as slice
    pub fn as_slice(&self) -> &[u8];
    
    /// Get as mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [u8];
}
```

**Why**: 2-3x faster host↔device transfers with pinned memory

---

## Part 3: Result Aggregation (P0)

### 3.1 Aggregation Engine

**File**: `crates/runtime/gpu/src/aggregation/engine.rs`

**Requirements**:
```rust
pub struct ResultAggregator {
    strategy: AggregationStrategy,
}

pub enum AggregationStrategy {
    Concatenate,        // Simple concat (vector ops)
    MatrixMerge,        // Combine matrix chunks
    Reduction,          // Sum/min/max across results
    Custom(Box<dyn AggregationFn>),
}

impl ResultAggregator {
    pub async fn aggregate(
        &self,
        partial_results: Vec<PartialResult>,
    ) -> Result<FinalResult> {
        match self.strategy {
            AggregationStrategy::MatrixMerge => {
                self.merge_matrix_chunks(partial_results).await
            }
            AggregationStrategy::Reduction => {
                self.reduce_results(partial_results).await
            }
            // ...
        }
    }
}
```

### 3.2 Matrix Chunk Merging

**File**: `crates/runtime/gpu/src/aggregation/matrix.rs`

**Requirements**:
```rust
pub struct MatrixChunk {
    pub row_start: usize,
    pub row_end: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub data: Vec<f32>,
}

pub fn merge_matrix_chunks(
    chunks: Vec<MatrixChunk>,
    total_rows: usize,
    total_cols: usize,
) -> Result<Matrix> {
    // 1. Validate chunks cover full matrix
    // 2. Sort by position
    // 3. Combine into final matrix
    // 4. Handle overlaps (if any)
}
```

### 3.3 Partial Failure Handling

**File**: `crates/runtime/gpu/src/aggregation/fault_tolerance.rs`

**Requirements**:
```rust
pub enum PartialResultStatus {
    Success(PartialResult),
    Failed(TowerId, Error),
    Timeout(TowerId),
}

pub struct PartialResultSet {
    expected_count: usize,
    received: Vec<PartialResultStatus>,
}

impl PartialResultSet {
    /// Check if we can proceed with partial results
    pub fn is_sufficient(&self) -> bool;
    
    /// Get successful results
    pub fn successful_results(&self) -> Vec<&PartialResult>;
    
    /// Decide: aggregate partial or retry failed
    pub fn recovery_strategy(&self) -> RecoveryStrategy;
}

pub enum RecoveryStrategy {
    AggregatePartial,       // Continue with what we have
    RetryFailed,            // Retry failed towers
    Failover(Vec<TowerId>), // Re-assign work to other towers
    Abort,                  // Job cannot complete
}
```

---

## Part 4: Workload Partitioning (P1)

### 4.1 Data Partitioning

**File**: `crates/runtime/gpu/src/partitioning/data.rs`

**Requirements**:
```rust
pub struct WorkloadPartitioner {
    strategy: PartitionStrategy,
}

pub enum PartitionStrategy {
    EqualSplit,           // Divide equally
    CapabilityWeighted,   // Weight by resource capability
    MemoryConstrained,    // Partition to fit memory
    NetworkAware,         // Minimize data transfer
}

impl WorkloadPartitioner {
    pub fn partition(
        &self,
        workload: &UniversalWorkload,
        resources: &[ResourceInfo],
    ) -> Result<Vec<PartialWorkload>> {
        // Calculate optimal partitions based on:
        // 1. Resource capabilities
        // 2. Memory constraints
        // 3. Network bandwidth
        // 4. Data locality
    }
}
```

### 4.2 Matrix Partitioning

**File**: `crates/runtime/gpu/src/partitioning/matrix.rs`

**Requirements**:
```rust
pub fn partition_matrix_multiply(
    a_rows: usize,
    a_cols: usize,
    b_cols: usize,
    num_partitions: usize,
    memory_limit: Option<usize>,
) -> Vec<MatrixPartition> {
    // Split by rows, columns, or blocks
    // Minimize communication overhead
    // Balance work across resources
}

pub struct MatrixPartition {
    pub partition_id: usize,
    pub a_rows: Range<usize>,
    pub b_cols: Range<usize>,
    pub expected_size: usize,
}
```

---

## Part 5: Fault Tolerance (P1)

### 5.1 Retry Logic

**File**: `crates/runtime/gpu/src/fault_tolerance/retry.rs`

**Requirements**:
```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub timeout: Duration,
}

pub enum BackoffStrategy {
    Fixed(Duration),
    Exponential { initial: Duration, multiplier: f64 },
    Linear { initial: Duration, increment: Duration },
}

pub async fn execute_with_retry<F, T>(
    operation: F,
    policy: &RetryPolicy,
) -> Result<T>
where
    F: Fn() -> Future<Output = Result<T>>,
{
    let mut attempts = 0;
    loop {
        match timeout(policy.timeout, operation()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) | Err(_) if attempts < policy.max_attempts => {
                attempts += 1;
                let delay = policy.backoff.delay(attempts);
                tokio::time::sleep(delay).await;
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(Error::Timeout),
        }
    }
}
```

### 5.2 GPU Failover to CPU

**File**: `crates/runtime/gpu/src/fault_tolerance/failover.rs`

**Requirements**:
```rust
pub struct FailoverManager {
    cpu_fallback: Arc<CpuComputeResource>,
}

impl FailoverManager {
    pub async fn execute_with_failover(
        &self,
        workload: &UniversalWorkload,
        primary_resource: &dyn UniversalComputeResource,
    ) -> Result<WorkloadResult> {
        match primary_resource.execute(workload).await {
            Ok(result) => Ok(result),
            Err(e) if e.is_recoverable() => {
                tracing::warn!("GPU execution failed, falling back to CPU: {}", e);
                self.cpu_fallback.execute(workload).await
            }
            Err(e) => Err(e),
        }
    }
}
```

### 5.3 Tower Failure Handling

**File**: `crates/runtime/gpu/src/fault_tolerance/tower.rs`

**Requirements**:
```rust
pub struct TowerHealthMonitor {
    health_checks: HashMap<TowerId, HealthStatus>,
}

pub struct HealthStatus {
    pub last_successful: Instant,
    pub consecutive_failures: u32,
    pub available: bool,
}

impl TowerHealthMonitor {
    /// Mark tower as failed
    pub fn record_failure(&mut self, tower_id: &TowerId);
    
    /// Mark tower as healthy
    pub fn record_success(&mut self, tower_id: &TowerId);
    
    /// Check if tower should be used
    pub fn is_available(&self, tower_id: &TowerId) -> bool;
    
    /// Remove unreliable towers from pool
    pub fn prune_unhealthy(&mut self, threshold: u32);
}
```

---

## Part 6: Performance & Observability (P2)

### 6.1 Execution Metrics

**File**: `crates/runtime/gpu/src/metrics/collector.rs`

**Requirements**:
```rust
pub struct ExecutionMetrics {
    // Timing
    pub total_duration: Duration,
    pub kernel_duration: Duration,
    pub transfer_duration: Duration,
    pub overhead_duration: Duration,
    
    // GPU
    pub gpu_utilization: f32,        // 0.0 - 1.0
    pub memory_bandwidth_gbps: f64,
    pub compute_throughput_gflops: f64,
    
    // Transfer
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
    pub upload_bandwidth_gbps: f64,
    pub download_bandwidth_gbps: f64,
    
    // Efficiency
    pub compute_efficiency: f32,     // compute time / total time
    pub memory_efficiency: f32,      // effective bandwidth / peak bandwidth
}

impl ExecutionMetrics {
    pub fn collect_during<F, T>(operation: F) -> (T, Self)
    where
        F: FnOnce() -> T;
}
```

### 6.2 Performance Profiling

**File**: `crates/runtime/gpu/src/metrics/profiler.rs`

**Requirements**:
```rust
pub struct GpuProfiler {
    events: Vec<ProfileEvent>,
}

pub struct ProfileEvent {
    pub name: String,
    pub start: Instant,
    pub duration: Duration,
    pub category: EventCategory,
}

pub enum EventCategory {
    KernelExecution,
    MemoryTransfer,
    Allocation,
    Compilation,
}

impl GpuProfiler {
    pub fn start_event(&mut self, name: impl Into<String>);
    pub fn end_event(&mut self);
    pub fn report(&self) -> PerformanceReport;
}
```

---

## Part 7: Ecosystem Integration

### 7.1 BearDog Integration for Receipts

**File**: `crates/runtime/gpu/src/integration/beardog.rs`

**Requirements**:
```rust
pub struct BearDogReceiptSigner {
    beardog_client: BearDogClient,
}

impl BearDogReceiptSigner {
    pub async fn sign_receipt(
        &self,
        receipt: &Receipt,
    ) -> Result<SignedReceipt> {
        // 1. Serialize receipt
        // 2. Send to BearDog for signing
        // 3. Return receipt with signature
    }
    
    pub async fn verify_receipt(
        &self,
        signed_receipt: &SignedReceipt,
    ) -> Result<bool> {
        // 1. Extract signature
        // 2. Verify with BearDog
        // 3. Return valid/invalid
    }
}
```

### 7.2 Songbird Integration for Discovery

**File**: `crates/runtime/gpu/src/integration/songbird.rs`

**Requirements**:
```rust
pub struct SongbirdDiscovery {
    songbird_client: SongbirdClient,
}

impl SongbirdDiscovery {
    pub async fn discover_towers(&self) -> Result<Vec<TowerInfo>> {
        // 1. Query Songbird for available towers
        // 2. Get capability information
        // 3. Return tower list
    }
    
    pub async fn announce_capabilities(
        &self,
        capabilities: &ComputeCapabilities,
    ) -> Result<()> {
        // 1. Package capability data
        // 2. Announce via Songbird
        // 3. Handle announcement lifecycle
    }
}
```

### 7.3 NestGate Integration for Data

**File**: `crates/runtime/gpu/src/integration/nestgate.rs`

**Requirements**:
```rust
pub struct NestGateDataSource {
    nestgate_client: NestGateClient,
}

impl NestGateDataSource {
    pub async fn load_input_data(
        &self,
        data_ref: &DataReference,
    ) -> Result<Vec<u8>> {
        // 1. Request data from NestGate
        // 2. Receive data stream
        // 3. Return as buffer for compute
    }
    
    pub async fn save_output_data(
        &self,
        data: &[u8],
        metadata: &DataMetadata,
    ) -> Result<DataReference> {
        // 1. Stream data to NestGate
        // 2. Get reference/ID
        // 3. Return reference
    }
}
```

---

## Implementation Priority

### Phase 1: Core GPU Execution (Week 1)
1. OpenCL backend implementation
2. GPU auto-detection
3. Basic memory management
4. Single-GPU workload execution

**Goal**: Execute real GPU kernels on RTX 2070 SUPER

### Phase 2: Memory & Performance (Week 2)
1. Memory pool implementation
2. Pinned memory for fast transfers
3. Performance metrics collection
4. Basic profiling

**Goal**: Efficient GPU memory usage and performance tracking

### Phase 3: Federation (Week 3)
1. Workload partitioning
2. Result aggregation
3. Fault tolerance & retry
4. Multi-tower execution

**Goal**: Real distributed compute across towers

### Phase 4: Ecosystem Integration (Week 4)
1. BearDog receipt signing
2. Songbird tower discovery
3. NestGate data integration
4. End-to-end workflow

**Goal**: Full ecosystem integration

---

## Success Criteria

### Minimal Viable (Phase 1 Complete):
- ✅ Execute OpenCL kernel on RTX 2070 SUPER
- ✅ Matrix multiply 2048x2048 in <50ms
- ✅ Allocate and manage GPU memory
- ✅ Transfer data host↔device correctly

### Federation Ready (Phase 3 Complete):
- ✅ Split workload across 2 towers
- ✅ Aggregate results correctly
- ✅ Handle tower failure gracefully
- ✅ Generate execution receipts

### Ecosystem Integrated (Phase 4 Complete):
- ✅ BearDog-signed receipts
- ✅ Songbird tower discovery
- ✅ NestGate data persistence
- ✅ Full Squirrel integration

---

## Testing Strategy

### Unit Tests
- Each backend (OpenCL, CUDA)
- Memory allocator
- Partitioning algorithms
- Aggregation logic

### Integration Tests
- End-to-end GPU execution
- Multi-tower federation
- Ecosystem primal integration

### Performance Tests
- Benchmark against native OpenCL/CUDA
- Measure overhead
- Profile memory usage
- Test scaling (1 → N towers)

### Chaos Tests
- Random tower failures
- Network interruptions
- OOM conditions
- Partial result handling

---

## Notes

### Design Principles
1. **Standalone First**: ToadStool must work without other primals
2. **Ecosystem Enhanced**: Other primals add capabilities, not core functionality
3. **Hardware Agnostic**: Same code works on any GPU/CPU
4. **Fault Tolerant**: Gracefully handle all failure modes
5. **Observable**: Rich metrics for optimization

### Current Architecture is Sound
- Universal abstraction: ✅ Correct
- Resource pool: ✅ Correct
- Scheduling: ✅ Correct
- Traits: ✅ Well-designed

### What's Missing is Implementation
Not design changes - just filling in the GPU execution layer.

---

**Version History**:
- v1.0 (2025-12-18): Initial specification based on architecture review

