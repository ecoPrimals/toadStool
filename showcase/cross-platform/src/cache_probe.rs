//! Cache Probing Microbenchmark
//!
//! Discovers memory hierarchy via bandwidth microbenchmarks.
//! This is NOT hardcoded values — the silicon tells us what it can do.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p cross-platform-showcase --bin cache_probe --release
//! ```

use barracuda::device::{CacheAwareTiler, SubstrateMemoryHierarchy, WgpuDevice};
use std::time::Instant;

/// Run cache probing benchmarks on all available GPUs
pub async fn run_cache_probe() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║         CACHE PROBING MICROBENCHMARK - RUNTIME DISCOVERY             ║");
    println!("║                  The Silicon Tells Us What It Can Do                 ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝\n");

    // Enumerate all GPUs
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapters: Vec<wgpu::Adapter> = instance.enumerate_adapters(wgpu::Backends::all());

    if adapters.is_empty() {
        println!("No GPU adapters found!");
        return Ok(());
    }

    println!("Found {} adapter(s)\n", adapters.len());

    for adapter in &adapters {
        let info = adapter.get_info();

        // Skip CPU adapters for this benchmark
        if info.device_type == wgpu::DeviceType::Cpu {
            println!("─── Skipping CPU adapter: {} ───\n", info.name);
            continue;
        }

        println!("┌─────────────────────────────────────────────────────────────────────┐");
        println!("│ Probing: {} ({:?})", info.name, info.device_type);
        println!("│ Driver: {} {}", info.driver, info.driver_info);
        println!("└─────────────────────────────────────────────────────────────────────┘\n");

        // Create device
        let (device, queue) = match adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("cache_probe"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
        {
            Ok(d) => d,
            Err(e) => {
                println!("  ⚠ Failed to create device: {}\n", e);
                continue;
            }
        };

        let wgpu_device = WgpuDevice::from_existing(
            std::sync::Arc::new(device),
            std::sync::Arc::new(queue),
            info.clone(),
        );

        // Run probing with timing
        println!("  Running bandwidth probes...");
        let probe_start = Instant::now();
        let hierarchy = SubstrateMemoryHierarchy::probe(&wgpu_device).await;
        let probe_time = probe_start.elapsed();

        println!("  ✓ Probing completed in {:.2?}\n", probe_time);

        // Report discovered hierarchy
        print_hierarchy(&hierarchy);

        // Run tiling analysis
        print_tiling_analysis(&hierarchy);

        // Validate against estimate (for comparison)
        let estimate = SubstrateMemoryHierarchy::estimate(&wgpu_device);
        print_probe_vs_estimate(&hierarchy, &estimate);

        println!();
    }

    // Summary recommendations
    print_recommendations();

    Ok(())
}

fn print_hierarchy(hierarchy: &SubstrateMemoryHierarchy) {
    println!("  ╭────────────────────────────────────────────────────────────────────╮");
    println!(
        "  │ DISCOVERED MEMORY HIERARCHY: {}",
        hierarchy.substrate_name
    );
    println!("  │ Substrate Type: {:?}", hierarchy.substrate_type);
    println!("  ├────────────────────────────────────────────────────────────────────┤");

    if hierarchy.cache_levels.is_empty() {
        println!("  │ No cache levels detected (probing may need larger ranges)");
    } else {
        for (i, cache) in hierarchy.cache_levels.iter().enumerate() {
            let size_mb = cache.size_bytes as f64 / (1024.0 * 1024.0);
            println!(
                "  │ {} {:>2}: {:>8.2} MB  @ {:>7.1} GB/s  ({})",
                if i == 0 { "└" } else { "├" },
                cache.name,
                size_mb,
                cache.bandwidth_gbs,
                if cache.shared { "shared" } else { "private" }
            );
        }
    }

    let vram_gb = hierarchy.main_memory.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    println!("  ├────────────────────────────────────────────────────────────────────┤");
    println!(
        "  │ VRAM: {:>6.2} GB  @ {:>7.1} GB/s",
        vram_gb, hierarchy.main_memory.bandwidth_gbs
    );

    let optimal_mb = hierarchy.optimal_tile_bytes as f64 / (1024.0 * 1024.0);
    println!("  ├────────────────────────────────────────────────────────────────────┤");
    println!("  │ Optimal Tile Size: {:>6.2} MB", optimal_mb);
    println!("  ╰────────────────────────────────────────────────────────────────────╯\n");
}

fn print_tiling_analysis(hierarchy: &SubstrateMemoryHierarchy) {
    println!("  TILING ANALYSIS (for common tensor sizes):");
    println!("  ──────────────────────────────────────────────");

    let tiler = CacheAwareTiler::new(hierarchy.clone());

    // Common tensor sizes
    let sizes = [
        (1_000_000, "1M elements (4 MB)"),
        (10_000_000, "10M elements (40 MB)"),
        (50_000_000, "50M elements (200 MB)"),
        (100_000_000, "100M elements (400 MB)"),
        (250_000_000, "250M elements (1 GB)"),
    ];

    for (elements, desc) in &sizes {
        let total_bytes = (*elements as u64) * 4; // f32
        let config = tiler.optimal_tile_size(total_bytes, 4, 3.0);

        let tile_mb = config.tile_bytes as f64 / (1024.0 * 1024.0);
        println!(
            "    {:>25}: {} tiles @ {:.1} MB → {} cache, {:.0} GB/s expected",
            desc, config.num_tiles, tile_mb, config.target_cache, config.expected_bandwidth_gbs
        );
    }
    println!();
}

fn print_probe_vs_estimate(
    probed: &SubstrateMemoryHierarchy,
    estimated: &SubstrateMemoryHierarchy,
) {
    println!("  PROBE vs ESTIMATE COMPARISON:");
    println!("  ─────────────────────────────");

    let probed_cache = probed.total_cache_bytes();
    let estimated_cache = estimated.total_cache_bytes();

    let probed_mb = probed_cache as f64 / (1024.0 * 1024.0);
    let estimated_mb = estimated_cache as f64 / (1024.0 * 1024.0);

    let diff = if probed_cache > estimated_cache {
        (probed_cache as f64 / estimated_cache as f64) - 1.0
    } else {
        (estimated_cache as f64 / probed_cache as f64) - 1.0
    };

    let comparison = if probed_cache > estimated_cache {
        format!("+{:.0}% larger than estimate", diff * 100.0)
    } else if probed_cache < estimated_cache {
        format!("-{:.0}% smaller than estimate", diff * 100.0)
    } else {
        "matches estimate".to_string()
    };

    println!("    Probed cache:    {:>8.2} MB", probed_mb);
    println!(
        "    Estimated cache: {:>8.2} MB  ({})",
        estimated_mb, comparison
    );
    println!(
        "    Probed VRAM BW:  {:>8.1} GB/s",
        probed.main_memory.bandwidth_gbs
    );
    println!(
        "    Estimated BW:    {:>8.1} GB/s",
        estimated.main_memory.bandwidth_gbs
    );
    println!();
}

fn print_recommendations() {
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                          RECOMMENDATIONS                              ║");
    println!("╠══════════════════════════════════════════════════════════════════════╣");
    println!("║ 1. Use probed values for production workloads (more accurate)        ║");
    println!("║ 2. Tile large tensors to fit in discovered cache                     ║");
    println!("║ 3. For repeated operations, data should fit in largest cache         ║");
    println!("║ 4. High bandwidth variance between sizes indicates cache boundaries  ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    if let Err(e) = run_cache_probe().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
