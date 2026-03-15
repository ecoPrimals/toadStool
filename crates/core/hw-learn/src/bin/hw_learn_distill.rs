// SPDX-License-Identifier: AGPL-3.0-only
//! CLI tool: distill a PMU init recipe from two MMIO traces.
//!
//! Usage:
//!   hw-learn-distill <baseline.txt> <compute.txt> [output.json]
//!
//! The target GPU architecture is inferred from the compute trace.
//! The baseline trace (no compute) is diffed against the compute trace
//! to isolate compute-specific register writes.

use hw_learn::distiller::{GpuArch, RecipeDistiller, Vendor};
use hw_learn::knowledge::{export_recipe, KnowledgeStore};
use hw_learn::observer::{GpuSelector, ObserveConfig, TraceMode, TraceObserver};
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        eprintln!(
            "Usage: {} <chip> <baseline.txt> <compute.txt> [output.json]",
            args[0]
        );
        eprintln!();
        eprintln!("Distill a PMU init recipe from two MMIO traces.");
        eprintln!("  chip:         GPU chip codename (e.g. gv100, ad104)");
        eprintln!("  baseline.txt: MMIO trace without compute init");
        eprintln!("  compute.txt:  MMIO trace with compute init");
        eprintln!("  output.json:  Output recipe file (default: <chip>.json)");
        std::process::exit(1);
    }

    let chip = &args[1];
    let baseline_path = Path::new(&args[2]);
    let compute_path = Path::new(&args[3]);
    let output_path = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| format!("{chip}.json"));

    let baseline_config = ObserveConfig {
        mode: TraceMode::MmioTrace,
        trace_path: Some(baseline_path.to_path_buf()),
        gpu_selector: GpuSelector::Auto,
        trigger_compute: false,
    };
    let compute_config = ObserveConfig {
        mode: TraceMode::MmioTrace,
        trace_path: Some(compute_path.to_path_buf()),
        gpu_selector: GpuSelector::Auto,
        trigger_compute: false,
    };

    eprintln!("Loading baseline trace: {}", baseline_path.display());
    let baseline = TraceObserver::observe(&baseline_config).unwrap_or_else(|e| {
        eprintln!("Error reading baseline: {e}");
        std::process::exit(1);
    });
    eprintln!("  {} events", baseline.events.len());

    eprintln!("Loading compute trace: {}", compute_path.display());
    let compute = TraceObserver::observe(&compute_config).unwrap_or_else(|e| {
        eprintln!("Error reading compute trace: {e}");
        std::process::exit(1);
    });
    eprintln!("  {} events", compute.events.len());

    let target_arch = GpuArch {
        vendor: infer_vendor(chip),
        generation: String::new(),
        chip: chip.clone(),
        compute_class: String::new(),
    };
    let recipe = RecipeDistiller::distill(&compute, Some(&baseline), target_arch);
    eprintln!("Recipe: {} steps", recipe.steps.len());

    let json = export_recipe(&recipe).unwrap_or_else(|e| {
        eprintln!("Error serializing recipe: {e}");
        std::process::exit(1);
    });

    std::fs::write(&output_path, &json).unwrap_or_else(|e| {
        eprintln!("Error writing {output_path}: {e}");
        std::process::exit(1);
    });
    eprintln!("Recipe written to {output_path}");

    let store_dir = PathBuf::from("hw-learn-recipes");
    match KnowledgeStore::open(&store_dir) {
        Ok(mut store) => match store.store(&recipe) {
            Ok(id) => eprintln!("Also saved to knowledge store: {id}"),
            Err(e) => eprintln!("Warning: could not save to knowledge store: {e}"),
        },
        Err(e) => eprintln!("Warning: could not open knowledge store: {e}"),
    }
}

fn infer_vendor(chip: &str) -> Vendor {
    let lower = chip.to_lowercase();
    if lower.starts_with("gv")
        || lower.starts_with("ga")
        || lower.starts_with("ad")
        || lower.starts_with("tu")
    {
        Vendor::Nvidia
    } else if lower.starts_with("navi") || lower.starts_with("gfx") || lower.starts_with("rdna") {
        Vendor::Amd
    } else {
        Vendor::Intel
    }
}
