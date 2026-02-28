//! Multi-GPU Debug Benchmark
//!
//! Debug why pipeline caching breaks across multiple GPUs

use barracuda::device::pipeline_cache::{BindGroupLayoutSignature, GLOBAL_CACHE};
use barracuda::multi_gpu::{GpuPool, WorkloadConfig};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║     MULTI-GPU DEBUG                                                           ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let config = WorkloadConfig {
        exclude_software: true,
        min_gflops: 100.0,
        ..Default::default()
    };

    let pool = GpuPool::with_config(config).await?;

    println!("Found {} GPUs:\n", pool.devices().len());

    for idx in 0..pool.devices().len() {
        let device = pool
            .device(idx)
            .ok_or_else(|| std::io::Error::other("No device"))?;
        let wgpu_device = device.device();
        let device_id = wgpu_device.global_id();

        println!("  GPU {}: {}", idx, device.name());
        println!("    Device ID: {:?}", device_id);

        // Create a layout for this device
        let layout_sig = BindGroupLayoutSignature::elementwise_binary();
        let adapter_info = device.adapter_info();
        println!("    Creating/getting bind group layout...");
        let layout =
            GLOBAL_CACHE.get_or_create_layout(wgpu_device, adapter_info, layout_sig, Some("Test"));
        println!("    Layout created successfully");

        // Try to create a bind group with this layout
        println!("    Creating bind group...");
        let test_buffer = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Test"),
            size: 1024,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let bind_group_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            wgpu_device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Test"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: test_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: test_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: test_buffer.as_entire_binding(),
                    },
                ],
            })
        }));

        match bind_group_result {
            Ok(_) => println!("    ✅ Bind group created successfully\n"),
            Err(_) => println!("    ❌ Bind group creation FAILED\n"),
        }
    }

    println!("Cache stats:");
    let stats = GLOBAL_CACHE.stats();
    println!("  Shaders:   {}", stats.shaders);
    println!("  Layouts:   {}", stats.layouts);
    println!("  Pipelines: {}", stats.pipelines);

    println!("\nExpected layouts: {} (one per GPU)", pool.devices().len());
    if stats.layouts == pool.devices().len() {
        println!("✅ Each GPU has its own layout (device ID keying works)");
    } else {
        println!("❌ Layout count mismatch - device ID keying may be broken");
    }

    Ok(())
}
