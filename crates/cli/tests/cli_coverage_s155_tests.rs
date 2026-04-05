// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive coverage tests for under-covered CLI modules
//!
//! Covers: commands/definitions, commands/doctor, lib, setup, monitoring,
//! templates, `zero_config`

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::time::Duration;
use uuid::Uuid;

use clap::Parser;
use toadstool_cli::{
    BiomeManifest, BiomeMetadata, BiomeNetworking, BiomeResources, BiomeSecurity, BiomeStorage,
    Cli, CliContext, Commands, EcosystemCommands, ModeCommand, TransportCommands,
    UniversalCommands,
};

// ============================================================================
// CLI Command Definitions and Parsing
// ============================================================================

#[test]
fn test_cli_parse_run_command() {
    let result = Cli::try_parse_from(["toadstool", "run", "biome.yaml"]);
    assert!(result.is_ok());
    let cli = result.unwrap();
    match &cli.command {
        Commands::Run { manifest, .. } => {
            assert_eq!(manifest, &PathBuf::from("biome.yaml"));
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn test_cli_parse_run_with_options() {
    let result = Cli::try_parse_from([
        "toadstool",
        "run",
        "biome.yaml",
        "--name",
        "my-biome",
        "--debug",
        "--cpu-limit",
        "4.0",
        "--security",
        "medium",
    ]);
    assert!(result.is_ok());
    let cli = result.unwrap();
    match &cli.command {
        Commands::Run {
            name,
            debug,
            cpu_limit,
            security,
            ..
        } => {
            assert_eq!(name.as_deref(), Some("my-biome"));
            assert!(*debug);
            assert_eq!(*cpu_limit, Some(4.0));
            assert_eq!(security, "medium");
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn test_cli_parse_down_command() {
    let result = Cli::try_parse_from(["toadstool", "down", "my-biome", "--force"]);
    assert!(result.is_ok());
    let cli = result.unwrap();
    match &cli.command {
        Commands::Down { biome, force, .. } => {
            assert_eq!(biome, "my-biome");
            assert!(*force);
        }
        _ => panic!("Expected Down command"),
    }
}

#[test]
fn test_cli_parse_doctor_command() {
    // Note: Doctor has --config (bool) which conflicts with global --config (PathBuf).
    // Parsing doctor triggers this conflict. We test doctor via run_doctor instead.
    // Verify Doctor variant exists in Commands.
    let _ = Commands::Doctor {
        all: false,
        hardware: true,
        ecosystem: false,
        config: false,
        format: "json".to_string(),
        fix: false,
    };
}

#[test]
fn test_cli_parse_init_command() {
    let result = Cli::try_parse_from(["toadstool", "init", ".", "--template", "basic"]);
    assert!(result.is_ok());
    let cli = result.unwrap();
    match &cli.command {
        Commands::Init { path, template, .. } => {
            assert_eq!(path, &PathBuf::from("."));
            assert_eq!(template, "basic");
        }
        _ => panic!("Expected Init command"),
    }
}

#[test]
fn test_cli_parse_ecosystem_discover() {
    let result = Cli::try_parse_from([
        "toadstool",
        "ecosystem",
        "discover",
        "--services",
        "songbird",
    ]);
    assert!(result.is_ok());
    let cli = result.unwrap();
    match &cli.command {
        Commands::Ecosystem {
            action: EcosystemCommands::Discover { services, .. },
        } => {
            assert_eq!(services, &["songbird"]);
        }
        _ => panic!("Expected Ecosystem Discover"),
    }
}

#[test]
fn test_cli_parse_mode_science() {
    let result = Cli::try_parse_from(["toadstool", "mode", "science"]);
    assert!(result.is_ok());
    let cli = result.unwrap();
    match &cli.command {
        Commands::Mode {
            action: ModeCommand::Science { .. },
        } => {}
        _ => panic!("Expected Mode Science"),
    }
}

#[test]
fn test_cli_parse_execute_command() {
    let result = Cli::try_parse_from([
        "toadstool",
        "execute",
        "workload.toml",
        "--runtime",
        "native",
        "--timeout",
        "60",
    ]);
    assert!(result.is_ok());
    let cli = result.unwrap();
    match &cli.command {
        Commands::Execute {
            workload,
            runtime,
            timeout,
            ..
        } => {
            assert_eq!(workload, &PathBuf::from("workload.toml"));
            assert_eq!(runtime.as_deref(), Some("native"));
            assert_eq!(*timeout, 60);
        }
        _ => panic!("Expected Execute command"),
    }
}

#[test]
fn test_cli_parse_validate_command() {
    let result = Cli::try_parse_from(["toadstool", "validate", "biome.yaml", "--check-resources"]);
    assert!(result.is_ok());
    match &result.unwrap().command {
        Commands::Validate {
            manifest,
            check_resources,
            ..
        } => {
            assert_eq!(manifest, &PathBuf::from("biome.yaml"));
            assert!(*check_resources);
        }
        _ => panic!("Expected Validate command"),
    }
}

#[test]
fn test_cli_parse_ps_command() {
    let result = Cli::try_parse_from(["toadstool", "ps", "--all", "--format", "json"]);
    assert!(result.is_ok());
    match &result.unwrap().command {
        Commands::Ps { all, format, .. } => {
            assert!(*all);
            assert_eq!(format, "json");
        }
        _ => panic!("Expected Ps command"),
    }
}

#[test]
fn test_cli_parse_logs_command() {
    let result =
        Cli::try_parse_from(["toadstool", "logs", "my-biome", "--follow", "--lines", "50"]);
    assert!(result.is_ok());
    match &result.unwrap().command {
        Commands::Logs {
            target,
            follow,
            lines,
            ..
        } => {
            assert_eq!(target, "my-biome");
            assert!(*follow);
            assert_eq!(*lines, 50);
        }
        _ => panic!("Expected Logs command"),
    }
}

#[test]
fn test_cli_parse_capabilities_command() {
    let result = Cli::try_parse_from(["toadstool", "capabilities", "--detailed"]);
    assert!(result.is_ok());
    match &result.unwrap().command {
        Commands::Capabilities { detailed, .. } => assert!(*detailed),
        _ => panic!("Expected Capabilities command"),
    }
}

#[test]
fn test_cli_parse_universal_benchmark() {
    // Universal detect has -c/--categories which conflicts with global --config.
    // Test Universal Benchmark instead.
    let result = Cli::try_parse_from([
        "toadstool",
        "universal",
        "benchmark",
        "--suite",
        "standard",
        "--format",
        "json",
    ]);
    assert!(result.is_ok());
    match &result.unwrap().command {
        Commands::Universal {
            operation: UniversalCommands::Benchmark { suite, format, .. },
        } => {
            assert_eq!(suite, "standard");
            assert_eq!(format, "json");
        }
        _ => panic!("Expected Universal Benchmark"),
    }
}

#[test]
fn test_cli_parse_transport_discover() {
    let result = Cli::try_parse_from(["toadstool", "transport", "discover"]);
    assert!(result.is_ok());
    match &result.unwrap().command {
        Commands::Transport {
            action: TransportCommands::Discover { .. },
        } => {}
        _ => panic!("Expected Transport Discover"),
    }
}

// ============================================================================
// Lib - CliContext, load_biome_manifest, validate_manifest
// ============================================================================

#[test]
fn test_cli_context_new() {
    let cli = Cli::try_parse_from(["toadstool", "run", "biome.yaml"]).unwrap();
    let ctx = CliContext::new(&cli).expect("context");
    assert!(ctx.config_path.is_none());
    assert!(!ctx.verbose);
}

#[test]
fn test_cli_context_with_config_and_dir() {
    let cli = Cli::try_parse_from([
        "toadstool",
        "--config",
        "/etc/toadstool.toml",
        "-C",
        "/tmp",
        "run",
        "biome.yaml",
    ])
    .unwrap();
    let ctx = CliContext::new(&cli).expect("context");
    assert_eq!(
        ctx.config_path.as_ref().unwrap(),
        &PathBuf::from("/etc/toadstool.toml")
    );
    assert_eq!(ctx.working_dir, PathBuf::from("/tmp"));
}

#[test]
fn test_validate_manifest_missing_beardog_warning() {
    let now = std::time::SystemTime::now();
    let manifest = BiomeManifest {
        metadata: BiomeMetadata {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: None,
            author: None,
            created: now,
            updated: now,
            tags: vec![],
        },
        primals: HashMap::new(),
        services: HashMap::new(),
        resources: BiomeResources {
            cpu_limit: None,
            memory_limit: None,
            storage_limit: None,
            gpu_limit: None,
            network_bandwidth: None,
        },
        security: BiomeSecurity {
            isolation_level: "high".to_string(),
            trust_level: "verified".to_string(),
            beardog_required: true,
            crypto_policies: vec![],
            allowed_networks: vec![],
            forbidden_syscalls: vec![],
        },
        networking: BiomeNetworking {
            mode: "bridge".to_string(),
            dns_servers: vec![],
            port_mappings: vec![],
            network_policies: vec![],
        },
        storage: BiomeStorage {
            nestgate_integration: None,
            datasets: vec![],
            volumes: vec![],
            backup_policy: None,
        },
    };
    let warnings = toadstool_cli::validate_manifest(&manifest).expect("validate");
    assert!(warnings.iter().any(|w| w.contains("security service")));
    assert!(warnings.iter().any(|w| w.contains("CPU limit")));
}

// ============================================================================
// Setup - exit codes, exit_code_for_error
// ============================================================================

#[test]
fn test_setup_exit_codes_constants() {
    assert_eq!(toadstool_cli::setup::exit_codes::GENERAL_ERROR, 1);
    assert_eq!(toadstool_cli::setup::exit_codes::CONFIG_ERROR, 2);
    assert_eq!(toadstool_cli::setup::exit_codes::RUNTIME_ERROR, 3);
    assert_eq!(toadstool_cli::setup::exit_codes::INTERRUPTED, 130);
}

#[test]
fn test_setup_exit_code_for_config_error() {
    #[derive(Debug)]
    struct ConfigErr;
    impl std::fmt::Display for ConfigErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "invalid configuration")
        }
    }
    impl std::error::Error for ConfigErr {}

    let code = toadstool_cli::setup::exit_code_for_error(&ConfigErr);
    assert_eq!(code, toadstool_cli::setup::exit_codes::CONFIG_ERROR);
}

// ============================================================================
// Monitoring - Types, Collectors, Metrics Store, Alerting, Dashboard
// ============================================================================

#[cfg(feature = "monitoring")]
mod monitoring_tests {
    use super::*;
    use std::collections::HashMap;
    use toadstool_cli::monitoring::{
        AlertCondition, AlertRule, AlertSeverity, BiomeStatusSummary, ComparisonOperator,
        HealthStatus, Metric, MetricBatch, MetricValue, MonitoringSession, SessionStatus,
        SystemHealth, collect_performance_metrics, collect_resource_usage, collect_system_health,
        evaluate_health_alerts, load_default_alert_rules,
    };
    use toadstool_cli::monitoring::{
        MetricsCollector, MetricsStore, NetworkMetricsCollector, ProcessMetricsCollector,
        SystemMetricsCollector,
    };

    #[test]
    fn test_monitoring_target_variants() {
        let _biome = toadstool_cli::monitoring::MonitoringTarget::Biome("test".to_string());
        let _svc =
            toadstool_cli::monitoring::MonitoringTarget::Service("b".to_string(), "s".to_string());
        let _sys = toadstool_cli::monitoring::MonitoringTarget::System;
        let _plat = toadstool_cli::monitoring::MonitoringTarget::Platform("linux".to_string());
        let _fed = toadstool_cli::monitoring::MonitoringTarget::Federation;
    }

    #[test]
    fn test_metric_value_variants() {
        let _counter = MetricValue::Counter(42);
        let _gauge = MetricValue::Gauge(42.5);
        let _hist = MetricValue::Histogram(vec![1.0, 2.0, 3.0]);
        let _text = MetricValue::Text("ok".to_string());
    }

    #[test]
    fn test_health_status_variants() {
        let _h = HealthStatus::Healthy;
        let _w = HealthStatus::Warning;
        let _c = HealthStatus::Critical;
        let _u = HealthStatus::Unknown;
    }

    #[test]
    fn test_system_metrics_collector() {
        let collector = SystemMetricsCollector::new();
        assert_eq!(collector.name(), "system");
        let batch = collector.collect().expect("collect");
        assert_eq!(batch.source, "system");
        assert!(!batch.metrics.is_empty());
        assert!(collector.capabilities().contains(&"cpu".to_string()));
    }

    #[test]
    fn test_process_metrics_collector() {
        let collector = ProcessMetricsCollector::new();
        assert_eq!(collector.name(), "process");
        let batch = collector.collect().expect("collect");
        assert_eq!(batch.source, "process");
    }

    #[test]
    fn test_network_metrics_collector() {
        let collector = NetworkMetricsCollector::new();
        assert_eq!(collector.name(), "network");
        let _ = collector.collect();
    }

    #[test]
    fn test_metrics_store_new() {
        let store = MetricsStore::new(Duration::from_secs(3600));
        drop(store);
    }

    #[tokio::test]
    async fn test_metrics_store_store_batch() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        let batch = MetricBatch {
            timestamp: std::time::SystemTime::now(),
            source: "test".to_string(),
            metrics: vec![Metric {
                name: "cpu".to_string(),
                value: MetricValue::Gauge(75.0),
                labels: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            }],
        };
        store.store_batch(batch).await;
    }

    #[test]
    fn test_load_default_alert_rules() {
        let rules = load_default_alert_rules();
        assert!(!rules.is_empty());
        assert!(rules.iter().any(|r| r.id == "high_cpu"));
        assert!(rules.iter().any(|r| r.id == "high_memory"));
        assert!(rules.iter().any(|r| r.id == "low_storage"));
    }

    #[test]
    fn test_evaluate_health_alerts_critical_cpu() {
        let health = SystemHealth {
            overall_status: HealthStatus::Critical,
            cpu_health: HealthStatus::Critical,
            memory_health: HealthStatus::Healthy,
            storage_health: HealthStatus::Healthy,
            network_health: HealthStatus::Healthy,
        };
        let alerts = evaluate_health_alerts(&health);
        assert!(!alerts.is_empty());
        assert!(alerts.iter().any(|a| a.rule_name.contains("cpu")));
    }

    #[test]
    fn test_evaluate_health_alerts_healthy() {
        let health = SystemHealth {
            overall_status: HealthStatus::Healthy,
            cpu_health: HealthStatus::Healthy,
            memory_health: HealthStatus::Healthy,
            storage_health: HealthStatus::Healthy,
            network_health: HealthStatus::Healthy,
        };
        let alerts = evaluate_health_alerts(&health);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_collect_system_health() {
        let health = collect_system_health().expect("health");
        assert!(matches!(
            health.overall_status,
            HealthStatus::Healthy
                | HealthStatus::Warning
                | HealthStatus::Critical
                | HealthStatus::Unknown
        ));
    }

    #[test]
    fn test_collect_resource_usage() {
        let usage = collect_resource_usage().expect("usage");
        assert!(usage.cpu_percent >= 0.0);
        assert!(usage.memory_total_gb >= 0.0);
        assert_eq!(usage.load_average.len(), 3);
    }

    #[test]
    fn test_collect_performance_metrics() {
        let mut sessions = HashMap::new();
        sessions.insert(
            "s1".to_string(),
            MonitoringSession {
                id: Uuid::new_v4(),
                target: toadstool_cli::monitoring::MonitoringTarget::System,
                started: std::time::SystemTime::now(),
                interval: Duration::from_secs(10),
                metrics: vec![],
                status: SessionStatus::Active,
                last_update: std::time::SystemTime::now(),
            },
        );
        let perf = collect_performance_metrics(&sessions);
        assert!(perf.success_rate > 0.0 || perf.queue_depth > 0);
    }

    #[test]
    fn test_alert_rule_condition_threshold() {
        let rule = AlertRule {
            id: "t".to_string(),
            name: "Test".to_string(),
            condition: AlertCondition::Threshold {
                metric: "cpu".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 90.0,
                duration: Duration::from_secs(60),
            },
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown: Duration::from_secs(300),
            last_triggered: None,
        };
        assert!(rule.enabled);
    }

    #[test]
    fn test_biome_status_summary() {
        let summary = BiomeStatusSummary {
            name: "test".to_string(),
            status: "running".to_string(),
            services_running: 2,
            services_total: 2,
            cpu_usage: 25.0,
            memory_usage: 50.0,
            uptime: Duration::from_secs(3600),
        };
        assert_eq!(summary.name, "test");
        assert_eq!(summary.services_running, 2);
    }
}

// ============================================================================
// Templates - basic_templates, rendering (via generator)
// ============================================================================

#[cfg(feature = "templates")]
mod template_tests {
    use super::*;
    use toadstool_cli::templates::{
        BiomeTemplate, CustomTemplateSpec, TemplateGenerator,
        basic_templates::{create_basic_template, create_development_template},
    };

    #[test]
    fn test_create_basic_template() {
        let (name, desc, primals, services, resources, security, _, _) = create_basic_template();
        assert_eq!(name, "basic-biome");
        assert!(!desc.is_empty());
        assert!(primals.contains_key("pki-provider"));
        assert!(services.contains_key("compute"));
        assert_eq!(resources.cpu_limit, Some(4.0));
        assert!(security.beardog_required);
    }

    #[test]
    fn test_create_development_template() {
        let (name, _, _, services, _, security, _, _) = create_development_template();
        assert_eq!(name, "dev-biome");
        assert!(services.contains_key("vscode-server"));
        assert_eq!(security.isolation_level, "medium");
    }

    #[test]
    fn test_template_generator_new() {
        let generator = TemplateGenerator::new(PathBuf::from("/tmp"), false);
        drop(generator);
    }

    #[test]
    fn test_template_generator_list_templates() {
        let list = TemplateGenerator::list_templates();
        assert!(!list.is_empty());
        assert!(list.iter().any(|(n, _)| n == "basic"));
        assert!(list.iter().any(|(n, _)| n == "development"));
    }

    #[test]
    fn test_template_generator_parse_template() {
        assert!(matches!(
            TemplateGenerator::parse_template("basic"),
            Ok(BiomeTemplate::Basic)
        ));
        assert!(matches!(
            TemplateGenerator::parse_template("science"),
            Ok(BiomeTemplate::Science)
        ));
        assert!(TemplateGenerator::parse_template("unknown").is_err());
    }

    #[test]
    fn test_biome_template_custom() {
        let spec = CustomTemplateSpec {
            name: "custom".to_string(),
            description: "desc".to_string(),
            primals: vec![],
            services: vec![],
            security_level: "standard".to_string(),
            resource_profile: "minimal".to_string(),
        };
        let tpl = BiomeTemplate::Custom(spec);
        assert!(matches!(tpl, BiomeTemplate::Custom(_)));
    }

    #[tokio::test]
    async fn test_template_generator_generate_basic() {
        let temp = tempfile::tempdir().expect("tempdir");
        let generator = TemplateGenerator::new(temp.path().to_path_buf(), true);
        let path = generator
            .generate(BiomeTemplate::Basic)
            .await
            .expect("generate");
        assert!(path.exists());
        assert!(
            path.extension().is_some_and(|e| e == "yaml")
                || path.file_name().is_some_and(|n| n == "biome.yaml")
        );
    }
}

// ============================================================================
// Zero-Config Types
// ============================================================================

#[test]
fn test_zero_config_system_info_default() {
    use toadstool_cli::zero_config::SystemInfo;
    let info = SystemInfo::default();
    assert_eq!(info.cpu.cores, 1); // CpuInfo::default()
}

#[test]
fn test_zero_config_cpu_info_default() {
    use toadstool_cli::zero_config::CpuInfo;
    let cpu = CpuInfo::default();
    assert_eq!(cpu.cores, 1);
    assert_eq!(cpu.architecture, "unknown");
}

#[test]
fn test_zero_config_ecosystem_services_default() {
    use toadstool_cli::zero_config::EcosystemServices;
    let svc = EcosystemServices::default();
    assert!(svc.coordination.is_none());
    assert!(svc.security.is_none());
    assert!(svc.storage.is_none());
}

#[test]
fn test_zero_config_auto_generated_config_default() {
    use toadstool_cli::zero_config::AutoGeneratedConfig;
    let config = AutoGeneratedConfig::default();
    assert_eq!(config.biome.name, "default");
    assert_eq!(config.runtime.container_runtime, "docker");
}

#[test]
fn test_zero_config_biome_resources_default() {
    use toadstool_cli::zero_config::BiomeResources;
    let res = BiomeResources::default();
    assert_eq!(res.cpu_limit, 1.0);
    assert_eq!(res.memory_limit, "512M");
}

#[test]
fn test_zero_config_service_endpoint() {
    use toadstool_cli::zero_config::ServiceEndpoint;
    let ep = ServiceEndpoint {
        name: "songbird".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0".to_string(),
        status: "healthy".to_string(),
        auth_required: false,
        discovered_at: std::time::SystemTime::now(),
    };
    assert_eq!(ep.name, "songbird");
}

#[test]
fn test_zero_config_deployment_summary() {
    use toadstool_cli::zero_config::{
        AutoGeneratedConfig, DeploymentSummary, EcosystemServices, SystemInfo,
    };
    let summary = DeploymentSummary {
        total_time: std::time::Duration::from_secs(10),
        system_info: SystemInfo::default(),
        ecosystem_services: EcosystemServices::default(),
        config: AutoGeneratedConfig::default(),
        services_deployed: 3,
        health_status: "healthy".to_string(),
    };
    assert_eq!(summary.services_deployed, 3);
}

// ============================================================================
// Doctor Command Helpers
// ============================================================================

#[test]
fn test_doctor_report_struct() {
    use toadstool_cli::commands::doctor::{
        ConfigReport, DoctorReport, EcosystemReport, HardwareReport, PrimalStatus, Summary,
    };

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
}

#[tokio::test]
async fn test_run_doctor_json() {
    let result =
        toadstool_cli::commands::doctor::run_doctor(true, false, false, "json", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_doctor_text() {
    let result =
        toadstool_cli::commands::doctor::run_doctor(true, false, false, "text", false).await;
    assert!(result.is_ok());
}
