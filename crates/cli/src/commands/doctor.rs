//! Doctor command - System health check and diagnostics
//!
//! **UniBin Standard Compliant**: Diagnose ToadStool installation, runtime,
//! and ecosystem connectivity.
//!
//! # Checks Performed
//!
//! - Hardware detection (GPU, NPU, CPU capabilities)
//! - Ecosystem connectivity (primal socket discovery)
//! - Configuration validity
//! - Runtime dependencies

use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use sysinfo::{System, SystemExt};

/// Results of the doctor diagnostic checks
#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub hardware: HardwareReport,
    pub ecosystem: EcosystemReport,
    pub config: ConfigReport,
    pub summary: Summary,
}

#[derive(Debug, Serialize)]
pub struct HardwareReport {
    pub cpu_cores: usize,
    pub cpu_features: Vec<String>,
    pub gpu_detected: bool,
    pub gpu_info: Option<String>,
    pub npu_detected: bool,
    pub npu_info: Option<String>,
    pub memory_total_mb: u64,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EcosystemReport {
    pub biomeos_dir_exists: bool,
    pub biomeos_dir: String,
    pub sockets_found: Vec<String>,
    pub primals_reachable: Vec<PrimalStatus>,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PrimalStatus {
    pub name: String,
    pub socket_exists: bool,
    pub reachable: bool,
}

#[derive(Debug, Serialize)]
pub struct ConfigReport {
    pub config_file_exists: bool,
    pub config_file_path: Option<String>,
    pub env_vars_set: Vec<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub total_checks: usize,
    pub passed: usize,
    pub warnings: usize,
    pub errors: usize,
    pub overall_status: String,
}

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

    // Hardware checks
    let hardware = if check_all || check_hardware {
        let report = check_hardware_health().await;
        total_checks += 4; // CPU, GPU, NPU, Memory
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

    // Ecosystem checks
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

    // Config checks
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

    // Output report
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text_report(&report);
    }

    Ok(())
}

async fn check_hardware_health() -> HardwareReport {
    let mut issues = vec![];

    // CPU detection
    let cpu_cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);

    let mut cpu_features = vec![];
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            cpu_features.push("AVX2".to_string());
        }
        if std::arch::is_x86_feature_detected!("avx512f") {
            cpu_features.push("AVX-512".to_string());
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        cpu_features.push("NEON".to_string());
    }

    // GPU detection (check for wgpu adapter availability)
    let gpu_detected = check_gpu_available().await;
    let gpu_info = if gpu_detected {
        Some("GPU adapter available via wgpu".to_string())
    } else {
        issues.push("No GPU detected - compute will use CPU fallback".to_string());
        None
    };

    // NPU detection
    let npu_detected = Path::new("/dev/akida0").exists()
        || std::fs::read_dir("/sys/bus/pci/devices")
            .ok()
            .map(|entries| {
                entries.flatten().any(|e| {
                    std::fs::read_to_string(e.path().join("vendor"))
                        .ok()
                        .map(|v| v.trim() == "0x1e7c") // BrainChip vendor ID
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

    let npu_info = if npu_detected {
        Some("Akida NPU detected".to_string())
    } else {
        None
    };

    // Memory detection
    let sys = System::new_all();
    let memory_total_mb = sys.total_memory() / (1024 * 1024);

    HardwareReport {
        cpu_cores,
        cpu_features,
        gpu_detected,
        gpu_info,
        npu_detected,
        npu_info,
        memory_total_mb,
        issues,
    }
}

async fn check_gpu_available() -> bool {
    // Check if any GPU backend is available
    // This is a lightweight check that doesn't require full wgpu initialization
    #[cfg(target_os = "linux")]
    {
        // Check for Vulkan ICD
        Path::new("/usr/share/vulkan/icd.d").exists() || std::env::var("VK_ICD_FILENAMES").is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true // Assume available on other platforms
    }
}

async fn check_ecosystem_health() -> EcosystemReport {
    let mut issues = vec![];

    // Check biomeOS directory
    let biomeos_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let biomeos_dir_exists = biomeos_dir.exists();

    if !biomeos_dir_exists {
        issues.push(format!(
            "biomeOS directory does not exist: {}",
            biomeos_dir.display()
        ));
    }

    // Find sockets
    let mut sockets_found = vec![];
    if biomeos_dir_exists {
        if let Ok(entries) = std::fs::read_dir(&biomeos_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "sock").unwrap_or(false)
                    || path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.ends_with(".sock") || !n.contains('.'))
                        .unwrap_or(false)
                {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        sockets_found.push(name.to_string());
                    }
                }
            }
        }
    }

    // Check primal connectivity
    let primals = ["toadstool", "songbird", "beardog", "nestgate"];
    let mut primals_reachable = vec![];

    for primal in primals {
        let socket_path = biomeos_dir.join(format!("{primal}.sock"));
        let socket_exists = socket_path.exists();
        let reachable = if socket_exists {
            tokio::net::UnixStream::connect(&socket_path).await.is_ok()
        } else {
            false
        };

        primals_reachable.push(PrimalStatus {
            name: primal.to_string(),
            socket_exists,
            reachable,
        });
    }

    EcosystemReport {
        biomeos_dir_exists,
        biomeos_dir: biomeos_dir.display().to_string(),
        sockets_found,
        primals_reachable,
        issues,
    }
}

async fn check_config_health() -> ConfigReport {
    let issues = vec![];

    // Check for config file
    let home = std::env::var("HOME").ok();
    let config_paths: Vec<PathBuf> = vec![
        PathBuf::from("toadstool.toml"),
        PathBuf::from(".toadstool/config.toml"),
    ];

    // Add home-relative paths if HOME is set
    let mut all_paths = config_paths;
    if let Some(ref home_dir) = home {
        all_paths.push(PathBuf::from(home_dir).join(".config/toadstool/config.toml"));
    }

    let mut config_file_exists = false;
    let mut config_file_path = None;

    for path in all_paths {
        if path.exists() {
            config_file_exists = true;
            config_file_path = Some(path.display().to_string());
            break;
        }
    }

    // Check environment variables
    let env_vars = [
        "TOADSTOOL_BIND_HOST",
        "TOADSTOOL_API_PORT",
        "SONGBIRD_URL",
        "BEARDOG_URL",
        "NESTGATE_URL",
        "XDG_RUNTIME_DIR",
    ];

    let env_vars_set: Vec<String> = env_vars
        .iter()
        .filter(|v| std::env::var(v).is_ok())
        .map(|s| (*s).to_string())
        .collect();

    ConfigReport {
        config_file_exists,
        config_file_path,
        env_vars_set,
        issues,
    }
}

fn print_text_report(report: &DoctorReport) {
    println!("\n=== ToadStool Doctor Report ===\n");

    // Hardware
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

    // Ecosystem
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

    // Config
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

    // Summary
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hardware_check() {
        let report = check_hardware_health().await;
        assert!(report.cpu_cores > 0);
        assert!(report.memory_total_mb > 0);
    }

    #[tokio::test]
    async fn test_ecosystem_check() {
        let report = check_ecosystem_health().await;
        // biomeos_dir should be set (even if doesn't exist)
        assert!(!report.biomeos_dir.is_empty());
    }

    #[tokio::test]
    async fn test_config_check() {
        let report = check_config_health().await;
        // XDG_RUNTIME_DIR is usually set on Linux
        #[cfg(target_os = "linux")]
        assert!(
            report.env_vars_set.contains(&"XDG_RUNTIME_DIR".to_string())
                || report.env_vars_set.is_empty()
        );
    }
}
