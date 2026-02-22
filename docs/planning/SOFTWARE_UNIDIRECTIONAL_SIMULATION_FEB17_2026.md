# Software Unidirectional Pipeline Simulation

**Date**: February 17, 2026  
**Status**: Implementation Design  
**Origin**: Validating unidirectional patterns without extra hardware

---

## The Goal

Simulate the unidirectional compute pipeline **within a single system** using only PCIe, by partitioning bandwidth and enforcing data flow discipline.

```
Physical Unidirectional:
  Computer A ──HDMI──► Computer B
  (no round-trips because physically separate)

Software Simulation:
  Same GPU, but enforce 90% input / 10% output bandwidth discipline
  (no round-trips because we choose not to)
```

---

## Architecture

### PCIe Bandwidth Partitioning

```
┌─────────────────────────────────────────────────────────────────────┐
│                         PCIe x16 (64 GB/s)                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  INPUT PARTITION (90% = 57.6 GB/s)                                  │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  • Work unit uploads (continuous stream)                      │  │
│  │  • Parameter buffers                                          │  │
│  │  • Shader uniforms                                            │  │
│  │  • Fire-and-forget (no waiting for results)                   │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  OUTPUT PARTITION (10% = 6.4 GB/s)                                  │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  • Completed results (batched, async)                         │  │
│  │  • Only when staging buffer fills                             │  │
│  │  • Never blocks input stream                                  │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### GPU Memory Partitioning

```
┌─────────────────────────────────────────────────────────────────────┐
│                      GPU VRAM (e.g., 24 GB)                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  COMPUTE WORKSPACE (80% = 19.2 GB)                                  │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  • Active computation buffers                                 │  │
│  │  • Intermediate results                                       │  │
│  │  • Recycled per work unit                                     │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  INPUT STAGING (10% = 2.4 GB)                                       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  • Ring buffer for incoming work units                        │  │
│  │  • CPU writes here continuously                               │  │
│  │  • GPU reads and computes                                     │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  OUTPUT STAGING (10% = 2.4 GB)                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  • Ring buffer for completed results                          │  │
│  │  • GPU writes here after compute                              │  │
│  │  • CPU reads in batches (async, non-blocking)                 │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Implementation

### Core Types

```rust
/// Configuration for unidirectional pipeline simulation
#[derive(Debug, Clone)]
pub struct UnidirectionalConfig {
    /// Fraction of VRAM for input staging (0.0-1.0)
    pub input_staging_fraction: f32,
    
    /// Fraction of VRAM for output staging (0.0-1.0)
    pub output_staging_fraction: f32,
    
    /// Target input bandwidth utilization (0.0-1.0)
    pub input_bandwidth_target: f32,
    
    /// Target output bandwidth utilization (0.0-1.0)  
    pub output_bandwidth_target: f32,
    
    /// Batch size for output downloads (bytes)
    pub output_batch_size: usize,
    
    /// Enable strict unidirectional mode (panic on sync readback)
    pub strict_mode: bool,
}

impl Default for UnidirectionalConfig {
    fn default() -> Self {
        Self {
            input_staging_fraction: 0.10,
            output_staging_fraction: 0.10,
            input_bandwidth_target: 0.90,
            output_bandwidth_target: 0.10,
            output_batch_size: 64 * 1024 * 1024, // 64 MB batches
            strict_mode: false,
        }
    }
}
```

### Ring Buffer Staging

```rust
/// GPU ring buffer for streaming data
pub struct GpuRingBuffer {
    buffer: wgpu::Buffer,
    capacity: usize,
    write_head: AtomicUsize,  // Where CPU writes next
    read_head: AtomicUsize,   // Where GPU reads next
    direction: BufferDirection,
}

#[derive(Debug, Clone, Copy)]
pub enum BufferDirection {
    /// CPU → GPU (input staging)
    HostToDevice,
    /// GPU → CPU (output staging)
    DeviceToHost,
}

impl GpuRingBuffer {
    /// Create a ring buffer for staging
    pub fn new(device: &wgpu::Device, capacity: usize, direction: BufferDirection) -> Self {
        let usage = match direction {
            BufferDirection::HostToDevice => {
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE
            }
            BufferDirection::DeviceToHost => {
                wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ
            }
        };
        
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ring_buffer"),
            size: capacity as u64,
            usage,
            mapped_at_creation: false,
        });
        
        Self {
            buffer,
            capacity,
            write_head: AtomicUsize::new(0),
            read_head: AtomicUsize::new(0),
            direction,
        }
    }
    
    /// Available space for writing
    pub fn available_write(&self) -> usize {
        let write = self.write_head.load(Ordering::Acquire);
        let read = self.read_head.load(Ordering::Acquire);
        
        if write >= read {
            self.capacity - (write - read) - 1
        } else {
            read - write - 1
        }
    }
    
    /// Available data for reading
    pub fn available_read(&self) -> usize {
        let write = self.write_head.load(Ordering::Acquire);
        let read = self.read_head.load(Ordering::Acquire);
        
        if write >= read {
            write - read
        } else {
            self.capacity - read + write
        }
    }
}
```

### Unidirectional Pipeline

```rust
/// Simulated unidirectional compute pipeline
pub struct UnidirectionalPipeline {
    device: Arc<WgpuDevice>,
    config: UnidirectionalConfig,
    
    // Staging buffers
    input_ring: GpuRingBuffer,
    output_ring: GpuRingBuffer,
    
    // Compute workspace
    workspace: GpuBuffer,
    
    // Metrics
    bytes_uploaded: AtomicU64,
    bytes_downloaded: AtomicU64,
    work_units_submitted: AtomicU64,
    work_units_completed: AtomicU64,
    
    // Async handles
    pending_downloads: Mutex<Vec<PendingDownload>>,
}

struct PendingDownload {
    buffer_slice: wgpu::BufferSlice<'static>,
    callback: Box<dyn FnOnce(Vec<u8>) + Send>,
}

impl UnidirectionalPipeline {
    /// Create a new unidirectional pipeline
    pub async fn new(device: Arc<WgpuDevice>, config: UnidirectionalConfig) -> Result<Self> {
        let vram_size = device.limits().max_buffer_size as usize;
        
        let input_size = (vram_size as f32 * config.input_staging_fraction) as usize;
        let output_size = (vram_size as f32 * config.output_staging_fraction) as usize;
        let workspace_size = vram_size - input_size - output_size;
        
        tracing::info!(
            "Creating unidirectional pipeline: input={}MB, output={}MB, workspace={}MB",
            input_size / 1024 / 1024,
            output_size / 1024 / 1024,
            workspace_size / 1024 / 1024,
        );
        
        let input_ring = GpuRingBuffer::new(
            device.inner(),
            input_size,
            BufferDirection::HostToDevice,
        );
        
        let output_ring = GpuRingBuffer::new(
            device.inner(),
            output_size,
            BufferDirection::DeviceToHost,
        );
        
        let workspace = device.create_buffer(workspace_size)?;
        
        Ok(Self {
            device,
            config,
            input_ring,
            output_ring,
            workspace,
            bytes_uploaded: AtomicU64::new(0),
            bytes_downloaded: AtomicU64::new(0),
            work_units_submitted: AtomicU64::new(0),
            work_units_completed: AtomicU64::new(0),
            pending_downloads: Mutex::new(Vec::new()),
        })
    }
    
    /// Submit work unit (fire and forget - no waiting!)
    pub fn submit_work(&self, data: &[u8]) -> Result<WorkHandle> {
        if self.config.strict_mode {
            // In strict mode, never block on output
            assert!(
                self.input_ring.available_write() >= data.len(),
                "Input ring full - would block (strict mode)"
            );
        }
        
        // Write to input ring buffer
        let offset = self.write_to_input_ring(data)?;
        
        // Dispatch compute (async, returns immediately)
        let handle = self.dispatch_compute(offset, data.len())?;
        
        self.bytes_uploaded.fetch_add(data.len() as u64, Ordering::Relaxed);
        self.work_units_submitted.fetch_add(1, Ordering::Relaxed);
        
        Ok(handle)
    }
    
    /// Poll for completed results (non-blocking)
    pub fn poll_results(&self) -> Vec<CompletedWork> {
        let mut results = Vec::new();
        
        // Check if output ring has enough data for a batch
        while self.output_ring.available_read() >= self.config.output_batch_size {
            if let Some(batch) = self.try_read_output_batch() {
                results.push(batch);
            } else {
                break;
            }
        }
        
        self.work_units_completed.fetch_add(results.len() as u64, Ordering::Relaxed);
        results
    }
    
    /// Get pipeline statistics
    pub fn stats(&self) -> PipelineStats {
        PipelineStats {
            bytes_uploaded: self.bytes_uploaded.load(Ordering::Relaxed),
            bytes_downloaded: self.bytes_downloaded.load(Ordering::Relaxed),
            work_units_submitted: self.work_units_submitted.load(Ordering::Relaxed),
            work_units_completed: self.work_units_completed.load(Ordering::Relaxed),
            input_utilization: self.calculate_input_utilization(),
            output_utilization: self.calculate_output_utilization(),
        }
    }
}
```

### Work Submission Pattern

```rust
/// Example: Parameter sweep with unidirectional pipeline
pub async fn parameter_sweep_unidirectional(
    pipeline: &UnidirectionalPipeline,
    parameters: impl Iterator<Item = ParameterSet>,
) -> Vec<SweepResult> {
    let mut results = Vec::new();
    let mut pending_count = 0;
    
    for params in parameters {
        // Encode parameters as bytes
        let data = params.encode();
        
        // Submit (fire and forget!)
        pipeline.submit_work(&data)?;
        pending_count += 1;
        
        // Poll for completed results (non-blocking)
        // This keeps the output buffer from filling up
        for completed in pipeline.poll_results() {
            results.push(SweepResult::decode(&completed.data));
            pending_count -= 1;
        }
    }
    
    // Drain remaining results
    while pending_count > 0 {
        tokio::time::sleep(Duration::from_micros(100)).await;
        for completed in pipeline.poll_results() {
            results.push(SweepResult::decode(&completed.data));
            pending_count -= 1;
        }
    }
    
    results
}
```

---

## Bandwidth Throttling

To accurately simulate 90/10 partitioning, we can throttle bandwidth:

```rust
/// Bandwidth throttler for simulation
pub struct BandwidthThrottler {
    target_rate: f64,      // bytes per second
    window_size: Duration, // measurement window
    bytes_transferred: AtomicU64,
    window_start: Mutex<Instant>,
}

impl BandwidthThrottler {
    /// Wait if we're exceeding target bandwidth
    pub async fn throttle(&self, bytes: usize) {
        let now = Instant::now();
        let mut window_start = self.window_start.lock().await;
        
        let elapsed = now.duration_since(*window_start);
        let bytes_this_window = self.bytes_transferred.load(Ordering::Relaxed);
        
        // Reset window if needed
        if elapsed >= self.window_size {
            *window_start = now;
            self.bytes_transferred.store(0, Ordering::Relaxed);
            return;
        }
        
        // Calculate current rate
        let current_rate = bytes_this_window as f64 / elapsed.as_secs_f64();
        
        if current_rate > self.target_rate {
            // Throttle: wait until we're under budget
            let excess_bytes = bytes_this_window as f64 - (self.target_rate * elapsed.as_secs_f64());
            let wait_time = Duration::from_secs_f64(excess_bytes / self.target_rate);
            tokio::time::sleep(wait_time).await;
        }
        
        self.bytes_transferred.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}

impl UnidirectionalPipeline {
    /// Submit with bandwidth throttling
    pub async fn submit_work_throttled(&self, data: &[u8]) -> Result<WorkHandle> {
        // Throttle input to 90% of PCIe bandwidth
        self.input_throttler.throttle(data.len()).await;
        self.submit_work(data)
    }
}
```

---

## Metrics and Validation

```rust
#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
    pub work_units_submitted: u64,
    pub work_units_completed: u64,
    pub input_utilization: f32,   // 0.0 - 1.0
    pub output_utilization: f32,  // 0.0 - 1.0
}

impl PipelineStats {
    /// Validate we're achieving unidirectional pattern
    pub fn validate_unidirectional(&self, config: &UnidirectionalConfig) -> ValidationResult {
        let input_ratio = self.bytes_uploaded as f64 / 
            (self.bytes_uploaded + self.bytes_downloaded) as f64;
        
        let expected_ratio = config.input_bandwidth_target as f64 / 
            (config.input_bandwidth_target + config.output_bandwidth_target) as f64;
        
        let deviation = (input_ratio - expected_ratio).abs();
        
        ValidationResult {
            achieved_input_ratio: input_ratio,
            expected_input_ratio: expected_ratio,
            deviation,
            is_valid: deviation < 0.05, // Within 5% of target
        }
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub achieved_input_ratio: f64,
    pub expected_input_ratio: f64,
    pub deviation: f64,
    pub is_valid: bool,
}
```

---

## Comparison: Traditional vs Simulated Unidirectional

### Benchmark Setup

```rust
/// Benchmark comparing round-trip vs unidirectional patterns
pub async fn benchmark_patterns(
    device: Arc<WgpuDevice>,
    work_units: usize,
    unit_size: usize,
) -> BenchmarkResult {
    // Traditional round-trip pattern
    let traditional_time = {
        let start = Instant::now();
        for _ in 0..work_units {
            let input = vec![0u8; unit_size];
            device.upload(&input).await?;
            device.dispatch_compute().await?;
            let _output = device.download().await?;  // BLOCKS!
        }
        start.elapsed()
    };
    
    // Simulated unidirectional pattern
    let unidirectional_time = {
        let pipeline = UnidirectionalPipeline::new(
            device.clone(),
            UnidirectionalConfig::default(),
        ).await?;
        
        let start = Instant::now();
        
        // Submit all work (fire and forget)
        for _ in 0..work_units {
            let input = vec![0u8; unit_size];
            pipeline.submit_work(&input)?;
        }
        
        // Collect all results
        let mut collected = 0;
        while collected < work_units {
            collected += pipeline.poll_results().len();
            tokio::time::sleep(Duration::from_micros(10)).await;
        }
        
        start.elapsed()
    };
    
    BenchmarkResult {
        traditional_time,
        unidirectional_time,
        speedup: traditional_time.as_secs_f64() / unidirectional_time.as_secs_f64(),
    }
}
```

### Expected Results

| Workload | Traditional | Unidirectional | Speedup |
|----------|-------------|----------------|---------|
| 1K small work units | Latency-bound | Throughput-bound | 2-5× |
| 10K medium work units | PCIe contention | Pipelined | 3-10× |
| 100K tiny work units | Dominated by round-trips | Amortized | 10-50× |

---

## Integration with ToadStool

### Feature Flag

```toml
# Cargo.toml
[features]
default = []
unidirectional = []  # Enable unidirectional pipeline simulation
```

### API Surface

```rust
// In crates/barracuda/src/lib.rs
#[cfg(feature = "unidirectional")]
pub mod unidirectional {
    pub use crate::pipeline::unidirectional::{
        UnidirectionalConfig,
        UnidirectionalPipeline,
        PipelineStats,
        WorkHandle,
        CompletedWork,
    };
}
```

### Usage

```rust
use barracuda::unidirectional::{UnidirectionalPipeline, UnidirectionalConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let device = WgpuDevice::new().await?;
    
    let config = UnidirectionalConfig {
        input_bandwidth_target: 0.90,
        output_bandwidth_target: 0.10,
        strict_mode: true,  // Panic if we accidentally do sync readback
        ..Default::default()
    };
    
    let pipeline = UnidirectionalPipeline::new(device.into(), config).await?;
    
    // Fire-and-forget work submission
    for params in parameter_space() {
        pipeline.submit_work(&params.encode())?;
    }
    
    // Async result collection
    while let Some(results) = pipeline.poll_results() {
        process_results(results);
    }
    
    // Validate we achieved unidirectional pattern
    let stats = pipeline.stats();
    let validation = stats.validate_unidirectional(&config);
    assert!(validation.is_valid, "Failed to achieve unidirectional pattern");
    
    Ok(())
}
```

---

## Key Insight

**The unidirectional pattern is a data flow discipline, not a hardware requirement.**

By enforcing:
- Fire-and-forget input (never wait for results during upload)
- Batched async output (never block on readback)
- Ring buffer staging (smooth out bursts)

We can achieve most of the benefits of physical separation **without extra hardware**.

The hardware (HDMI + capture card) is an optimization for when you need:
- True physical isolation
- Cross-machine pipelines
- Maximum throughput

But the **pattern** can be validated and used with just software.

---

## Next Steps

1. [ ] Implement `GpuRingBuffer` in BarraCuda
2. [ ] Implement `UnidirectionalPipeline` 
3. [ ] Add bandwidth throttling
4. [ ] Benchmark against traditional pattern
5. [ ] Integrate with hotSpring parameter sweeps

---

*From the ToadStool evolution desk — software unidirectional simulation*
