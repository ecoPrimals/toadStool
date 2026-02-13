//! Latency Breakdown Benchmark
//!
//! Isolates WHERE the latency is coming from:
//! 1. Shader compilation
//! 2. Pipeline creation
//! 3. Bind group creation  
//! 4. Command encoding
//! 5. Queue submission
//! 6. GPU execution + sync

use anyhow::Result;
use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use std::time::Instant;
use wgpu::util::DeviceExt;

const SHADER_SOURCE: &str = r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) { return; }
    output[idx] = a[idx] + b[idx];
}
"#;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     LATENCY BREAKDOWN BENCHMARK                                               ║");
    println!("║     Finding where the overhead comes from                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let config = WorkloadConfig {
        exclude_software: true,
        min_gflops: 100.0,
        ..Default::default()
    };
    
    let pool = GpuPool::with_config(config).await?;
    
    let size = 1_000_000usize;  // 1M elements
    let iterations = 20;

    for idx in 0..pool.devices().len() {
        let wgpu_device = pool.device(idx).ok_or_else(|| anyhow::anyhow!("No device"))?;
        let device = wgpu_device.device();
        let queue = wgpu_device.queue();
        let name = wgpu_device.name();
        
        println!("\n══════════════════════════════════════════════════════════════════════════════");
        println!("  {}", name);
        println!("══════════════════════════════════════════════════════════════════════════════\n");

        // Create test data and buffers (not timed - this is setup)
        let data: Vec<f32> = (0..size).map(|i| i as f32 * 0.001).collect();
        
        let buf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("A"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let buf_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("B"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let buf_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Out"),
            size: (size * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // ═══════════════════════════════════════════════════════════════════════
        // MEASURE 1: SHADER COMPILATION
        // ═══════════════════════════════════════════════════════════════════════
        let mut shader_times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            let _shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Test"),
                source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
            });
            shader_times.push(start.elapsed().as_micros() as f64);
        }
        let shader_avg = shader_times.iter().sum::<f64>() / iterations as f64;
        let shader_first = shader_times[0];
        let shader_subsequent = shader_times[1..].iter().sum::<f64>() / (iterations - 1) as f64;
        
        println!("  1. SHADER COMPILATION");
        println!("     First:      {:>8.1} μs", shader_first);
        println!("     Subsequent: {:>8.1} μs (avg)", shader_subsequent);
        println!("     Average:    {:>8.1} μs\n", shader_avg);

        // Keep one shader for subsequent tests
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        // ═══════════════════════════════════════════════════════════════════════
        // MEASURE 2: BIND GROUP LAYOUT CREATION
        // ═══════════════════════════════════════════════════════════════════════
        let bgl_entries = [
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
        
        let mut bgl_times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            let _bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &bgl_entries,
            });
            bgl_times.push(start.elapsed().as_micros() as f64);
        }
        let bgl_avg = bgl_times.iter().sum::<f64>() / iterations as f64;
        
        println!("  2. BIND GROUP LAYOUT CREATION");
        println!("     Average:    {:>8.1} μs\n", bgl_avg);

        // Keep one for subsequent tests
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &bgl_entries,
        });

        // ═══════════════════════════════════════════════════════════════════════
        // MEASURE 3: BIND GROUP CREATION
        // ═══════════════════════════════════════════════════════════════════════
        let mut bg_times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            let _bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buf_a.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: buf_b.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: buf_out.as_entire_binding() },
                ],
            });
            bg_times.push(start.elapsed().as_micros() as f64);
        }
        let bg_avg = bg_times.iter().sum::<f64>() / iterations as f64;
        
        println!("  3. BIND GROUP CREATION");
        println!("     Average:    {:>8.1} μs\n", bg_avg);

        // Keep one for subsequent tests
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buf_a.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buf_b.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: buf_out.as_entire_binding() },
            ],
        });

        // ═══════════════════════════════════════════════════════════════════════
        // MEASURE 4: PIPELINE CREATION
        // ═══════════════════════════════════════════════════════════════════════
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        
        let mut pipeline_times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            let _pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });
            pipeline_times.push(start.elapsed().as_micros() as f64);
        }
        let pipeline_avg = pipeline_times.iter().sum::<f64>() / iterations as f64;
        let pipeline_first = pipeline_times[0];
        let pipeline_subsequent = pipeline_times[1..].iter().sum::<f64>() / (iterations - 1) as f64;
        
        println!("  4. COMPUTE PIPELINE CREATION");
        println!("     First:      {:>8.1} μs", pipeline_first);
        println!("     Subsequent: {:>8.1} μs (avg)", pipeline_subsequent);
        println!("     Average:    {:>8.1} μs\n", pipeline_avg);

        // Keep one for subsequent tests
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // ═══════════════════════════════════════════════════════════════════════
        // MEASURE 5: COMMAND ENCODING (no submit)
        // ═══════════════════════════════════════════════════════════════════════
        let workgroups = (size as u32).div_ceil(64);
        
        let mut encode_times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);
            }
            let _cmd = encoder.finish();
            encode_times.push(start.elapsed().as_micros() as f64);
        }
        let encode_avg = encode_times.iter().sum::<f64>() / iterations as f64;
        
        println!("  5. COMMAND ENCODING (no submit)");
        println!("     Average:    {:>8.1} μs\n", encode_avg);

        // ═══════════════════════════════════════════════════════════════════════
        // MEASURE 6: SUBMIT + WAIT (GPU execution)
        // ═══════════════════════════════════════════════════════════════════════
        let mut submit_times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);
            }
            
            let start = Instant::now();
            queue.submit(Some(encoder.finish()));
            device.poll(wgpu::Maintain::Wait);
            submit_times.push(start.elapsed().as_micros() as f64);
        }
        let submit_avg = submit_times.iter().sum::<f64>() / iterations as f64;
        
        println!("  6. SUBMIT + GPU EXECUTION + WAIT");
        println!("     Average:    {:>8.1} μs\n", submit_avg);

        // ═══════════════════════════════════════════════════════════════════════
        // MEASURE 7: FULL OPERATION (like current add.rs)
        // ═══════════════════════════════════════════════════════════════════════
        let mut full_times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            
            // Recreate everything like add.rs does
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
            });
            
            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &bgl_entries,
            });
            
            let _bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buf_a.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: buf_b.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: buf_out.as_entire_binding() },
                ],
            });
            
            let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
            
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pl),
                module: &shader,
                entry_point: "main",
            });
            
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);
            }
            
            queue.submit(Some(encoder.finish()));
            device.poll(wgpu::Maintain::Wait);
            
            full_times.push(start.elapsed().as_micros() as f64);
        }
        let full_avg = full_times.iter().sum::<f64>() / iterations as f64;
        
        println!("  7. FULL OPERATION (recreate everything)");
        println!("     Average:    {:>8.1} μs\n", full_avg);

        // ═══════════════════════════════════════════════════════════════════════
        // MEASURE 8: OPTIMIZED (reuse shader + pipeline)
        // ═══════════════════════════════════════════════════════════════════════
        let mut opt_times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            
            // Only create encoder and submit - reuse everything else
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);
            }
            
            queue.submit(Some(encoder.finish()));
            device.poll(wgpu::Maintain::Wait);
            
            opt_times.push(start.elapsed().as_micros() as f64);
        }
        let opt_avg = opt_times.iter().sum::<f64>() / iterations as f64;
        
        println!("  8. OPTIMIZED (reuse shader + pipeline)");
        println!("     Average:    {:>8.1} μs\n", opt_avg);

        // ═══════════════════════════════════════════════════════════════════════
        // SUMMARY
        // ═══════════════════════════════════════════════════════════════════════
        println!("  ══════════════════════════════════════════════════════════════");
        println!("  SUMMARY:");
        println!("  ══════════════════════════════════════════════════════════════");
        println!("  Shader compilation:     {:>8.1} μs ({:>5.1}% of full)", shader_avg, 100.0 * shader_avg / full_avg);
        println!("  Pipeline creation:      {:>8.1} μs ({:>5.1}% of full)", pipeline_avg, 100.0 * pipeline_avg / full_avg);
        println!("  Bind group layout:      {:>8.1} μs ({:>5.1}% of full)", bgl_avg, 100.0 * bgl_avg / full_avg);
        println!("  Bind group:             {:>8.1} μs ({:>5.1}% of full)", bg_avg, 100.0 * bg_avg / full_avg);
        println!("  Encoding:               {:>8.1} μs ({:>5.1}% of full)", encode_avg, 100.0 * encode_avg / full_avg);
        println!("  Submit + GPU + Wait:    {:>8.1} μs ({:>5.1}% of full)", submit_avg, 100.0 * submit_avg / full_avg);
        println!("  ──────────────────────────────────────────────────────────────");
        println!("  Full (current add.rs):  {:>8.1} μs", full_avg);
        println!("  Optimized (reuse):      {:>8.1} μs", opt_avg);
        println!("  Potential speedup:      {:>8.1}x", full_avg / opt_avg);
        println!("\n  CUDA reference:         ~15-50 μs");
        println!("  ROCm reference:         ~20-60 μs");
    }

    println!("\n═══ ROOT CAUSE ANALYSIS ═══\n");
    println!("  The massive latency gap comes from recreating resources on EVERY operation:");
    println!("  - Shader compilation: WGSL → SPIR-V → GPU machine code (expensive!)");
    println!("  - Pipeline creation: Validates shader, creates GPU state");
    println!();
    println!("  CUDA/ROCm compile kernels ONCE at load time, then just dispatch.");
    println!("  wgpu CAN do this too - we just need to cache shaders and pipelines.");
    println!();
    println!("  FIX: Cache shaders and pipelines per operation type.");
    println!("  Expected improvement: 5-10x faster single-op latency.");

    Ok(())
}
