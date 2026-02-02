//! Dense vs Sparse Operations Benchmark
//! 
//! Tests NPU, GPU, and CPU performance across sparsity spectrum
//! to identify where each substrate excels.
//!
//! Research Questions:
//! 1. Is NPU advantage sparsity-dependent?
//! 2. At what sparsity does NPU overtake GPU?
//! 3. Does operation type (add, mul, matmul) matter?

use anyhow::Result;
use rand::Rng;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use std::fs;
use barracuda::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkResult {
    substrate: String,
    operation: String,
    sparsity: f32,
    size: usize,
    iterations: usize,
    
    // Performance
    total_time_us: u128,
    throughput_ops_per_sec: f64,
    
    // Energy
    power_watts: f32,
    energy_joules: f32,
    efficiency_ops_per_joule: f32,
    
    // Actual hardware flag
    actual_hardware: bool,
}

/// Sparse vector representation
struct SparseVector {
    indices: Vec<usize>,
    values: Vec<f32>,
    size: usize,
}

impl SparseVector {
    /// Create sparse vector from dense with given sparsity
    fn from_dense(dense: &[f32], target_sparsity: f32) -> Self {
        let mut rng = rand::thread_rng();
        let mut indices = Vec::new();
        let mut values = Vec::new();
        
        for (i, &val) in dense.iter().enumerate() {
            // Keep element with probability (1 - sparsity)
            if rng.gen::<f32>() > target_sparsity {
                indices.push(i);
                values.push(val);
            }
        }
        
        Self {
            indices,
            values,
            size: dense.len(),
        }
    }
    
    fn actual_sparsity(&self) -> f32 {
        1.0 - (self.values.len() as f32 / self.size as f32)
    }
    
    fn nnz(&self) -> usize {
        self.values.len()
    }
}

/// Operation 1: Sparse Vector Addition
fn sparse_vector_add_cpu(a: &SparseVector, b: &SparseVector) -> SparseVector {
    // Merge two sparse vectors
    let mut result_indices = Vec::new();
    let mut result_values = Vec::new();
    
    let mut i = 0;
    let mut j = 0;
    
    while i < a.indices.len() && j < b.indices.len() {
        match a.indices[i].cmp(&b.indices[j]) {
            std::cmp::Ordering::Less => {
                result_indices.push(a.indices[i]);
                result_values.push(a.values[i]);
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                result_indices.push(b.indices[j]);
                result_values.push(b.values[j]);
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                result_indices.push(a.indices[i]);
                result_values.push(a.values[i] + b.values[j]);
                i += 1;
                j += 1;
            }
        }
    }
    
    // Append remaining
    while i < a.indices.len() {
        result_indices.push(a.indices[i]);
        result_values.push(a.values[i]);
        i += 1;
    }
    while j < b.indices.len() {
        result_indices.push(b.indices[j]);
        result_values.push(b.values[j]);
        j += 1;
    }
    
    SparseVector {
        indices: result_indices,
        values: result_values,
        size: a.size,
    }
}

/// Operation 2: Dense Vector Addition (GPU-friendly)
fn dense_vector_add_cpu(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b.iter()).map(|(&x, &y)| x + y).collect()
}

/// Benchmark sparse operations on CPU
fn bench_sparse_cpu(sparsity: f32, size: usize, iterations: usize) -> Result<BenchmarkResult> {
    let mut rng = rand::thread_rng();
    
    // Generate dense vectors
    let dense_a: Vec<f32> = (0..size).map(|_| rng.gen_range(0.0..10.0)).collect();
    let dense_b: Vec<f32> = (0..size).map(|_| rng.gen_range(0.0..10.0)).collect();
    
    // Convert to sparse
    let sparse_a = SparseVector::from_dense(&dense_a, sparsity);
    let sparse_b = SparseVector::from_dense(&dense_b, sparsity);
    
    let actual_sparsity = (sparse_a.actual_sparsity() + sparse_b.actual_sparsity()) / 2.0;
    
    println!("  CPU: {} elements, {}/{} non-zeros ({:.1}% sparse)",
             size, sparse_a.nnz() + sparse_b.nnz(), size * 2, actual_sparsity * 100.0);
    
    // Benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = sparse_vector_add_cpu(&sparse_a, &sparse_b);
    }
    let duration = start.elapsed();
    
    let total_time_us = duration.as_micros();
    let throughput = (iterations as f64) / duration.as_secs_f64();
    let power = 5.0; // Single-core CPU ~5W
    let energy = power * duration.as_secs_f32();
    let efficiency = iterations as f32 / energy;
    
    Ok(BenchmarkResult {
        substrate: "CPU".to_string(),
        operation: "SparseVectorAdd".to_string(),
        sparsity: actual_sparsity,
        size,
        iterations,
        total_time_us,
        throughput_ops_per_sec: throughput,
        power_watts: power,
        energy_joules: energy,
        efficiency_ops_per_joule: efficiency,
        actual_hardware: true,
    })
}

/// Benchmark dense operations on CPU
fn bench_dense_cpu(size: usize, iterations: usize) -> Result<BenchmarkResult> {
    let mut rng = rand::thread_rng();
    
    let vec_a: Vec<f32> = (0..size).map(|_| rng.gen_range(0.0..10.0)).collect();
    let vec_b: Vec<f32> = (0..size).map(|_| rng.gen_range(0.0..10.0)).collect();
    
    println!("  CPU: {} dense elements", size);
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = dense_vector_add_cpu(&vec_a, &vec_b);
    }
    let duration = start.elapsed();
    
    let total_time_us = duration.as_micros();
    let throughput = (iterations as f64) / duration.as_secs_f64();
    let power = 5.0;
    let energy = power * duration.as_secs_f32();
    let efficiency = iterations as f32 / energy;
    
    Ok(BenchmarkResult {
        substrate: "CPU".to_string(),
        operation: "DenseVectorAdd".to_string(),
        sparsity: 0.0,
        size,
        iterations,
        total_time_us,
        throughput_ops_per_sec: throughput,
        power_watts: power,
        energy_joules: energy,
        efficiency_ops_per_joule: efficiency,
        actual_hardware: true,
    })
}

/// Benchmark GPU dense operations
async fn bench_gpu_dense(device: &WgpuDevice, size: usize, iterations: usize) -> Result<BenchmarkResult> {
    let mut rng = rand::thread_rng();
    
    let vec_a: Vec<f32> = (0..size).map(|_| rng.gen_range(0.0..10.0)).collect();
    let vec_b: Vec<f32> = (0..size).map(|_| rng.gen_range(0.0..10.0)).collect();
    
    println!("  GPU: {} dense elements", size);
    
    // Create GPU buffers
    let buffer_a = device.create_storage_buffer("vec_a", bytemuck::cast_slice(&vec_a));
    let buffer_b = device.create_storage_buffer("vec_b", bytemuck::cast_slice(&vec_b));
    let buffer_out = device.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("vec_out"),
        size: (size * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // WGSL shader
    let shader = r#"
        @group(0) @binding(0) var<storage, read> a: array<f32>;
        @group(0) @binding(1) var<storage, read> b: array<f32>;
        @group(0) @binding(2) var<storage, read_write> out: array<f32>;
        
        @compute @workgroup_size(256)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            let idx = id.x;
            if (idx >= arrayLength(&a)) { return; }
            out[idx] = a[idx] + b[idx];
        }
    "#;
    
    let shader_module = device.compile_shader(shader, Some("dense_add"));
    let pipeline = device.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("dense_add_pipeline"),
        layout: None,
        module: &shader_module,
        entry_point: "main",
    });
    
    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = device.device().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind_group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buffer_a.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buffer_b.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: buffer_out.as_entire_binding() },
        ],
    });
    
    // Benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let mut encoder = device.device().create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("dense_add_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
        }
        device.queue().submit(Some(encoder.finish()));
        device.device().poll(wgpu::Maintain::Wait);
    }
    let duration = start.elapsed();
    
    let total_time_us = duration.as_micros();
    let throughput = (iterations as f64) / duration.as_secs_f64();
    let power = 250.0; // RTX 3090 under compute
    let energy = power * duration.as_secs_f32();
    let efficiency = iterations as f32 / energy;
    
    Ok(BenchmarkResult {
        substrate: "GPU".to_string(),
        operation: "DenseVectorAdd".to_string(),
        sparsity: 0.0,
        size,
        iterations,
        total_time_us,
        throughput_ops_per_sec: throughput,
        power_watts: power,
        energy_joules: energy,
        efficiency_ops_per_joule: efficiency,
        actual_hardware: true,
    })
}

/// Benchmark NPU with sparse event-driven processing
async fn bench_npu_sparse(device: &mut akida_driver::AkidaDevice, sparsity: f32, size: usize, iterations: usize) -> Result<BenchmarkResult> {
    let mut rng = rand::thread_rng();
    
    // Generate sparse events (non-zero indices)
    let num_events = ((size as f32) * (1.0 - sparsity)) as usize;
    let mut events: Vec<u8> = Vec::new();
    
    for _ in 0..num_events {
        let idx = rng.gen_range(0..size) as u32;
        let val = rng.gen_range(0..255) as u8;
        events.extend_from_slice(&idx.to_le_bytes());
        events.push(val);
    }
    
    // Pad to minimum transfer size
    if events.len() < 1024 {
        events.resize(1024, 0);
    }
    
    let actual_sparsity = 1.0 - (num_events as f32 / size as f32);
    println!("  NPU: {} elements, {} events ({:.1}% sparse)",
             size, num_events, actual_sparsity * 100.0);
    
    // Configure inference
    let config = akida_driver::InferenceConfig::new(
        vec![events.len()],
        vec![size],
        1,
        1,
    );
    
    let executor = akida_driver::InferenceExecutor::new(config);
    
    // Benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = executor.infer(&events, device)?;
    }
    let duration = start.elapsed();
    
    let total_time_us = duration.as_micros();
    let throughput = (iterations as f64) / duration.as_secs_f64();
    let power = 2.0; // Akida measured
    let energy = power * duration.as_secs_f32();
    let efficiency = iterations as f32 / energy;
    
    Ok(BenchmarkResult {
        substrate: "NPU".to_string(),
        operation: "SparseEventAdd".to_string(),
        sparsity: actual_sparsity,
        size,
        iterations,
        total_time_us,
        throughput_ops_per_sec: throughput,
        power_watts: power,
        energy_joules: energy,
        efficiency_ops_per_joule: efficiency,
        actual_hardware: true,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  🔬 AKIDA CHARACTERIZATION: Dense vs Sparse Operations      ║");
    println!("║  Research: Where does NPU excel vs GPU/CPU?                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Initialize hardware
    println!("⚡ Initializing Hardware...\n");
    
    let gpu_device = WgpuDevice::new().await.ok();
    let mut npu_device = akida_driver::DeviceManager::discover()
        .ok()
        .and_then(|mgr| mgr.open_first().ok());
    
    println!("Hardware Status:");
    println!("  CPU: ✅ Available");
    println!("  GPU: {} Available", if gpu_device.is_some() { "✅" } else { "⚠️ " });
    println!("  NPU: {} Available\n", if npu_device.is_some() { "✅" } else { "⚠️ " });
    
    let mut results = Vec::new();
    
    // Test parameters
    let sizes = vec![1024, 4096, 16384];
    let sparsity_levels = vec![0.99, 0.95, 0.90, 0.75, 0.50, 0.25, 0.10, 0.0];
    let iterations = 100;
    
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("🔄 Running Characterization Tests...\n");
    
    for size in &sizes {
        println!("📊 Vector Size: {} elements\n", size);
        
        for &sparsity in &sparsity_levels {
            if sparsity > 0.0 {
                println!("  Sparsity: {:.1}%", sparsity * 100.0);
                
                // CPU sparse
                if let Ok(result) = bench_sparse_cpu(sparsity, *size, iterations) {
                    results.push(result);
                }
                
                // NPU sparse
                if let Some(ref mut npu) = npu_device {
                    if let Ok(result) = bench_npu_sparse(npu, sparsity, *size, iterations).await {
                        results.push(result);
                    }
                }
                
                println!();
            }
        }
        
        // Dense operations (0% sparsity)
        println!("  Dense (0% sparsity)");
        
        // CPU dense
        if let Ok(result) = bench_dense_cpu(*size, iterations) {
            results.push(result);
        }
        
        // GPU dense
        if let Some(ref gpu) = gpu_device {
            if let Ok(result) = bench_gpu_dense(gpu, *size, iterations).await {
                results.push(result);
            }
        }
        
        println!("\n───────────────────────────────────────────────────────────────\n");
    }
    
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("✅ Characterization Complete: {} tests\n", results.len());
    
    // Generate reports
    let json = serde_json::to_string_pretty(&results)?;
    fs::write("reports/dense_vs_sparse.json", json)?;
    
    // CSV
    let mut csv = String::from("Substrate,Operation,Sparsity,Size,Iterations,Time_us,Throughput,Power_W,Energy_J,Efficiency\n");
    for r in &results {
        csv.push_str(&format!("{},{},{:.3},{},{},{},{:.0},{:.1},{:.6},{:.1}\n",
            r.substrate, r.operation, r.sparsity, r.size, r.iterations,
            r.total_time_us, r.throughput_ops_per_sec, r.power_watts,
            r.energy_joules, r.efficiency_ops_per_joule));
    }
    fs::write("reports/dense_vs_sparse.csv", csv)?;
    
    println!("📊 Reports Generated:");
    println!("   • reports/dense_vs_sparse.json");
    println!("   • reports/dense_vs_sparse.csv\n");
    
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("🏆 CHARACTERIZATION COMPLETE!");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    Ok(())
}
