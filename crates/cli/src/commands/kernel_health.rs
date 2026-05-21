// SPDX-License-Identifier: AGPL-3.0-or-later
//! `toadstool kernel-health` — kernel build environment health check CLI.
//!
//! Runs 3-layer detection (autoconf freshness, struct layout probe,
//! reference cross-check) and prints a human-readable report or JSON.

use toadstool_cylinder::vfio::kernel_health;

use crate::Result;

pub async fn execute_kernel_health(format: &str, repair: bool) -> Result<()> {
    let report = kernel_health::full_kernel_health_check().map_err(|e| {
        crate::CliError::Other(format!("kernel health check failed: {e}"))
    })?;

    if format == "json" {
        let mut output = serde_json::to_value(&report).unwrap_or_default();

        if repair && !report.layout_matches {
            let repair_result =
                kernel_health::repair_autoconf(kernel_health::RepairStrategy::PackageRestore);
            let repair_json = match repair_result {
                Ok(path) => serde_json::json!({
                    "success": true,
                    "restored_path": path.display().to_string(),
                }),
                Err(e) => serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                }),
            };
            if let Some(obj) = output.as_object_mut() {
                obj.insert("repair".into(), repair_json);
            }
        }

        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
        return Ok(());
    }

    // Text format
    println!("Kernel Build Environment Health Check");
    println!("=====================================\n");

    // Layer 1
    let freshness_icon = if report.autoconf_fresh { "PASS" } else { "WARN" };
    println!("Layer 1 — autoconf.h Freshness: {freshness_icon}");
    if report.autoconf_age_delta_secs <= 0 {
        println!(
            "  autoconf.h is {}s older than kernel image (expected)",
            -report.autoconf_age_delta_secs
        );
    } else {
        println!(
            "  autoconf.h is {}s NEWER than kernel image (suspicious!)",
            report.autoconf_age_delta_secs
        );
    }

    // Layer 2
    println!();
    if let (Some(init), Some(exit)) = (report.struct_module_init_offset, report.struct_module_exit_offset) {
        println!("Layer 2 — Struct Module Probe:");
        println!("  init offset: 0x{init:x}");
        println!("  exit offset: 0x{exit:x}");
    } else {
        println!("Layer 2 — Struct Module Probe: UNAVAILABLE");
        println!("  (probe compilation failed — missing headers or toolchain)");
    }

    // Layer 3
    println!();
    if let (Some(init), Some(exit)) = (report.reference_init_offset, report.reference_exit_offset) {
        println!("Layer 3 — Reference Module Cross-Check:");
        println!("  init offset: 0x{init:x}");
        println!("  exit offset: 0x{exit:x}");
    } else {
        println!("Layer 3 — Reference Module Cross-Check: UNAVAILABLE");
        println!("  (no reference .ko found on this system)");
    }

    // Verdict
    println!();
    let verdict = if report.layout_matches { "HEALTHY" } else { "UNHEALTHY" };
    println!("Verdict: {verdict}");
    println!("  {}", report.diagnosis);

    // Repair if requested
    if repair && !report.layout_matches {
        println!();
        println!("Attempting repair via .deb cache...");
        match kernel_health::repair_autoconf(kernel_health::RepairStrategy::PackageRestore) {
            Ok(path) => {
                println!("  Restored: {}", path.display());
                println!("  Re-run `toadstool kernel-health` to verify.");
            }
            Err(e) => {
                println!("  Repair failed: {e}");
                println!("  Try: sudo apt-get install --reinstall linux-headers-$(uname -r)");
            }
        }
    }

    Ok(())
}
