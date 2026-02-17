//! NVIDIA Vulkan Analysis - Understanding the 8% vs 77% gap
//!
//! AMD RADV achieves 77% theoretical bandwidth, NVIDIA only 8%.
//! This benchmark investigates where NVIDIA's wgpu/Vulkan overhead comes from.

use anyhow::Result;
use barracuda::device::{warmup_pool, WarmupConfig};
use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use barracuda::prelude::*;
use std::sync::Arc;
use std::time::Instant;

/// Raw wgpu compute without Tensor abstraction
async fn benchmark_raw_compute(
    device: &Arc<WgpuDevice>,
    size: usize,
    iterations: usize,
) -> Result<f64> {
    let shader_source = format!(
        r#"
        @group(0) @binding(0) var<storage, read> a: array<f32>;
        @group(0) @binding(1) var<storage, read> b: array<f32>;
        @group(0) @binding(2) var<storage, read_write> result: array<f32>;

        @compute @workgroup_size(256)
        fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
            let idx = global_id.x;
            if (idx < {}u) {{
                result[idx] = a[idx] + b[idx];
            }}
        }}
    "#,
        size
    );

    let wgpu_device = device.device();

    // Create shader module
    let shader = wgpu_device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Raw Add Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Create pipeline layout
    let bind_group_layout =
        wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raw Layout"),
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

    let pipeline_layout = wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Raw Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = wgpu_device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Raw Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    cache: None,
    compilation_options: Default::default(),
    });

    // Create buffers
    let data: Vec<f32> = (0..size).map(|i| (i % 10000) as f32 * 0.0001).collect();
    let buffer_a = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer A"),
        size: (size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let buffer_b = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer B"),
        size: (size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let buffer_c = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer C"),
        size: (size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    device
        .queue()
        .write_buffer(&buffer_a, 0, bytemuck::cast_slice(&data));
    device
        .queue()
        .write_buffer(&buffer_b, 0, bytemuck::cast_slice(&data));

    // Pre-create bind group
    let bind_group = wgpu_device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Raw Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_a.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer_b.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: buffer_c.as_entire_binding(),
            },
        ],
    });

    let workgroups = size.div_ceil(256);

    // Warmup
    for _ in 0..3 {
        let mut encoder =
            wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }
        device.queue().submit(Some(encoder.finish()));
    }
    wgpu_device.poll(wgpu::Maintain::Wait);

    // Benchmark - measure ONLY submission overhead
    let start = Instant::now();
    for _ in 0..iterations {
        let mut encoder =
            wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }
        device.queue().submit(Some(encoder.finish()));
    }
    wgpu_device.poll(wgpu::Maintain::Wait);
    let elapsed = start.elapsed();

    Ok(elapsed.as_secs_f64() * 1000.0 / iterations as f64)
}

/// Measure individual overhead components
async fn measure_overheads(device: &Arc<WgpuDevice>, size: usize) -> Result<()> {
    let wgpu_device = device.device();

    let shader_source = format!(
        r#"
        @group(0) @binding(0) var<storage, read> a: array<f32>;
        @group(0) @binding(1) var<storage, read> b: array<f32>;
        @group(0) @binding(2) var<storage, read_write> result: array<f32>;

        @compute @workgroup_size(256)
        fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
            let idx = global_id.x;
            if (idx < {}u) {{
                result[idx] = a[idx] + b[idx];
            }}
        }}
    "#,
        size
    );

    // Pre-create everything
    let shader = wgpu_device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let bind_group_layout =
        wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    let pipeline_layout = wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = wgpu_device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
    cache: None,
    compilation_options: Default::default(),
    });

    let buffer_a = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let buffer_b = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let buffer_c = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let bind_group = wgpu_device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_a.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer_b.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: buffer_c.as_entire_binding(),
            },
        ],
    });

    let workgroups = size.div_ceil(256);
    let iterations = 100;

    // 1. Measure encoder creation
    let start = Instant::now();
    for _ in 0..iterations {
        let _encoder =
            wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    }
    let encoder_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

    // 2. Measure pass begin/end
    let start = Instant::now();
    for _ in 0..iterations {
        let mut encoder =
            wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let _pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        }
        let _ = encoder.finish();
    }
    let pass_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64 - encoder_time;

    // 3. Measure dispatch recording
    let start = Instant::now();
    for _ in 0..iterations {
        let mut encoder =
            wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }
        let _ = encoder.finish();
    }
    let dispatch_record_time =
        start.elapsed().as_secs_f64() * 1000.0 / iterations as f64 - encoder_time - pass_time;

    // 4. Measure submit (without waiting)
    let start = Instant::now();
    for _ in 0..iterations {
        let mut encoder =
            wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }
        device.queue().submit(Some(encoder.finish()));
    }
    wgpu_device.poll(wgpu::Maintain::Wait); // Wait for all
    let submit_total_time = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    let submit_only_time = submit_total_time - encoder_time - pass_time - dispatch_record_time;

    // 5. Measure poll (synchronous wait)
    let start = Instant::now();
    for _ in 0..iterations {
        let mut encoder =
            wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }
        device.queue().submit(Some(encoder.finish()));
        wgpu_device.poll(wgpu::Maintain::Wait);
    }
    let total_with_wait = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    let gpu_wait_time = total_with_wait - submit_total_time;

    println!(
        "    Overhead Breakdown ({}M elements, {} iterations):",
        size / 1_000_000,
        iterations
    );
    println!("    ┌──────────────────────────┬────────────┬───────────┐");
    println!("    │ Component                │ Time (μs)  │ % Total   │");
    println!("    ├──────────────────────────┼────────────┼───────────┤");
    println!(
        "    │ Encoder creation         │ {:>8.1}   │ {:>6.1}%   │",
        encoder_time * 1000.0,
        encoder_time / total_with_wait * 100.0
    );
    println!(
        "    │ Compute pass begin/end   │ {:>8.1}   │ {:>6.1}%   │",
        pass_time * 1000.0,
        pass_time / total_with_wait * 100.0
    );
    println!(
        "    │ Dispatch recording       │ {:>8.1}   │ {:>6.1}%   │",
        dispatch_record_time * 1000.0,
        dispatch_record_time / total_with_wait * 100.0
    );
    println!(
        "    │ Queue submit             │ {:>8.1}   │ {:>6.1}%   │",
        submit_only_time * 1000.0,
        submit_only_time / total_with_wait * 100.0
    );
    println!(
        "    │ GPU execution + wait     │ {:>8.1}   │ {:>6.1}%   │",
        gpu_wait_time * 1000.0,
        gpu_wait_time / total_with_wait * 100.0
    );
    println!("    ├──────────────────────────┼────────────┼───────────┤");
    println!(
        "    │ TOTAL per operation      │ {:>8.1}   │ 100.0%    │",
        total_with_wait * 1000.0
    );
    println!("    └──────────────────────────┴────────────┴───────────┘");

    // Calculate theoretical vs actual
    let bytes_transferred = size * 3 * 4; // 3 buffers, 4 bytes each
    let theoretical_peak = if device.name().contains("3090") {
        936.0
    } else {
        576.0
    }; // GB/s
    let theoretical_time_ms = (bytes_transferred as f64 / 1e9) / theoretical_peak * 1000.0;
    let achieved_bandwidth = (bytes_transferred as f64 / 1e9) / (total_with_wait / 1000.0);

    println!(
        "\n    Theoretical minimum: {:.3} ms at {:.0} GB/s",
        theoretical_time_ms, theoretical_peak
    );
    println!(
        "    Achieved:            {:.3} ms at {:.1} GB/s ({:.1}% efficiency)",
        total_with_wait,
        achieved_bandwidth,
        achieved_bandwidth / theoretical_peak * 100.0
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  NVIDIA Vulkan Analysis - Why 8% vs AMD's 77%?                               ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let config = WorkloadConfig {
        exclude_software: true,
        min_gflops: 100.0,
        ..Default::default()
    };

    let pool = GpuPool::with_config(config).await?;
    let devices: Vec<_> = (0..pool.device_count())
        .filter_map(|i| pool.device(i))
        .collect();

    warmup_pool(&devices, &WarmupConfig::default())?;

    for device in &devices {
        let name = device.name();
        let is_nvidia = name.contains("NVIDIA");
        let is_amd = name.contains("AMD") || name.contains("RADV");

        println!("══════════════════════════════════════════════════════════════════════════════");
        println!(
            "  {} [{}]",
            name,
            if is_nvidia {
                "Vulkan/NVIDIA"
            } else if is_amd {
                "Vulkan/RADV"
            } else {
                "Unknown"
            }
        );
        println!(
            "══════════════════════════════════════════════════════════════════════════════\n"
        );

        // Test 1: Raw wgpu compute at different sizes
        println!("  Test 1: Raw wgpu compute (bypassing Tensor layer)");
        println!("  ┌────────────────┬────────────┬────────────┬────────────┐");
        println!("  │ Size           │ Time       │ Bandwidth  │ % Peak     │");
        println!("  ├────────────────┼────────────┼────────────┼────────────┤");

        let theoretical = if is_nvidia { 936.0 } else { 576.0 };

        for (size, label) in [(1_000_000, "1M"), (5_000_000, "5M"), (10_000_000, "10M")] {
            let iterations = if size >= 10_000_000 { 20 } else { 50 };
            let time_ms = benchmark_raw_compute(device, size, iterations).await?;
            let bandwidth = (size * 3 * 4) as f64 / 1e9 / (time_ms / 1000.0);
            let pct = bandwidth / theoretical * 100.0;

            println!(
                "  │ {:>14} │ {:>7.2} ms │ {:>7.1} GB/s│ {:>7.1}%   │",
                label, time_ms, bandwidth, pct
            );
        }
        println!("  └────────────────┴────────────┴────────────┴────────────┘\n");

        // Test 2: Detailed overhead breakdown
        println!("  Test 2: Where does the time go?");
        measure_overheads(device, 10_000_000).await?;

        println!("\n");
    }

    // Summary
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("  INSIGHTS");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    println!("  NVIDIA Vulkan vs AMD RADV observations:\n");

    println!("  1. AMD RADV (open-source) achieves near-peak bandwidth (~77%+)");
    println!("     - Mesa developers have optimized Vulkan paths extensively");
    println!("     - RADV batch submission is highly efficient\n");

    println!("  2. NVIDIA proprietary Vulkan driver has higher overhead");
    println!("     - Possible internal synchronization/validation layers");
    println!("     - NVIDIA optimizes heavily for CUDA, less for Vulkan compute");
    println!("     - Their Vulkan focus is graphics, not GPGPU\n");

    println!("  3. BarraCUDA implications:");
    println!("     - AMD already near parity with vendor-specific solutions");
    println!("     - NVIDIA optimization opportunities:");
    println!("       a) Use CUDA interop for NVIDIA (sacrifice portability)");
    println!("       b) Batch more aggressively to amortize driver overhead");
    println!("       c) Investigate NVIDIA-specific Vulkan extensions");
    println!("       d) Consider timeline semaphores for async submission\n");

    println!("  4. For real workloads:");
    println!("     - Use large batches (amortize overhead)");
    println!("     - Prefer AMD for pure wgpu/Vulkan compute workloads");
    println!("     - NVIDIA still faster for CUDA, but wgpu closes gap at scale\n");

    Ok(())
}
