// SPDX-License-Identifier: AGPL-3.0-only
//! Doctor command - System health check and diagnostics
//!
//! **UniBin Standard Compliant**: Diagnose ToadStool installation, runtime,
//! and ecosystem connectivity.
#![allow(deprecated)]
//!
//! # Checks Performed
//!
//! - Hardware detection (GPU, NPU, CPU capabilities)
//! - Ecosystem connectivity (primal socket discovery)
//! - Configuration validity
//! - Runtime dependencies

mod checks;
mod display;
mod types;

#[cfg(test)]
mod tests;

use crate::Result;

pub use types::{
    ConfigReport, DoctorReport, EcosystemReport, HardwareReport, PrimalStatus, Summary,
};

use checks::{check_config_health, check_ecosystem_health, check_hardware_health};
use display::print_text_report;

/// Run doctor diagnostics
pub async fn run_doctor(
    check_hardware: bool,
    check_ecosystem: bool,
    check_config: bool,
    format: &str,
    _fix: bool,
) -> Result<()> {
    let check_all = !check_hardware && !check_ecosystem && !check_config;

    let mut total_checks = 0;
    let mut passed = 0;
    let mut warnings = 0;
    let mut errors = 0;

    let hardware = if check_all || check_hardware {
        let report = check_hardware_health().await;
        total_checks += 4;
        if report.cpu_cores > 0 {
            passed += 1;
        }
        if report.gpu_detected {
            passed += 1;
        } else {
            warnings += 1;
        }
        if report.npu_detected {
            passed += 1;
        } else {
            warnings += 1;
        }
        if report.memory_total_mb > 1024 {
            passed += 1;
        } else {
            warnings += 1;
        }
        errors += report.issues.len();
        report
    } else {
        HardwareReport {
            cpu_cores: 0,
            cpu_features: vec![],
            gpu_detected: false,
            gpu_info: None,
            npu_detected: false,
            npu_info: None,
            memory_total_mb: 0,
            issues: vec!["Skipped".to_string()],
        }
    };

    let ecosystem = if check_all || check_ecosystem {
        let report = check_ecosystem_health().await;
        total_checks += 2 + report.primals_reachable.len();
        if report.biomeos_dir_exists {
            passed += 1;
        } else {
            errors += 1;
        }
        passed += report.sockets_found.len();
        for primal in &report.primals_reachable {
            if primal.reachable {
                passed += 1;
            } else {
                warnings += 1;
            }
        }
        errors += report.issues.len();
        report
    } else {
        EcosystemReport {
            biomeos_dir_exists: false,
            biomeos_dir: String::new(),
            sockets_found: vec![],
            primals_reachable: vec![],
            issues: vec!["Skipped".to_string()],
        }
    };

    let config = if check_all || check_config {
        let report = check_config_health().await;
        total_checks += 2;
        if report.config_file_exists {
            passed += 1;
        } else {
            warnings += 1;
        }
        if !report.env_vars_set.is_empty() {
            passed += 1;
        }
        errors += report.issues.len();
        report
    } else {
        ConfigReport {
            config_file_exists: false,
            config_file_path: None,
            env_vars_set: vec![],
            issues: vec!["Skipped".to_string()],
        }
    };

    let overall_status = if errors > 0 {
        "UNHEALTHY"
    } else if warnings > 0 {
        "DEGRADED"
    } else {
        "HEALTHY"
    }
    .to_string();

    let report = DoctorReport {
        hardware,
        ecosystem,
        config,
        summary: Summary {
            total_checks,
            passed,
            warnings,
            errors,
            overall_status,
        },
    };

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text_report(&report);
    }

    Ok(())
}
