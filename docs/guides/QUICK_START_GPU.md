# 🚀 ToadStool GPU Quick Start Guide

Get GPU compute running in 5 minutes!

---

## Prerequisites

### Hardware
- **GPU**: NVIDIA, AMD, or Intel with OpenCL support
- **Drivers**: Latest GPU drivers installed
- **OS**: Linux, macOS, or Windows with OpenCL ICD loader

### Software
- Rust 1.70+ (`rustup update`)
- OpenCL drivers for your GPU

---

## Installation

### 1. Clone ToadStool
```bash
git clone <repo-url>
cd toadstool
```

### 2. Build with GPU Support
```bash
cargo build --release --features toadstool-runtime-gpu/opencl
```

**Expected output**: Clean compilation, no warnings

---

## Running Your First GPU Workload

### Single GPU Demo
```bash
cargo run --release --bin opencl_gpu_demo \
  --features toadstool-runtime-gpu/opencl
```

**What you'll see**:
```
🎮 ToadStool OpenCL GPU Demo
================================

🔍 Discovering OpenCL devices...
✅ OpenCL device initialized!

📊 GPU Capabilities:
   Type: OpenCL GPU: <Your GPU>
   Parallel Threads: <N>
   Memory: <X> GB
   FP64 Support: true/false
   Peak FLOPS: <Y> GFLOPS

🚀 Workload 1: General Compute
   ✅ Done in <X>µs

🚀 Workload 2: Parallel Reduction
   ✅ Done in <Y>µs
```

### Multi-Tower Federation Demo
```bash
cargo run --release --bin distributed_gpu_demo
```

**What you'll see**:
- Local tower initialized
- Remote tower registration simulation
- Federation statistics
- Integration status

---

## Verification Checklist

Run these commands to verify everything works:

### 1. Build Check
```bash
cargo build --release -p toadstool-runtime-gpu --features opencl
# Expected: Clean build, 0 errors
```

### 2. Lint Check
```bash
cargo clippy -p toadstool-runtime-gpu --features opencl -- -D warnings
# Expected: Finished, 0 warnings
```

### 3. Test Check
```bash
cargo test -p toadstool-runtime-gpu --lib --features opencl
# Expected: All tests pass
```

### 4. Demo Check
```bash
cargo run --release --bin opencl_gpu_demo --features toadstool-runtime-gpu/opencl
# Expected: GPU discovered, workloads executed successfully
```

---

## Using in Your Code

### Basic GPU Execution

```rust
use toadstool_runtime_gpu::{
    backends::OpenClComputeResource,
    universal::{
        ComputeBuffer, ComputeRequirements, Operation,
        UniversalComputeResource, UniversalKernel, UniversalWorkload,
    },
    types::DataType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize GPU
    let gpu = OpenClComputeResource::new()?;
    
    // 2. Create context
    let mut context = gpu.create_context().await?;
    
    // 3. Define workload
    let workload = UniversalWorkload {
        id: uuid::Uuid::new_v4().to_string(),
        requirements: ComputeRequirements {
            min_parallel_threads: 1024,
            memory_bytes: 2048,
            precision: Precision::Int8,
            operations: vec![Operation::GeneralCompute],
            max_execution_time: Some(Duration::from_secs(5)),
            preferred_access_pattern: Some(MemoryAccessPattern::Sequential),
        },
        kernel: UniversalKernel::Operation {
            operation: Operation::GeneralCompute,
            parameters: HashMap::new(),
        },
        inputs: vec![ComputeBuffer {
            name: "input".to_string(),
            data: vec![0u8; 1024],
            element_type: DataType::UInt8,
        }],
        output_size: 1024,
        hints: OptimizationHints::default(),
    };
    
    // 4. Execute
    let result = context.execute(&workload).await?;
    
    println!("Execution time: {:?}", result.metrics.execution_time);
    println!("Memory used: {} bytes", result.metrics.memory_used);
    
    Ok(())
}
```

### Multi-Tower Federation

```rust
use toadstool_runtime_gpu::{
    distributed_scheduler::{DistributedGpuScheduler, RemoteTowerEndpoint},
    scheduler::{SchedulingPolicy, UniversalComputeScheduler},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create local scheduler
    let local = Arc::new(UniversalComputeScheduler::new(
        SchedulingPolicy::CapabilityMatch
    ));
    
    // 2. Create distributed scheduler
    let distributed = DistributedGpuScheduler::new(local);
    
    // 3. Register remote towers (via Songbird discovery)
    let remote = RemoteTowerEndpoint {
        tower_id: "tower-1".to_string(),
        address: "10.0.0.2:8080".to_string(),
        gpu_capabilities: None,
        last_seen: std::time::Instant::now(),
        latency_ms: 5,
    };
    distributed.register_remote_tower(remote).await;
    
    // 4. Execute with strategy
    let result = distributed.execute_distributed(
        workload,
        PartitionStrategy::Single
    ).await?;
    
    Ok(())
}
```

---

## Troubleshooting

### "No OpenCL platforms found"
**Solution**: Install OpenCL drivers for your GPU
- NVIDIA: `sudo apt install nvidia-opencl-dev` (or latest driver)
- AMD: `sudo apt install mesa-opencl-icd`
- Intel: `sudo apt install intel-opencl-icd`

### "No OpenCL devices found"
**Solution**: 
1. Check GPU is detected: `lspci | grep VGA`
2. Check drivers loaded: `nvidia-smi` or `clinfo`
3. Try running with `sudo` to test permissions

### Compilation errors with OpenCL
**Solution**: Install OpenCL headers
```bash
# Ubuntu/Debian
sudo apt install ocl-icd-opencl-dev

# Fedora/RHEL
sudo dnf install ocl-icd-devel

# macOS
# OpenCL included in system
```

### Slow first execution
**Expected**: First kernel compilation takes ~100ms
**Solution**: Program caching makes subsequent runs <1ms

---

## Performance Tips

### 1. Enable Release Mode
Always use `--release` for production workloads:
```bash
cargo run --release --bin <your-app>
```

### 2. Batch Workloads
Group similar workloads to benefit from program caching:
```rust
for workload in workloads {
    context.execute(&workload).await?;
    // Subsequent executions use cached programs
}
```

### 3. Use Memory Pool
Buffer reuse reduces allocation overhead:
```rust
let pool = MemoryPool::new();
let buffer = pool.acquire_buffer(size, &queue).await?;
// ... use buffer ...
pool.release_buffer(buffer).await;
```

### 4. Monitor Performance
```rust
let stats = scheduler.statistics().await;
println!("Cache hit rate: {:.1}%", stats.cache_hits as f64 / 
    (stats.cache_hits + stats.cache_misses) as f64 * 100.0);
```

---

## Next Steps

### Enable Federation
1. Set up second tower with ToadStool + GPU
2. Ensure both on same LAN
3. Enable Songbird discovery
4. Register remote towers
5. Execute distributed workloads

### Integrate Ecosystem
- **BearDog**: Add cryptographic receipts to results
- **Songbird**: Advertise GPU capabilities on network
- **NestGate**: Persist large computation results
- **NestGate**: Enable AI-driven scheduling

### Advanced Features
- Multi-GPU per tower
- Custom OpenCL kernels
- Advanced partitioning strategies
- Performance profiling

---

## Documentation

- **Implementation**: `COMPREHENSIVE_COMPLETION_REPORT_DEC_18_2025.md`
- **Hardware Validation**: `EXECUTION_SUCCESS_RTX_2070_SUPER.md`
- **Architecture**: `crates/runtime/gpu/GPU_EVOLUTION_STRATEGY.md`
- **Examples**: `examples/opencl_gpu_demo.rs`, `examples/distributed_gpu_demo.rs`

---

## Support

### File Size Compliance
✅ All files < 1000 lines

### Code Quality
✅ Zero clippy warnings
✅ All tests passing
✅ No production mocks
✅ Zero hardcoding

### Hardware Tested
✅ NVIDIA GeForce RTX 2070 SUPER
- Other NVIDIA GPUs should work
- AMD GPUs supported via OpenCL
- Intel GPUs supported via OpenCL

---

**You're ready to run GPU workloads on ToadStool! 🚀**

For questions or issues, see the comprehensive documentation in the repository root.

