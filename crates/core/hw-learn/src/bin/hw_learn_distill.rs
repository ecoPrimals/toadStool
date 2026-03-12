// SPDX-License-Identifier: AGPL-3.0-only
//! CLI tool: distill a PMU init recipe from two MMIO traces.
//!
//! Usage:
//!   hw-learn-distill <chip> <baseline.txt> <compute.txt> [output.json]
//!
//! Example:
//!   hw-learn-distill gv100 baseline.txt compute.txt gv100_recipe.json

use hw_learn::distiller::{build_recipe, diff_traces};
use hw_learn::knowledge::RecipeStore;
use hw_learn::observer::MmioTrace;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <chip> <baseline.txt> <compute.txt> [output.json]", args[0]);
        eprintln!();
        eprintln!("Distill a PMU init recipe from two MMIO traces.");
        eprintln!("  chip:         GPU chip codename (e.g. gv100)");
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
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{chip}.json"));

    eprintln!("Loading baseline trace: {}", baseline_path.display());
    let baseline = MmioTrace::from_file(baseline_path).unwrap_or_else(|e| {
        eprintln!("Error reading baseline: {e}");
        std::process::exit(1);
    });
    eprintln!("  {} accesses, base: {:?}", baseline.len(), baseline.base_address);

    eprintln!("Loading compute trace: {}", compute_path.display());
    let compute = MmioTrace::from_file(compute_path).unwrap_or_else(|e| {
        eprintln!("Error reading compute trace: {e}");
        std::process::exit(1);
    });
    eprintln!("  {} accesses, base: {:?}", compute.len(), compute.base_address);

    let base_addr = compute.base_address.unwrap_or(0);
    let diff = diff_traces(&baseline, &compute);
    eprintln!("Diff: {} compute-specific writes", diff.len());

    let recipe = build_recipe(chip, &diff, base_addr);
    eprintln!("Recipe: {} steps", recipe.len());

    for step in &recipe.steps {
        eprintln!(
            "  {:#010x} = {:#010x}  ({:?}{})",
            step.offset,
            step.value,
            step.class,
            step.delay_us
                .map(|d| format!(", delay {d}µs"))
                .unwrap_or_default()
        );
    }

    let json = recipe.to_json().unwrap_or_else(|e| {
        eprintln!("Error serializing recipe: {e}");
        std::process::exit(1);
    });

    std::fs::write(&output_path, &json).unwrap_or_else(|e| {
        eprintln!("Error writing {output_path}: {e}");
        std::process::exit(1);
    });
    eprintln!("Recipe written to {output_path}");

    // Also save to the default recipe store
    let store = RecipeStore::default_location();
    match store.save(&recipe) {
        Ok(p) => eprintln!("Also saved to recipe store: {}", p.display()),
        Err(e) => eprintln!("Warning: could not save to recipe store: {e}"),
    }
}
