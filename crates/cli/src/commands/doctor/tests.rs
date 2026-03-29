// SPDX-License-Identifier: AGPL-3.0-only
use super::checks::{check_config_health, check_ecosystem_health, check_hardware_health};
use super::run_doctor;
use super::{ConfigReport, DoctorReport, EcosystemReport, HardwareReport, PrimalStatus, Summary};

#[tokio::test]
async fn test_hardware_check() {
    let report = check_hardware_health().await;
    assert!(report.cpu_cores > 0);
    assert!(report.memory_total_mb > 0);
}

#[tokio::test]
async fn test_ecosystem_check() {
    let report = check_ecosystem_health().await;
    assert!(!report.biomeos_dir.is_empty());
}

#[tokio::test]
async fn test_config_check() {
    let report = check_config_health().await;
    #[cfg(target_os = "linux")]
    assert!(
        report.env_vars_set.contains(&"XDG_RUNTIME_DIR".to_string())
            || report.env_vars_set.is_empty()
    );
}

#[test]
fn test_doctor_report_struct_creation() {
    let report = DoctorReport {
        hardware: HardwareReport {
            cpu_cores: 8,
            cpu_features: vec!["AVX2".to_string()],
            gpu_detected: true,
            gpu_info: Some("NVIDIA".to_string()),
            npu_detected: false,
            npu_info: None,
            memory_total_mb: 16384,
            issues: vec![],
        },
        ecosystem: EcosystemReport {
            biomeos_dir_exists: true,
            biomeos_dir: "/tmp/biomeos".to_string(),
            sockets_found: vec!["songbird.sock".to_string()],
            primals_reachable: vec![PrimalStatus {
                name: "songbird".to_string(),
                socket_exists: true,
                reachable: true,
            }],
            issues: vec![],
        },
        config: ConfigReport {
            config_file_exists: true,
            config_file_path: Some("/home/.config/toadstool/config.toml".to_string()),
            env_vars_set: vec!["TOADSTOOL_BIND_HOST".to_string()],
            issues: vec![],
        },
        summary: Summary {
            total_checks: 10,
            passed: 9,
            warnings: 1,
            errors: 0,
            overall_status: "HEALTHY".to_string(),
        },
    };
    assert_eq!(report.hardware.cpu_cores, 8);
    assert_eq!(report.summary.overall_status, "HEALTHY");
    assert_eq!(report.ecosystem.primals_reachable.len(), 1);
}

#[test]
fn test_primal_status_struct() {
    let status = PrimalStatus {
        name: "beardog".to_string(),
        socket_exists: false,
        reachable: false,
    };
    assert_eq!(status.name, "beardog");
    assert!(!status.socket_exists);
    assert!(!status.reachable);
}

#[test]
fn test_summary_status_variants() {
    let healthy = Summary {
        total_checks: 5,
        passed: 5,
        warnings: 0,
        errors: 0,
        overall_status: "HEALTHY".to_string(),
    };
    let degraded = Summary {
        total_checks: 5,
        passed: 4,
        warnings: 1,
        errors: 0,
        overall_status: "DEGRADED".to_string(),
    };
    let unhealthy = Summary {
        total_checks: 5,
        passed: 3,
        warnings: 0,
        errors: 2,
        overall_status: "UNHEALTHY".to_string(),
    };
    assert_eq!(healthy.overall_status, "HEALTHY");
    assert_eq!(degraded.overall_status, "DEGRADED");
    assert_eq!(unhealthy.overall_status, "UNHEALTHY");
}

#[tokio::test]
async fn test_run_doctor_check_all_false_skips_all() {
    let result = run_doctor(false, false, false, "json", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_doctor_json_format() {
    let result = run_doctor(true, false, false, "json", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_doctor_text_format() {
    let result = run_doctor(true, false, false, "text", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_doctor_hardware_only() {
    let result = run_doctor(true, false, false, "text", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_doctor_ecosystem_only() {
    let result = run_doctor(false, true, false, "text", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_doctor_config_only() {
    let result = run_doctor(false, false, true, "text", false).await;
    assert!(result.is_ok());
}
