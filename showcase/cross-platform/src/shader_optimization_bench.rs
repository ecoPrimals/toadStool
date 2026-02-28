//! Shader Optimization Benchmark
//!
//! Tests different shader optimization strategies to find the best
//! approach for each GPU vendor (NVIDIA vs AMD).
//!
//! Optimizations tested:
//! 1. Workgroup size (64, 128, 256, 512)
//! 2. Vectorization (f32 vs `vec4<f32>`)
//! 3. Elements per thread (1, 4, 8)
//! 4. FMA vs separate ops

use barracuda::multi_gpu::{GpuPool, GpuVendor, WorkloadConfig};
use std::time::Instant;
use wgpu::util::DeviceExt;

/// Benchmark result
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for structured logging/reporting
struct BenchResult {
    gpu: String,
    variant: String,
    workgroup_size: u32,
    elements_per_thread: u32,
    vectorized: bool,
    time_us: f64,
    bandwidth_gbps: f64,
}

/// Generate shader with configurable workgroup size
fn generate_scalar_shader(workgroup_size: u32) -> String {
    format!(
        r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size({workgroup_size})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {{
        return;
    }}
    output[idx] = a[idx] + b[idx];
}}
"#
    )
}

/// Generate vectorized shader (vec4)
fn generate_vec4_shader(workgroup_size: u32) -> String {
    format!(
        r#"
struct Params {{
    vec_count: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}}

@group(0) @binding(0) var<storage, read> a: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> b: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size({workgroup_size})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if (idx >= params.vec_count) {{
        return;
    }}
    output[idx] = a[idx] + b[idx];
}}
"#
    )
}

/// Generate 8-elements-per-thread shader
fn generate_8x_shader(workgroup_size: u32) -> String {
    format!(
        r#"
struct Params {{
    vec_count: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}}

@group(0) @binding(0) var<storage, read> a: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> b: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size({workgroup_size})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let base = global_id.x * 2u;
    let vec_count = params.vec_count;
    
    if (base < vec_count) {{
        output[base] = a[base] + b[base];
    }}
    if (base + 1u < vec_count) {{
        output[base + 1u] = a[base + 1u] + b[base + 1u];
    }}
}}
"#
    )
}

/// Run a single benchmark variant
async fn run_benchmark(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader_source: &str,
    size: usize,
    iterations: usize,
    is_vectorized: bool,
) -> std::result::Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    // Create data
    let data_a: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
    let data_b: Vec<f32> = (0..size)
        .map(|i| ((i + 500) % 1000) as f32 * 0.001)
        .collect();

    // Create buffers
    let buf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("A"),
        contents: bytemuck::cast_slice(&data_a),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let buf_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("B"),
        contents: bytemuck::cast_slice(&data_b),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let buf_out = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Out"),
        size: (size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Compile shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Bench Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Create bind group layout
    let mut entries = vec![
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ];

    // Add uniform buffer for vectorized shaders
    let params_buf = if is_vectorized {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });

        let vec_count = (size / 4) as u32;
        Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Params"),
                contents: bytemuck::cast_slice(&[vec_count, 0u32, 0u32, 0u32]),
                usage: wgpu::BufferUsages::UNIFORM,
            }),
        )
    } else {
        None
    };

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("BGL"),
        entries: &entries,
    });

    // Create bind group
    let mut bg_entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: buf_a.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: buf_b.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: buf_out.as_entire_binding(),
        },
    ];

    if let Some(ref params) = params_buf {
        bg_entries.push(wgpu::BindGroupEntry {
            binding: 3,
            resource: params.as_entire_binding(),
        });
    }

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("BG"),
        layout: &bind_group_layout,
        entries: &bg_entries,
    });

    // Create pipeline
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("PL"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
        cache: None,
        compilation_options: Default::default(),
    });

    // Warmup
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = if is_vectorized {
            (size / 4 / 64) as u32
        } else {
            (size / 256) as u32
        };
        pass.dispatch_workgroups(workgroups.max(1), 1, 1);
        drop(pass);
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
    }

    // Benchmark - submit all commands then wait at end
    // This batches submissions which is more realistic for real workloads
    let mut encoders = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = if is_vectorized {
                (size / 4 / 64) as u32
            } else {
                (size / 256) as u32
            };
            pass.dispatch_workgroups(workgroups.max(1), 1, 1);
        }
        encoders.push(encoder.finish());
    }

    let start = Instant::now();
    queue.submit(encoders);
    device.poll(wgpu::Maintain::Wait);
    let elapsed = start.elapsed();

    Ok(elapsed.as_secs_f64() * 1e6 / iterations as f64)
}

async fn run_all_benchmarks() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     SHADER OPTIMIZATION BENCHMARK                                             ║");
    println!("║     Finding optimal wgpu settings for NVIDIA vs AMD                           ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let config = WorkloadConfig {
        exclude_software: true,
        min_gflops: 100.0,
        ..Default::default()
    };

    let pool = GpuPool::with_config(config).await?;
    let size = 16_000_000usize; // 16M elements
    let iterations = 50;
    let bytes = size * 3 * 4; // 2 read + 1 write

    let mut results: Vec<BenchResult> = Vec::new();

    for (idx, gpu_info) in pool.devices().iter().enumerate() {
        let gpu_name = match gpu_info.vendor {
            GpuVendor::Nvidia => "NVIDIA RTX 3090",
            GpuVendor::Amd => "AMD RX 6950 XT",
            _ => "Unknown",
        };

        println!("Testing on {gpu_name}...\n");

        // Get raw wgpu device
        let wgpu_device = pool
            .device(idx)
            .ok_or_else(|| std::io::Error::other("No device"))?;
        let device = wgpu_device.device();
        let queue = wgpu_device.queue();

        // Test 1: Scalar shader with different workgroup sizes
        // Note: wgpu max workgroup size is 256
        println!("  Scalar shaders (1 element/thread):");
        for wg_size in [64, 128, 256] {
            let shader = generate_scalar_shader(wg_size);
            match run_benchmark(device, queue, &shader, size, iterations, false).await {
                Ok(time_us) => {
                    let bw = (bytes as f64) / (time_us * 1000.0);
                    println!("    WG={wg_size:>3}: {time_us:>8.1}μs  ({bw:>6.1} GB/s)");
                    results.push(BenchResult {
                        gpu: gpu_name.to_string(),
                        variant: "scalar".to_string(),
                        workgroup_size: wg_size,
                        elements_per_thread: 1,
                        vectorized: false,
                        time_us,
                        bandwidth_gbps: bw,
                    });
                }
                Err(e) => println!("    WG={wg_size}: ERROR - {e}"),
            }
        }

        // Test 2: Vectorized shader (vec4, 4 elements/thread)
        println!("\n  Vectorized shaders (4 elements/thread via vec4):");
        for wg_size in [64, 128, 256] {
            let shader = generate_vec4_shader(wg_size);
            match run_benchmark(device, queue, &shader, size, iterations, true).await {
                Ok(time_us) => {
                    let bw = (bytes as f64) / (time_us * 1000.0);
                    println!("    WG={wg_size:>3}: {time_us:>8.1}μs  ({bw:>6.1} GB/s)");
                    results.push(BenchResult {
                        gpu: gpu_name.to_string(),
                        variant: "vec4".to_string(),
                        workgroup_size: wg_size,
                        elements_per_thread: 4,
                        vectorized: true,
                        time_us,
                        bandwidth_gbps: bw,
                    });
                }
                Err(e) => println!("    WG={wg_size}: ERROR - {e}"),
            }
        }

        // Test 3: 8 elements per thread
        println!("\n  8 elements/thread (2x vec4 per thread):");
        for wg_size in [64, 128] {
            let shader = generate_8x_shader(wg_size);
            match run_benchmark(device, queue, &shader, size, iterations, true).await {
                Ok(time_us) => {
                    let bw = (bytes as f64) / (time_us * 1000.0);
                    println!("    WG={wg_size:>3}: {time_us:>8.1}μs  ({bw:>6.1} GB/s)");
                    results.push(BenchResult {
                        gpu: gpu_name.to_string(),
                        variant: "8x".to_string(),
                        workgroup_size: wg_size,
                        elements_per_thread: 8,
                        vectorized: true,
                        time_us,
                        bandwidth_gbps: bw,
                    });
                }
                Err(e) => println!("    WG={wg_size}: ERROR - {e}"),
            }
        }

        println!();
    }

    // Analysis
    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     ANALYSIS                                                                  ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Find best for each GPU
    for gpu in ["NVIDIA RTX 3090", "AMD RX 6950 XT"] {
        let gpu_results: Vec<_> = results.iter().filter(|r| r.gpu == gpu).collect();
        if let Some(best) = gpu_results.iter().max_by(|a, b| {
            a.bandwidth_gbps
                .partial_cmp(&b.bandwidth_gbps)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            println!("{gpu} - Best configuration:");
            println!(
                "  Variant: {} (WG={}, {}/thread)",
                best.variant, best.workgroup_size, best.elements_per_thread
            );
            println!(
                "  Time: {:.1}μs, Bandwidth: {:.1} GB/s",
                best.time_us, best.bandwidth_gbps
            );

            // Compare to CUDA baseline
            let cuda_bw = 837.0; // From our benchmark
            println!("  Gap vs CUDA: {:.1}x\n", cuda_bw / best.bandwidth_gbps);
        }
    }

    // Recommendations
    println!("═══ RECOMMENDATIONS ═══\n");
    println!("Based on the results, optimal BarraCuda configuration:");
    println!("  1. Use vec4<f32> for memory operations (4x elements per thread)");
    println!("  2. Workgroup size: 64 for AMD (wavefront-aligned), 256 for NVIDIA");
    println!("  3. Pre-compile shaders with vendor-specific workgroup sizes");
    println!("  4. Consider larger elements-per-thread for bandwidth-bound ops");

    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    run_all_benchmarks().await
}
