// SPDX-License-Identifier: AGPL-3.0-only
use super::types::DoctorReport;

pub(crate) fn print_text_report(report: &DoctorReport) {
    println!("\n=== ToadStool Doctor Report ===\n");

    println!("HARDWARE:");
    println!("  CPU Cores: {}", report.hardware.cpu_cores);
    if !report.hardware.cpu_features.is_empty() {
        println!(
            "  CPU Features: {}",
            report.hardware.cpu_features.join(", ")
        );
    }
    println!(
        "  GPU: {}",
        if report.hardware.gpu_detected {
            report.hardware.gpu_info.as_deref().unwrap_or("Detected")
        } else {
            "Not detected"
        }
    );
    println!(
        "  NPU: {}",
        if report.hardware.npu_detected {
            report.hardware.npu_info.as_deref().unwrap_or("Detected")
        } else {
            "Not detected"
        }
    );
    println!("  Memory: {} MB", report.hardware.memory_total_mb);
    for issue in &report.hardware.issues {
        println!("  [WARN] {issue}");
    }

    println!("\nECOSYSTEM:");
    println!(
        "  biomeOS Directory: {} ({})",
        report.ecosystem.biomeos_dir,
        if report.ecosystem.biomeos_dir_exists {
            "exists"
        } else {
            "MISSING"
        }
    );
    if !report.ecosystem.sockets_found.is_empty() {
        println!(
            "  Sockets Found: {}",
            report.ecosystem.sockets_found.join(", ")
        );
    }
    println!("  Primal Status:");
    for primal in &report.ecosystem.primals_reachable {
        let status = if primal.reachable {
            "OK"
        } else if primal.socket_exists {
            "UNREACHABLE"
        } else {
            "NOT RUNNING"
        };
        println!("    - {}: {status}", primal.name);
    }
    for issue in &report.ecosystem.issues {
        println!("  [ERROR] {issue}");
    }

    println!("\nCONFIGURATION:");
    println!(
        "  Config File: {}",
        report
            .config
            .config_file_path
            .as_deref()
            .unwrap_or("Not found")
    );
    if !report.config.env_vars_set.is_empty() {
        println!(
            "  Environment Variables: {}",
            report.config.env_vars_set.join(", ")
        );
    }
    for issue in &report.config.issues {
        println!("  [ERROR] {issue}");
    }

    println!("\n=== SUMMARY ===");
    println!(
        "  Checks: {} total, {} passed, {} warnings, {} errors",
        report.summary.total_checks,
        report.summary.passed,
        report.summary.warnings,
        report.summary.errors
    );
    println!("  Status: {}", report.summary.overall_status);
    println!();
}
