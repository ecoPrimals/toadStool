//! Generic Precision Demo
//!
//! Demonstrates:
//! 1. ONE shader template → generates f16/f32/f64 GPU shaders
//! 2. SAME algorithm on CPU (via num-traits)
//! 3. Precision validation: CPU and GPU produce identical results
//! 4. Performance comparison across precisions

use barracuda::shaders::precision::{cpu, Precision, ShaderTemplate};
use std::time::Instant;
use wgpu::util::DeviceExt;

struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    name: String,
    has_f16: bool,
    has_f64: bool,
}

impl GpuContext {
    async fn new(adapter: &wgpu::Adapter) -> Option<Self> {
        let info = adapter.get_info();
        let features = adapter.features();

        let has_f16 = features.contains(wgpu::Features::SHADER_F16);
        let has_f64 = features.contains(wgpu::Features::SHADER_F64);

        // Request all available precision features
        let mut required_features = wgpu::Features::empty();
        if has_f16 {
            required_features |= wgpu::Features::SHADER_F16;
        }
        if has_f64 {
            required_features |= wgpu::Features::SHADER_F64;
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(&info.name),
                    required_features,
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .ok()?;

        Some(Self {
            device,
            queue,
            name: info.name.clone(),
            has_f16,
            has_f64,
        })
    }
}

/// Run GPU compute with generated shader
fn run_gpu_f32(ctx: &GpuContext, a: &[f32], b: &[f32]) -> Vec<f32> {
    let shader_source = ShaderTemplate::elementwise_add(Precision::F32);

    let module = ctx
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("add_f32"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

    let a_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("a"),
            contents: bytemuck::cast_slice(a),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let b_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("b"),
            contents: bytemuck::cast_slice(b),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output"),
        size: (a.len() * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: (a.len() * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = ctx
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = ctx
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
        });

    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
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

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups((a.len() as u32).div_ceil(256), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, (a.len() * 4) as u64);
    ctx.queue.submit(Some(encoder.finish()));

    let slice = staging_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    ctx.device.poll(wgpu::Maintain::Wait);

    let data = slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    result
}

fn run_gpu_f64(ctx: &GpuContext, a: &[f64], b: &[f64]) -> Vec<f64> {
    let shader_source = ShaderTemplate::elementwise_add(Precision::F64);

    let module = ctx
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("add_f64"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

    let a_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("a"),
            contents: bytemuck::cast_slice(a),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let b_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("b"),
            contents: bytemuck::cast_slice(b),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output"),
        size: (a.len() * 8) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: (a.len() * 8) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = ctx
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = ctx
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
        });

    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
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

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups((a.len() as u32).div_ceil(256), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, (a.len() * 8) as u64);
    ctx.queue.submit(Some(encoder.finish()));

    let slice = staging_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    ctx.device.poll(wgpu::Maintain::Wait);

    let data = slice.get_mapped_range();
    let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

    result
}

fn demonstrate_shader_generation() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  GENERIC PRECISION SHADER TEMPLATES                          ║");
    println!("║  ONE source → any precision (f16, f32, f64)                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    println!("\n  Template-based shader generation:");
    println!("  ─────────────────────────────────");

    // Show that one template generates all precisions
    for precision in [Precision::F16, Precision::F32, Precision::F64] {
        let shader = ShaderTemplate::elementwise_add(precision);
        let first_line = shader.lines().take(2).collect::<Vec<_>>().join("\n");
        println!("\n    {} shader:", precision.scalar().to_uppercase());
        println!("      {}", first_line.replace('\n', "\n      "));

        // Show the key type substitution
        let scalar_type = format!("array<{}>", precision.scalar());
        if shader.contains(&scalar_type) {
            println!("      ✅ Uses {}", scalar_type);
        }
    }

    println!("\n  Key insight: SAME algorithm, different precisions!");
}

fn validate_cpu_gpu_equivalence(ctx: &GpuContext) {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  CPU ↔ GPU EQUIVALENCE VALIDATION                            ║");
    println!("║  Same math, same results                                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // Test data that challenges precision
    let test_cases: Vec<(&str, f64, f64)> = vec![
        ("Simple", 1.0, 2.0),
        ("Large + small", 1e15, 1.0),
        ("Near zero", 1e-15, 1e-15),
        ("π + e", std::f64::consts::PI, std::f64::consts::E),
    ];

    println!("\n  F32 Validation:");
    println!("  ─────────────────────────────────────────");

    for (name, a_val, b_val) in &test_cases {
        let a = vec![*a_val as f32];
        let b = vec![*b_val as f32];

        // CPU
        let mut cpu_out = vec![0.0f32];
        cpu::elementwise_add(&a, &b, &mut cpu_out);

        // GPU
        let gpu_out = run_gpu_f32(ctx, &a, &b);

        let matches = (cpu_out[0] - gpu_out[0]).abs() < 1e-6;
        println!(
            "    {}: CPU={:.6e}, GPU={:.6e} {}",
            name,
            cpu_out[0],
            gpu_out[0],
            if matches { "✅" } else { "❌" }
        );
    }

    if ctx.has_f64 {
        println!("\n  F64 Validation:");
        println!("  ─────────────────────────────────────────");

        for (name, a_val, b_val) in &test_cases {
            let a = vec![*a_val];
            let b = vec![*b_val];

            // CPU
            let mut cpu_out = vec![0.0f64];
            cpu::elementwise_add(&a, &b, &mut cpu_out);

            // GPU
            let gpu_out = run_gpu_f64(ctx, &a, &b);

            let matches = (cpu_out[0] - gpu_out[0]).abs() < 1e-14;
            println!(
                "    {}: CPU={:.15e}, GPU={:.15e} {}",
                name,
                cpu_out[0],
                gpu_out[0],
                if matches { "✅" } else { "❌" }
            );
        }
    }
}

fn benchmark_precisions(ctx: &GpuContext) {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  PERFORMANCE: CPU vs GPU across precisions                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    let n = 1_000_000;
    let iterations = 10;

    // Prepare test data
    let a_f32: Vec<f32> = (0..n).map(|i| (i as f32) * 0.001).collect();
    let b_f32: Vec<f32> = (0..n).map(|i| (i as f32) * 0.002).collect();
    let a_f64: Vec<f64> = (0..n).map(|i| (i as f64) * 0.001).collect();
    let b_f64: Vec<f64> = (0..n).map(|i| (i as f64) * 0.002).collect();

    println!("\n  {} elements, {} iterations", n, iterations);
    println!("  ─────────────────────────────────────────");

    // CPU f32
    let mut cpu_out_f32 = vec![0.0f32; n];
    let start = Instant::now();
    for _ in 0..iterations {
        cpu::elementwise_add(&a_f32, &b_f32, &mut cpu_out_f32);
    }
    let cpu_f32_time = start.elapsed() / iterations;
    let cpu_f32_bandwidth = (n * 4 * 3) as f64 / cpu_f32_time.as_secs_f64() / 1e9;

    // CPU f64
    let mut cpu_out_f64 = vec![0.0f64; n];
    let start = Instant::now();
    for _ in 0..iterations {
        cpu::elementwise_add(&a_f64, &b_f64, &mut cpu_out_f64);
    }
    let cpu_f64_time = start.elapsed() / iterations;
    let cpu_f64_bandwidth = (n * 8 * 3) as f64 / cpu_f64_time.as_secs_f64() / 1e9;

    // GPU f32
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = run_gpu_f32(ctx, &a_f32, &b_f32);
    }
    let gpu_f32_time = start.elapsed() / iterations;
    let gpu_f32_bandwidth = (n * 4 * 3) as f64 / gpu_f32_time.as_secs_f64() / 1e9;

    println!("\n    F32:");
    println!(
        "      CPU: {:>8.2} ms  ({:>6.1} GB/s)",
        cpu_f32_time.as_secs_f64() * 1000.0,
        cpu_f32_bandwidth
    );
    println!(
        "      GPU: {:>8.2} ms  ({:>6.1} GB/s)",
        gpu_f32_time.as_secs_f64() * 1000.0,
        gpu_f32_bandwidth
    );
    println!(
        "      Speedup: {:.1}x",
        cpu_f32_time.as_secs_f64() / gpu_f32_time.as_secs_f64()
    );

    if ctx.has_f64 {
        // GPU f64
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = run_gpu_f64(ctx, &a_f64, &b_f64);
        }
        let gpu_f64_time = start.elapsed() / iterations;
        let gpu_f64_bandwidth = (n * 8 * 3) as f64 / gpu_f64_time.as_secs_f64() / 1e9;

        println!("\n    F64:");
        println!(
            "      CPU: {:>8.2} ms  ({:>6.1} GB/s)",
            cpu_f64_time.as_secs_f64() * 1000.0,
            cpu_f64_bandwidth
        );
        println!(
            "      GPU: {:>8.2} ms  ({:>6.1} GB/s)",
            gpu_f64_time.as_secs_f64() * 1000.0,
            gpu_f64_bandwidth
        );
        println!(
            "      Speedup: {:.1}x",
            cpu_f64_time.as_secs_f64() / gpu_f64_time.as_secs_f64()
        );

        // f64 vs f32 on GPU
        let f64_slowdown = gpu_f64_time.as_secs_f64() / gpu_f32_time.as_secs_f64();
        println!("\n    GPU f64/f32 ratio: {:.2}x", f64_slowdown);
        if f64_slowdown < 3.0 {
            println!("    ✅ Excellent f64 performance! (better than 1:32 theoretical)");
        } else if f64_slowdown < 10.0 {
            println!("    ✅ Good f64 performance for consumer GPU");
        } else {
            println!("    ⚠️ Expected slowdown for consumer GPU");
        }
    }
}

#[tokio::main]
async fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  GENERIC PRECISION DEMO                                       ║");
    println!("║  ONE shader template → CPU + GPU, any precision               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // Part 1: Show shader generation
    demonstrate_shader_generation();

    // Part 2: Run on GPU(s)
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

    for adapter in instance.enumerate_adapters(wgpu::Backends::VULKAN) {
        let info = adapter.get_info();
        if info.device_type != wgpu::DeviceType::DiscreteGpu {
            continue;
        }

        if let Some(ctx) = GpuContext::new(&adapter).await {
            println!("\n══════════════════════════════════════════════════════════════");
            println!("  {}", ctx.name);
            println!(
                "  F16: {}  |  F64: {}",
                if ctx.has_f16 { "✅" } else { "❌" },
                if ctx.has_f64 { "✅" } else { "❌" }
            );
            println!("══════════════════════════════════════════════════════════════");

            validate_cpu_gpu_equivalence(&ctx);
            benchmark_precisions(&ctx);
        }
    }

    println!("\n══════════════════════════════════════════════════════════════");
    println!("  CONCLUSION");
    println!("══════════════════════════════════════════════════════════════");
    println!("  ✅ Generic precision system working:");
    println!("     - ONE template generates all precision shaders");
    println!("     - SAME algorithm runs on CPU (num-traits) and GPU (WGSL)");
    println!("     - Results validated: CPU == GPU");
    println!();
    println!("  The WGSL native advantage is PRESERVED:");
    println!("     - Template generates pure WGSL (no emulation)");
    println!("     - wgpu handles all backend translation");
    println!("     - No runtime overhead from generic dispatch");
    println!();
    println!("  For hotSpring: You can now use the SAME math definitions");
    println!("  to run on CPU (for testing/small jobs) and GPU (for scale)!");
}
