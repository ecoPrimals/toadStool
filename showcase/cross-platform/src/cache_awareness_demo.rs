//! Cache Awareness Demo
//!
//! Demonstrates ToadStool's runtime cache discovery and intelligent workload tiling.
//! NO VENDOR HARDCODING — the silicon tells us what it can do.

use barracuda::device::{CacheAwareTiler, CacheResidency, SubstrateMemoryHierarchy, WgpuDevice};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  ToadStool Universal Cache Awareness                                         ║");
    println!("║  Runtime discovery, not vendor hardcoding                                    ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Discover actual hardware
    let device = WgpuDevice::new().await?;
    let hierarchy = SubstrateMemoryHierarchy::discover(&device);

    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("  Discovered Cache Hierarchy: {}", device.name());
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    println!("  Type: {:?}", hierarchy.substrate_type);
    println!(
        "  Discovery method: {}\n",
        if hierarchy.probed {
            "probed"
        } else {
            "estimated"
        }
    );

    println!("  ┌────────────────────┬────────────────┬────────────────┐");
    println!("  │ Level              │ Size           │ BW (GB/s)      │");
    println!("  ├────────────────────┼────────────────┼────────────────┤");

    for cache in &hierarchy.cache_levels {
        let size_str = if cache.size_bytes >= 1024 * 1024 {
            format!("{} MB", cache.size_bytes / 1024 / 1024)
        } else {
            format!("{} KB", cache.size_bytes / 1024)
        };
        println!(
            "  │ {:18} │ {:14} │ {:14.0} │",
            cache.name, size_str, cache.bandwidth_gbs
        );
    }

    println!("  ├────────────────────┼────────────────┼────────────────┤");
    println!(
        "  │ DRAM/VRAM          │ {:14} │ {:14.0} │",
        format!(
            "{} GB",
            hierarchy.main_memory.size_bytes / 1024 / 1024 / 1024
        ),
        hierarchy.main_memory.bandwidth_gbs
    );
    println!("  └────────────────────┴────────────────┴────────────────┘\n");

    println!(
        "  Optimal tile size: {} MB",
        hierarchy.optimal_tile_bytes / 1024 / 1024
    );

    println!("\n══════════════════════════════════════════════════════════════════════════════");
    println!("  Cache Residency Analysis");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    let tiler = CacheAwareTiler::new(hierarchy.clone());

    let test_sizes = [
        (1_000_000, "1M elements (4 MB)"),
        (10_000_000, "10M elements (40 MB)"),
        (50_000_000, "50M elements (200 MB)"),
        (100_000_000, "100M elements (400 MB)"),
    ];

    println!("  ┌────────────────────────────────┬────────────────┬────────────────┐");
    println!("  │ Workload                       │ Status         │ Expected BW    │");
    println!("  ├────────────────────────────────┼────────────────┼────────────────┤");

    for (elements, desc) in &test_sizes {
        let data_bytes = (*elements as u64) * 4; // f32
        let (status, bw) = match tiler.is_cache_resident(data_bytes) {
            CacheResidency::Resident {
                cache_level,
                utilization,
            } => (
                format!("✅ {} ({:.0}%)", cache_level, utilization * 100.0),
                tiler.predict_bandwidth(data_bytes),
            ),
            CacheResidency::DramBound { .. } => (
                "⚠️ DRAM-bound".to_string(),
                tiler.predict_bandwidth(data_bytes),
            ),
        };
        println!("  │ {desc:30} │ {status:14} │ {bw:12.0} │");
    }

    println!("  └────────────────────────────────┴────────────────┴────────────────┘");

    println!("\n══════════════════════════════════════════════════════════════════════════════");
    println!("  Intelligent Tiling: 1 GB workload (A * B + C pattern)");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    let config = tiler.optimal_tile_size(
        1024 * 1024 * 1024, // 1 GB
        4,                  // f32
        3.0,                // A * B + C = 3x data reuse
    );

    println!("  Total data:      1 GB");
    println!("  Tile size:       {} MB", config.tile_bytes / 1024 / 1024);
    println!("  Number of tiles: {}", config.num_tiles);
    println!("  Target cache:    {}", config.target_cache);
    println!(
        "  Expected BW:     {:.0} GB/s",
        config.expected_bandwidth_gbs
    );

    println!("\n══════════════════════════════════════════════════════════════════════════════");
    println!("  KEY PRINCIPLES");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    println!("  1. RUNTIME DISCOVERY, NOT VENDOR HARDCODING");
    println!("     - The silicon tells us what it can do");
    println!("     - No \"if AMD then 128MB\" patterns");
    println!("     - Bandwidth probing finds actual cache boundaries\n");

    println!("  2. UNIVERSAL MODEL");
    println!("     - Same code works for CPU L3, GPU L2, Infinity Cache, Apple SLC");
    println!("     - Substrate type affects defaults, not behavior\n");

    println!("  3. INTELLIGENT TILING");
    println!("     - Workloads automatically tiled to fit available cache");
    println!("     - Fewer tiles = less dispatch overhead = faster execution\n");

    println!("  4. WHY >100% THEORETICAL BANDWIDTH HAPPENS");
    println!("     - Data fits in cache → cache bandwidth, not DRAM");
    println!("     - Not magic, just intelligent cache utilization");

    Ok(())
}
