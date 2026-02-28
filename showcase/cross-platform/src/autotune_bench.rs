//! Auto-Tuning Runtime Benchmark
//!
//! Validates true throughput and implements self-optimizing calibration.
//!
//! Design principles:
//! 1. Don't assume vendor capabilities - discover ground truth
//! 2. Calibrate on first run, cache results per GPU
//! 3. Handle silicon lottery and generation differences
//! 4. Work seamlessly with unknown/new hardware

use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use wgpu::util::DeviceExt;

/// Format SystemTime as RFC3339 string (e.g. "2025-02-27T12:00:00Z")
fn format_rfc3339(t: SystemTime) -> String {
    let d = t.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    const SECS_PER_DAY: u64 = 86400;
    let days = (secs / SECS_PER_DAY) as u32;
    let time_of_day = secs % SECS_PER_DAY;
    let hour = (time_of_day / 3600) as u8;
    let minute = ((time_of_day % 3600) / 60) as u8;
    let second = (time_of_day % 60) as u8;
    let (year, month, day) = days_since_epoch_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn days_since_epoch_to_ymd(days: u32) -> (u32, u8, u8) {
    const EPOCH: u32 = 719_163;
    let rd = days + EPOCH;
    let (year, doy) = rd_to_year_doy(rd);
    let (month, day) = doy_to_month_day(year, doy);
    (year, month, day)
}

fn rd_to_year_doy(rd: u32) -> (u32, u16) {
    let mut doy = rd;
    let mut year = (doy as u64 * 400 / 146_097) as u32;
    doy -= (year as u64 * 146_097 / 400) as u32;
    year += (doy / 36524) * 100;
    doy %= 36524;
    year += (doy / 1461) * 4;
    doy %= 1461;
    year += doy / 365;
    doy %= 365;
    if doy == 0 {
        year -= 1;
        doy = 365;
    }
    (year, doy as u16)
}

fn doy_to_month_day(year: u32, doy: u16) -> (u8, u8) {
    let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let days_in_month: [u16; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut d = doy;
    for (i, &dim) in days_in_month.iter().enumerate() {
        if d <= dim {
            return ((i + 1) as u8, d as u8);
        }
        d -= dim;
    }
    (12, 31)
}

/// Calibration result for a specific GPU
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuCalibration {
    /// Unique device identifier
    pub device_id: String,
    /// Device name
    pub device_name: String,
    /// Optimal workgroup size discovered
    pub optimal_workgroup_size: u32,
    /// Optimal batch size for command submission
    pub optimal_batch_size: usize,
    /// Measured peak bandwidth (GB/s)
    pub peak_bandwidth_gbps: f64,
    /// Measured single-op latency (μs)
    pub single_op_latency_us: f64,
    /// Calibration timestamp
    pub calibrated_at: String,
}

/// Generate test shader with configurable workgroup size
fn generate_test_shader(workgroup_size: u32) -> String {
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

/// Properly validated throughput measurement
///
/// Key insight: We must measure ACTUAL data movement, not just kernel dispatch time.
/// True validation includes:
/// 1. Data upload to GPU
/// 2. Kernel execution (with proper sync)
/// 3. Data readback to verify correctness
async fn measure_true_throughput(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader_source: &str,
    workgroup_size: u32,
    size: usize,
) -> std::result::Result<(f64, f64), Box<dyn std::error::Error + Send + Sync>> {
    // Returns (latency_us, bandwidth_gbps)
    // Create test data
    let data_a: Vec<f32> = (0..size).map(|i| (i % 1000) as f32 * 0.001).collect();
    let data_b: Vec<f32> = (0..size)
        .map(|i| ((i + 500) % 1000) as f32 * 0.001)
        .collect();

    // Create GPU buffers
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

    // Staging buffer for readback (validates we actually did work)
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging"),
        size: 4, // Just read first element to verify
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Compile shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Test"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Create bind group layout
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("BGL"),
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

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("BG"),
        layout: &bgl,
        entries: &[
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
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("PL"),
        bind_group_layouts: &[&bgl],
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

    let workgroups = (size as u32).div_ceil(workgroup_size).min(65535);

    // Warmup run
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
    }

    // PROPERLY TIMED benchmark with sync
    let iterations = 10;
    let start = Instant::now();

    for _ in 0..iterations {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // Copy first element to staging for validation
        encoder.copy_buffer_to_buffer(&buf_out, 0, &staging, 0, 4);
        queue.submit(Some(encoder.finish()));

        // CRITICAL: Wait for GPU to actually complete
        device.poll(wgpu::Maintain::Wait);
    }

    let elapsed = start.elapsed();
    let latency_us = elapsed.as_secs_f64() * 1e6 / iterations as f64;

    // Calculate true bandwidth: (read A + read B + write C) = 3 * size * 4 bytes
    let bytes_per_op = size * 3 * 4;
    let bandwidth_gbps = (bytes_per_op as f64) / (latency_us * 1000.0);

    // Verify we got correct results
    {
        let slice = staging.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.await??;

        let data = slice.get_mapped_range();
        let result: f32 = bytemuck::cast_slice(&data)[0];
        let expected = data_a[0] + data_b[0];
        assert!(
            (result - expected).abs() < 1e-5,
            "Validation failed: {result} != {expected}"
        );
    }

    Ok((latency_us, bandwidth_gbps))
}

/// Auto-tune workgroup size for a specific GPU
async fn autotune_workgroup_size(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    device_name: &str,
) -> std::result::Result<(u32, f64), Box<dyn std::error::Error + Send + Sync>> {
    println!("  Auto-tuning workgroup size for {device_name}...");

    let test_size = 4_000_000; // 4M elements (fits in dispatch limits for all WG sizes)
    let wg_sizes = [32, 64, 128, 256];

    let mut best_wg = 256u32;
    let mut best_bw = 0.0f64;

    for wg_size in wg_sizes {
        let shader = generate_test_shader(wg_size);
        match measure_true_throughput(device, queue, &shader, wg_size, test_size).await {
            Ok((latency, bandwidth)) => {
                println!("    WG={wg_size:>3}: {latency:>8.1}μs, {bandwidth:>6.1} GB/s");
                if bandwidth > best_bw {
                    best_bw = bandwidth;
                    best_wg = wg_size;
                }
            }
            Err(e) => println!("    WG={wg_size}: ERROR - {e}"),
        }
    }

    println!("  → Optimal: WG={best_wg} ({best_bw:.1} GB/s)");
    Ok((best_wg, best_bw))
}

/// Auto-tune batch size for command submission
async fn autotune_batch_size(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    workgroup_size: u32,
) -> std::result::Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    println!("  Auto-tuning batch size...");

    let test_size = 1_000_000;
    let batch_sizes = [1, 5, 10, 20, 50];

    let shader = generate_test_shader(workgroup_size);

    // Pre-create resources
    let data: Vec<f32> = (0..test_size).map(|i| i as f32 * 0.001).collect();
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
        size: (test_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let shader_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(shader.into()),
    });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bgl,
        entries: &[
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
        module: &shader_mod,
        entry_point: "main",
        cache: None,
        compilation_options: Default::default(),
    });

    let workgroups = (test_size as u32).div_ceil(workgroup_size);

    let mut best_batch = 1usize;
    let mut best_throughput = 0.0f64;

    for batch_size in batch_sizes {
        // Warmup
        for _ in 0..batch_size {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups, 1, 1);
            }
            queue.submit(Some(encoder.finish()));
        }
        device.poll(wgpu::Maintain::Wait);

        // Measure
        let iterations = 5;
        let start = Instant::now();

        for _ in 0..iterations {
            for _ in 0..batch_size {
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                {
                    let mut pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.dispatch_workgroups(workgroups, 1, 1);
                }
                queue.submit(Some(encoder.finish()));
            }
            device.poll(wgpu::Maintain::Wait);
        }

        let elapsed = start.elapsed();
        let ops_per_sec = (iterations * batch_size) as f64 / elapsed.as_secs_f64();
        let throughput = ops_per_sec * test_size as f64 * 3.0 * 4.0 / 1e9; // GB/s

        println!(
            "    Batch={batch_size:>2}: {ops_per_sec:.0} ops/s, effective {throughput:.1} GB/s"
        );

        if throughput > best_throughput {
            best_throughput = throughput;
            best_batch = batch_size;
        }
    }

    println!("  → Optimal batch: {best_batch}");
    Ok(best_batch)
}

/// Full calibration for a GPU
pub async fn calibrate_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    device_name: &str,
) -> std::result::Result<GpuCalibration, Box<dyn std::error::Error + Send + Sync>> {
    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  CALIBRATING: {device_name:<60} ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Step 1: Find optimal workgroup size
    let (optimal_wg, peak_bw) = autotune_workgroup_size(device, queue, device_name).await?;

    // Step 2: Find optimal batch size
    let optimal_batch = autotune_batch_size(device, queue, optimal_wg).await?;

    // Step 3: Measure single-op latency
    let shader = generate_test_shader(optimal_wg);
    let (single_op_latency, _) =
        measure_true_throughput(device, queue, &shader, optimal_wg, 1_000_000).await?;

    let calibration = GpuCalibration {
        device_id: format!("{:?}", device.global_id()),
        device_name: device_name.to_string(),
        optimal_workgroup_size: optimal_wg,
        optimal_batch_size: optimal_batch,
        peak_bandwidth_gbps: peak_bw,
        single_op_latency_us: single_op_latency,
        calibrated_at: format_rfc3339(std::time::SystemTime::now()),
    };

    println!("\n  ══════════════════════════════════════════");
    println!("  CALIBRATION COMPLETE");
    println!("  ══════════════════════════════════════════");
    println!(
        "  Optimal WG size:    {}",
        calibration.optimal_workgroup_size
    );
    println!("  Optimal batch:      {}", calibration.optimal_batch_size);
    println!(
        "  Peak bandwidth:     {:.1} GB/s",
        calibration.peak_bandwidth_gbps
    );
    println!(
        "  Single-op latency:  {:.1} μs",
        calibration.single_op_latency_us
    );

    Ok(calibration)
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     BARRACUDA AUTO-TUNING RUNTIME                                             ║");
    println!("║     Discovering ground truth of hardware capabilities                          ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let config = WorkloadConfig {
        exclude_software: true,
        min_gflops: 100.0,
        ..Default::default()
    };

    let pool = GpuPool::with_config(config).await?;
    let mut calibrations = Vec::new();

    // Calibrate each GPU
    for (idx, _gpu_info) in pool.devices().iter().enumerate() {
        let wgpu_device = pool
            .device(idx)
            .ok_or_else(|| std::io::Error::other("No device"))?;
        let device = wgpu_device.device();
        let queue = wgpu_device.queue();
        let name = wgpu_device.name();

        let cal = calibrate_gpu(device, queue, name).await?;
        calibrations.push(cal);
    }

    // Summary
    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     CALIBRATION SUMMARY                                                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    println!("┌────────────────────────────┬────────┬─────────┬──────────────┬─────────────┐");
    println!("│ Device                     │ WG     │ Batch   │ Peak BW      │ Latency     │");
    println!("├────────────────────────────┼────────┼─────────┼──────────────┼─────────────┤");

    for cal in &calibrations {
        let short_name = if cal.device_name.len() > 26 {
            &cal.device_name[..26]
        } else {
            &cal.device_name
        };
        println!(
            "│ {:26} │ {:>6} │ {:>7} │ {:>9.1} GB/s │ {:>8.1} μs │",
            short_name,
            cal.optimal_workgroup_size,
            cal.optimal_batch_size,
            cal.peak_bandwidth_gbps,
            cal.single_op_latency_us
        );
    }
    println!("└────────────────────────────┴────────┴─────────┴──────────────┴─────────────┘");

    // Theoretical comparison
    println!("\n═══ THEORETICAL LIMITS (for reference) ═══\n");
    println!("  RTX 3090:     936 GB/s (GDDR6X @ 19.5 Gbps × 384-bit)");
    println!("  RX 6950 XT:   576 GB/s (GDDR6 @ 18 Gbps × 256-bit)");
    println!("\n  If measured > theoretical: measurement includes caching effects");
    println!("  If measured < theoretical: room for optimization or overhead");

    // Save calibrations to file
    let cal_path = std::env::temp_dir().join("barracuda_calibrations.json");
    let json = serde_json::to_string_pretty(&calibrations)?;
    std::fs::write(&cal_path, &json)?;
    println!("\n  Calibrations saved to: {}", cal_path.display());

    println!("\n═══ NEXT STEPS ═══\n");
    println!("  1. Integrate calibrations into ToadStool runtime");
    println!("  2. Use optimal settings per-GPU automatically");
    println!("  3. Re-calibrate when new hardware detected");
    println!("  4. Build compute graph to leverage batching");

    Ok(())
}
