//! Streaming Heterogeneous Pipeline
//!
//! Simulates a real-time data processing system where:
//! - GPUs handle preprocessing (normalization, FFT-like transforms)
//! - NPUs handle event detection (spike finding, anomaly detection)
//! - CPUs handle coordination and aggregation
//!
//! This demonstrates continuous parallel utilization of all hardware.

use akida_driver::{select_backend, BackendSelection};

use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use barracuda::tensor::Tensor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;

/// Statistics tracker
struct PipelineStats {
    gpu_frames: AtomicU64,
    npu_events: AtomicU64,
    cpu_aggregations: AtomicU64,
    total_latency_us: AtomicU64,
}

impl PipelineStats {
    fn new() -> Self {
        Self {
            gpu_frames: AtomicU64::new(0),
            npu_events: AtomicU64::new(0),
            cpu_aggregations: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
        }
    }

    fn add_gpu_frame(&self, latency_us: u64) {
        self.gpu_frames.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    fn add_npu_event(&self, latency_us: u64) {
        self.npu_events.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    fn add_cpu_aggregation(&self, latency_us: u64) {
        self.cpu_aggregations.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    fn summary(&self) -> String {
        let gpu = self.gpu_frames.load(Ordering::Relaxed);
        let npu = self.npu_events.load(Ordering::Relaxed);
        let cpu = self.cpu_aggregations.load(Ordering::Relaxed);
        let total = gpu + npu + cpu;
        let latency = self.total_latency_us.load(Ordering::Relaxed);
        let avg_latency = if total > 0 { latency / total } else { 0 };

        format!(
            "GPU: {} frames | NPU: {} events | CPU: {} aggs | Avg latency: {}μs",
            gpu, npu, cpu, avg_latency
        )
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║     STREAMING HETEROGENEOUS PIPELINE                                 ║");
    println!("║     Real-time parallel processing across all hardware                ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    let stats = Arc::new(PipelineStats::new());
    let run_duration = Duration::from_secs(5);
    let start_time = Instant::now();

    // Initialize hardware
    println!("═══ Initializing Hardware ═══");

    let gpu_pool = GpuPool::with_config(WorkloadConfig {
        max_parallel: 4,
        prefer_discrete: true,
        exclude_software: true,
        min_gflops: 50.0,
    })
    .await?;
    println!("✓ GPU Pool: {}", gpu_pool.summary());

    let npu1_result = select_backend(BackendSelection::Vfio, "0000:a1:00.0");
    let npu2_result = select_backend(BackendSelection::Vfio, "0000:e2:00.0");
    println!(
        "✓ NPU #1: {}",
        if npu1_result.is_ok() {
            "Ready"
        } else {
            "Unavailable"
        }
    );
    println!(
        "✓ NPU #2: {}",
        if npu2_result.is_ok() {
            "Ready"
        } else {
            "Unavailable"
        }
    );
    println!(
        "✓ CPU: {} threads",
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    );
    println!();

    // Create channels for streaming data
    let (gpu_tx, mut gpu_rx) = mpsc::channel::<Vec<f32>>(32);
    let (npu_tx, mut npu_rx) = mpsc::channel::<Vec<f32>>(32);
    let (result_tx, mut result_rx) = mpsc::channel::<f64>(64);

    println!("═══ Starting Streaming Pipeline (5 seconds) ═══");
    println!();

    // Data generator task (simulates sensor input)
    let _gen_stats = stats.clone();
    let gen_gpu_tx = gpu_tx.clone();
    let gen_npu_tx = npu_tx.clone();

    let generator = tokio::spawn(async move {
        let mut frame_id = 0u64;
        let start = Instant::now();

        while start.elapsed() < run_duration {
            // Generate synthetic sensor data
            let frame_size = 256;
            let data: Vec<f32> = (0..frame_size)
                .map(|i| {
                    let t = (frame_id * frame_size as u64 + i as u64) as f32 * 0.001;
                    (t.sin() * 0.5 + 0.5 + (t * 7.0).cos() * 0.1).clamp(0.0, 1.0)
                })
                .collect();

            // Route to different processors
            if frame_id.is_multiple_of(3) {
                // Every 3rd frame to NPU for event detection
                let _ = gen_npu_tx.send(data.clone()).await;
            }
            // All frames to GPU for preprocessing
            let _ = gen_gpu_tx.send(data).await;

            frame_id += 1;

            // Simulate ~100 FPS input rate
            sleep(Duration::from_millis(10)).await;
        }
    });

    // GPU processing task
    let gpu_stats = stats.clone();
    let gpu_result_tx = result_tx.clone();
    let gpu_pool_clone = gpu_pool;

    let gpu_processor = tokio::spawn(async move {
        while let Some(data) = gpu_rx.recv().await {
            let start = Instant::now();

            // Get first available GPU
            if let Some(device) = gpu_pool_clone.device(0) {
                // Create tensor and perform GPU operations
                if let Ok(tensor) = Tensor::from_data(&data, vec![16, 16], device.clone()) {
                    // Simulate preprocessing: normalize and transform
                    if let Ok(result) = tensor.add(&tensor) {
                        if let Ok(output) = result.to_vec() {
                            let sum: f64 = output.iter().map(|&x| x as f64).sum();
                            let _ = gpu_result_tx.send(sum).await;
                        }
                    }
                }
            }

            let latency_us = start.elapsed().as_micros() as u64;
            gpu_stats.add_gpu_frame(latency_us);
        }
    });

    // NPU processing task
    let npu_stats = stats.clone();
    let npu_result_tx = result_tx.clone();

    let npu_processor = tokio::spawn(async move {
        // Try to get NPU backends
        let mut npu1 = select_backend(BackendSelection::Vfio, "0000:a1:00.0").ok();
        let mut npu2 = select_backend(BackendSelection::Vfio, "0000:e2:00.0").ok();
        let mut use_npu1 = true;

        while let Some(data) = npu_rx.recv().await {
            let start = Instant::now();

            // Alternate between NPUs for load balancing
            let output = if use_npu1 {
                npu1.as_mut().and_then(|npu| npu.infer(&data).ok())
            } else {
                npu2.as_mut().and_then(|npu| npu.infer(&data).ok())
            };
            use_npu1 = !use_npu1;

            if let Some(out) = output {
                let sum: f64 = out.iter().map(|&x| x as f64).sum();
                let _ = npu_result_tx.send(sum).await;
            }

            let latency_us = start.elapsed().as_micros() as u64;
            npu_stats.add_npu_event(latency_us);
        }
    });

    // CPU aggregation task
    let cpu_stats = stats.clone();

    let cpu_aggregator = tokio::spawn(async move {
        let mut buffer: Vec<f64> = Vec::with_capacity(100);

        while let Some(value) = result_rx.recv().await {
            buffer.push(value);

            // Aggregate every 50 results
            if buffer.len() >= 50 {
                let start = Instant::now();

                // Compute statistics
                let mean: f64 = buffer.iter().sum::<f64>() / buffer.len() as f64;
                let variance: f64 =
                    buffer.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / buffer.len() as f64;
                let _std_dev = variance.sqrt();

                buffer.clear();

                let latency_us = start.elapsed().as_micros() as u64;
                cpu_stats.add_cpu_aggregation(latency_us);
            }
        }
    });

    // Progress display
    let display_stats = stats.clone();
    let progress = tokio::spawn(async move {
        for i in 0..5 {
            sleep(Duration::from_secs(1)).await;
            println!("  [{}s] {}", i + 1, display_stats.summary());
        }
    });

    // Wait for completion
    let _ = generator.await;
    drop(gpu_tx);
    drop(npu_tx);
    drop(result_tx);

    let _ = gpu_processor.await;
    let _ = npu_processor.await;
    let _ = cpu_aggregator.await;
    let _ = progress.await;

    let total_duration = start_time.elapsed();

    println!();
    println!("═══ Pipeline Complete ═══");
    println!();
    println!("Final Statistics:");
    println!("  {}", stats.summary());
    println!("  Total runtime: {:.2}s", total_duration.as_secs_f64());

    let gpu_frames = stats.gpu_frames.load(Ordering::Relaxed);
    let npu_events = stats.npu_events.load(Ordering::Relaxed);
    let total_ops = gpu_frames + npu_events;

    if total_duration.as_secs_f64() > 0.0 {
        println!(
            "  Throughput: {:.1} ops/sec",
            total_ops as f64 / total_duration.as_secs_f64()
        );
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║     STREAMING PIPELINE COMPLETE                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");

    Ok(())
}
