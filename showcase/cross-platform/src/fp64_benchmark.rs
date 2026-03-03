// SPDX-License-Identifier: AGPL-3.0-or-later
//! FP64 Benchmark - Compare double precision performance across vendors
//!
//! This benchmark tests native f64 shader performance on GPUs that support SHADER_F64.
//! Both NVIDIA (via Vulkan) and AMD (via RADV) support this extension.
//!
//! Expected fp64:fp32 ratios:
//! - Consumer NVIDIA (RTX 3090): ~1:32
//! - Consumer AMD (RX 6950 XT): ~1:16
//! - Workstation (Titan V): ~1:2

use std::time::Instant;
use wgpu::util::DeviceExt;

const SHADER_ADD_F32: &str = r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {
        return;
    }
    output[idx] = a[idx] + b[idx];
}
"#;

const SHADER_ADD_F64: &str = r#"
// f64 is automatically available when SHADER_F64 feature is enabled on device
@group(0) @binding(0) var<storage, read> a: array<f64>;
@group(0) @binding(1) var<storage, read> b: array<f64>;
@group(0) @binding(2) var<storage, read_write> output: array<f64>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {
        return;
    }
    output[idx] = a[idx] + b[idx];
}
"#;

// Reserved for future multiplication benchmark
#[allow(dead_code)]
const SHADER_MUL_F32: &str = r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {
        return;
    }
    output[idx] = a[idx] * b[idx];
}
"#;

// Reserved for future multiplication benchmark
#[allow(dead_code)]
const SHADER_MUL_F64: &str = r#"
// f64 is automatically available when SHADER_F64 feature is enabled on device
@group(0) @binding(0) var<storage, read> a: array<f64>;
@group(0) @binding(1) var<storage, read> b: array<f64>;
@group(0) @binding(2) var<storage, read_write> output: array<f64>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {
        return;
    }
    output[idx] = a[idx] * b[idx];
}
"#;

struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    name: String,
    has_f64: bool,
}

impl GpuContext {
    async fn new(adapter: &wgpu::Adapter) -> Option<Self> {
        let info = adapter.get_info();
        let features = adapter.features();
        let has_f64 = features.contains(wgpu::Features::SHADER_F64);

        // Request f64 feature if available
        let required_features = if has_f64 {
            wgpu::Features::SHADER_F64
        } else {
            wgpu::Features::empty()
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(&info.name),
                    required_features,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .ok()?;

        Some(Self {
            device,
            queue,
            name: info.name.clone(),
            has_f64,
        })
    }
}

fn run_f32_benchmark(ctx: &GpuContext, size: usize, iterations: usize) -> (f64, f64) {
    // Create data
    let a_data: Vec<f32> = (0..size).map(|i| i as f32).collect();
    let b_data: Vec<f32> = (0..size).map(|i| (i * 2) as f32).collect();

    // Create buffers
    let a_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("a_f32"),
            contents: bytemuck::cast_slice(&a_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let b_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("b_f32"),
            contents: bytemuck::cast_slice(&b_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output_f32"),
        size: (size * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create pipeline for add
    let add_module = ctx
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("add_f32"),
            source: wgpu::ShaderSource::Wgsl(SHADER_ADD_F32.into()),
        });

    let bind_group_layout = ctx
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("f32_layout"),
            entries: &[
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
            ],
        });

    let pipeline_layout = ctx
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("f32_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let add_pipeline = ctx
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("add_f32_pipeline"),
            layout: Some(&pipeline_layout),
            module: &add_module,
            entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
        });

    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("f32_bind_group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: a_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: b_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    // Warmup
    for _ in 0..5 {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&add_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32).div_ceil(256), 1, 1);
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
    ctx.device.poll(wgpu::Maintain::Wait);

    // Benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&add_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32).div_ceil(256), 1, 1);
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
    ctx.device.poll(wgpu::Maintain::Wait);
    let elapsed = start.elapsed();

    let time_per_op = elapsed.as_secs_f64() / iterations as f64;
    let bytes = size * std::mem::size_of::<f32>() * 3; // 2 reads + 1 write
    let bandwidth = (bytes as f64 / time_per_op) / 1e9;

    (time_per_op * 1e6, bandwidth) // Return (μs, GB/s)
}

fn run_f64_benchmark(ctx: &GpuContext, size: usize, iterations: usize) -> Option<(f64, f64)> {
    if !ctx.has_f64 {
        return None;
    }

    // Create data
    let a_data: Vec<f64> = (0..size).map(|i| i as f64).collect();
    let b_data: Vec<f64> = (0..size).map(|i| (i * 2) as f64).collect();

    // Create buffers
    let a_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("a_f64"),
            contents: bytemuck::cast_slice(&a_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let b_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("b_f64"),
            contents: bytemuck::cast_slice(&b_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output_f64"),
        size: (size * std::mem::size_of::<f64>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create pipeline for add
    let add_module = ctx
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("add_f64"),
            source: wgpu::ShaderSource::Wgsl(SHADER_ADD_F64.into()),
        });

    let bind_group_layout = ctx
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("f64_layout"),
            entries: &[
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
            ],
        });

    let pipeline_layout = ctx
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("f64_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let add_pipeline = ctx
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("add_f64_pipeline"),
            layout: Some(&pipeline_layout),
            module: &add_module,
            entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
        });

    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("f64_bind_group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: a_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: b_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    // Warmup
    for _ in 0..5 {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&add_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32).div_ceil(256), 1, 1);
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
    ctx.device.poll(wgpu::Maintain::Wait);

    // Benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&add_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32).div_ceil(256), 1, 1);
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
    ctx.device.poll(wgpu::Maintain::Wait);
    let elapsed = start.elapsed();

    let time_per_op = elapsed.as_secs_f64() / iterations as f64;
    let bytes = size * std::mem::size_of::<f64>() * 3; // 2 reads + 1 write
    let bandwidth = (bytes as f64 / time_per_op) / 1e9;

    Some((time_per_op * 1e6, bandwidth)) // Return (μs, GB/s)
}

#[tokio::main]
async fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  FP64 vs FP32 PRECISION BENCHMARK                            ║");
    println!("║  Comparing double precision performance across vendors        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

    // Collect Vulkan adapters only (they support SHADER_F64)
    let mut contexts: Vec<GpuContext> = Vec::new();
    for adapter in instance.enumerate_adapters(wgpu::Backends::VULKAN) {
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::DiscreteGpu {
            if let Some(ctx) = GpuContext::new(&adapter).await {
                contexts.push(ctx);
            }
        }
    }

    if contexts.is_empty() {
        println!("No discrete GPUs with Vulkan support found!");
        return;
    }

    let sizes = [100_000, 1_000_000, 10_000_000];
    let iterations = 100;

    for ctx in &contexts {
        println!("══════════════════════════════════════════════════════════════");
        println!("  {}", ctx.name);
        println!(
            "  SHADER_F64: {}",
            if ctx.has_f64 {
                "✅ Supported"
            } else {
                "❌ Not available"
            }
        );
        println!("══════════════════════════════════════════════════════════════\n");

        println!(
            "  ┌────────────┬──────────────────────────┬──────────────────────────┬──────────┐"
        );
        println!(
            "  │ Size       │ FP32 (time / bandwidth)  │ FP64 (time / bandwidth)  │ Ratio    │"
        );
        println!(
            "  ├────────────┼──────────────────────────┼──────────────────────────┼──────────┤"
        );

        for &size in &sizes {
            let (f32_time, f32_bw) = run_f32_benchmark(ctx, size, iterations);

            let size_str = if size >= 1_000_000 {
                format!("{}M", size / 1_000_000)
            } else {
                format!("{}K", size / 1_000)
            };

            if let Some((f64_time, f64_bw)) = run_f64_benchmark(ctx, size, iterations) {
                let ratio = f64_time / f32_time;
                println!(
                    "  │ {size_str:>10} │ {f32_time:>8.1} μs / {f32_bw:>6.1} GB/s │ {f64_time:>8.1} μs / {f64_bw:>6.1} GB/s │ {ratio:>6.1}x  │"
                );
            } else {
                println!(
                    "  │ {:>10} │ {:>8.1} μs / {:>6.1} GB/s │ {:>23} │ {:>8} │",
                    size_str, f32_time, f32_bw, "N/A", "N/A"
                );
            }
        }

        println!(
            "  └────────────┴──────────────────────────┴──────────────────────────┴──────────┘\n"
        );
    }

    println!("══════════════════════════════════════════════════════════════");
    println!("  ANALYSIS");
    println!("══════════════════════════════════════════════════════════════");
    println!("  Expected fp64:fp32 ratios:");
    println!("    - Consumer NVIDIA (RTX 30xx/40xx): ~32x slower");
    println!("    - Consumer AMD (RDNA2/3): ~16x slower");
    println!("    - Workstation (Titan V, A100): ~2x slower");
    println!("  ");
    println!("  These consumer GPUs have fp64 for compatibility,");
    println!("  not performance. Use Titan V or datacenter GPUs");
    println!("  for production fp64 workloads.");
}
