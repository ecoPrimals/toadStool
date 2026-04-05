// SPDX-License-Identifier: AGPL-3.0-or-later
//! CLI tool: apply a PMU init recipe to a GPU via BAR0 MMIO.
//!
//! Usage:
//!   nvpmu-apply \<bdf\> \<recipe.json\> [--dry-run]
//!
//! Example:
//!   sudo nvpmu-apply 0000:65:00.0 `gv100_recipe.json`
//!   nvpmu-apply 0000:65:00.0 `gv100_recipe.json` --dry-run

use nvpmu::bar0::Bar0Access;
use nvpmu::init::{InitResult, apply_recipe};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <bdf> <recipe.json> [--dry-run]", args[0]);
        eprintln!();
        eprintln!("Apply a PMU init recipe to a GPU via BAR0 MMIO.");
        eprintln!("  bdf:          PCI bus:device.function (e.g. 0000:65:00.0)");
        eprintln!("  recipe.json:  Init recipe from hw-learn distiller");
        eprintln!("  --dry-run:    Print what would be done without writing");
        eprintln!();
        eprintln!("WARNING: This writes directly to GPU registers. Requires root.");
        std::process::exit(1);
    }

    let bdf = &args[1];
    let recipe_path = &args[2];
    let dry_run = args.iter().any(|a| a == "--dry-run");

    let recipe_json = std::fs::read_to_string(recipe_path).unwrap_or_else(|e| {
        eprintln!("Error reading recipe: {e}");
        std::process::exit(1);
    });

    if dry_run {
        eprintln!("DRY RUN: parsing recipe, not writing to hardware");
        let recipe: serde_json::Value = serde_json::from_str(&recipe_json).unwrap_or_else(|e| {
            eprintln!("Error parsing recipe JSON: {e}");
            std::process::exit(1);
        });
        if let Some(steps) = recipe.get("steps").and_then(|s| s.as_array()) {
            for step in steps {
                let offset = step
                    .get("offset")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let value = step
                    .get("value")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let class = step
                    .get("class")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                eprintln!("  WRITE {offset:#010x} = {value:#010x}  ({class})");
            }
            eprintln!("Would apply {} register writes to {bdf}", steps.len());
        }
        return;
    }

    eprintln!("Opening BAR0 for {bdf}...");
    let mut bar0 = Bar0Access::open(bdf).unwrap_or_else(|e| {
        eprintln!("Error opening BAR0: {e}");
        eprintln!("Hint: requires root or appropriate PCI sysfs permissions");
        std::process::exit(1);
    });
    eprintln!("BAR0 mapped: {} bytes", bar0.size());

    eprintln!("Applying recipe...");
    let result: InitResult = apply_recipe(&recipe_json, &mut bar0).unwrap_or_else(|e| {
        eprintln!("Error applying recipe: {e}");
        std::process::exit(1);
    });

    eprintln!("Steps applied: {}", result.steps_applied);
    eprintln!("Steps failed:  {}", result.steps_failed);
    eprintln!("Verify passed: {}", result.verify_passed);
    eprintln!("Verify failed: {}", result.verify_failed);

    if result.success {
        eprintln!("SUCCESS: PMU init recipe applied to {bdf}");
    } else {
        eprintln!("FAILURE: recipe application had errors");
        std::process::exit(1);
    }
}
