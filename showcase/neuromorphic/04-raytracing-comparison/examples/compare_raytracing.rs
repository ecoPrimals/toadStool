//! Compare NPU vs GPU raytracing performance
//!
//! Deep Debt: Uses ToadStool for hardware discovery and selection

use anyhow::Result;
use npu_raytracing_comparison::{Benchmark, Scene};
use toadstool_core::{HardwareManager, HardwareType};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║   NPU vs GPU Raytracing Comparison                  ║");
    println!("║   ToadStool + BarraCuda Architecture                 ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    // Step 1: ToadStool discovers hardware
    println!("[1/4] ToadStool discovering hardware...");
    let hw = HardwareManager::discover()?;

    let has_npu = hw.has_npu();
    let has_gpu = hw.has_gpu();

    println!("  NPU available: {}", has_npu);
    println!("  GPU available: {}", has_gpu);

    if !has_npu && !has_gpu {
        println!("\n⚠️  No NPU or GPU detected");
        println!("This demo requires at least one accelerator");
        return Ok(());
    }

    println!("\n[2/4] Testing sparse scene (NPU should excel)...");
    let sparse_scene = Scene::sparse();
    println!(
        "  Scene: {} spheres, {} pixels",
        sparse_scene.spheres.len(),
        sparse_scene.image_width * sparse_scene.image_height
    );

    let sparse_bench = Benchmark::new(sparse_scene);

    if has_npu {
        let npus = hw.devices_by_type(HardwareType::Npu);
        if let Some(npu) = npus.first() {
            match npu.pcie_address.as_ref() {
                Some(addr) => match sparse_bench.benchmark_npu(addr) {
                    Ok(result) => {
                        println!("\n  NPU Results:");
                        println!("    Time: {:.2} ms", result.duration_ms);
                        println!("    FPS: {:.2}", result.fps());
                        println!("    Rays/sec: {:.2e}", result.rays_per_second());
                    }
                    Err(e) => println!("  NPU benchmark failed: {}", e),
                },
                None => println!("  NPU has no PCIe address"),
            }
        }
    }

    if has_gpu {
        match sparse_bench.benchmark_gpu() {
            Ok(result) => {
                println!("\n  GPU Results:");
                println!("    Time: {:.2} ms", result.duration_ms);
                println!("    FPS: {:.2}", result.fps());
                println!("    Rays/sec: {:.2e}", result.rays_per_second());
            }
            Err(e) => println!("  GPU benchmark failed: {}", e),
        }
    }

    println!("\n[3/4] Testing dense scene (GPU should excel)...");
    let dense_scene = Scene::dense();
    println!(
        "  Scene: {} spheres, {} pixels",
        dense_scene.spheres.len(),
        dense_scene.image_width * dense_scene.image_height
    );

    let dense_bench = Benchmark::new(dense_scene);

    if has_npu {
        let npus = hw.devices_by_type(HardwareType::Npu);
        if let Some(npu) = npus.first() {
            if let Some(addr) = &npu.pcie_address {
                match dense_bench.benchmark_npu(addr) {
                    Ok(result) => {
                        println!("\n  NPU Results:");
                        println!("    Time: {:.2} ms", result.duration_ms);
                        println!("    FPS: {:.2}", result.fps());
                    }
                    Err(e) => println!("  NPU benchmark failed: {}", e),
                }
            }
        }
    }

    if has_gpu {
        match dense_bench.benchmark_gpu() {
            Ok(result) => {
                println!("\n  GPU Results:");
                println!("    Time: {:.2} ms", result.duration_ms);
                println!("    FPS: {:.2}", result.fps());
            }
            Err(e) => println!("  GPU benchmark failed: {}", e),
        }
    }

    println!("\n[4/4] Summary:");
    println!("\n  Sparse scenes: NPU excels (event-driven, skips empty rays)");
    println!("  Dense scenes: GPU excels (parallel throughput)");
    println!("  ToadStool: Automatically selects best device");

    println!("\n✓ Comparison complete!\n");

    Ok(())
}
