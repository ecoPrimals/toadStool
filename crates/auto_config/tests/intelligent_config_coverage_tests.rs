// SPDX-License-Identifier: AGPL-3.0-only
//! Coverage tests for intelligent config: detection, analysis, hardware probing, recommendations

#![allow(clippy::pedantic)]

use toadstool_auto_config::hardware::{PerformanceClass, SystemCapabilities};
use toadstool_auto_config::intelligent::{
    classify_performance, ConfigGenerator, ConfigSnapshot, ConfigValidator, EnvironmentHint,
    IntelligentAutoConfig, PerformanceMetrics, PlatformConfig, PlatformInfo, PlatformOptimization,
    PlatformOptimizer, PlatformSupport, UsageHints, UsageLearner,
};

#[test]
fn test_intelligent_platform_optimizer_new() {
    let optimizer = PlatformOptimizer::new();
    assert!(!optimizer.platform_info.os_name.is_empty());
    assert!(!optimizer.platform_info.architecture.is_empty());
}

#[test]
fn test_intelligent_platform_optimizer_default() {
    let optimizer = PlatformOptimizer::default();
    assert!(!optimizer.platform_info.os_name.is_empty());
}

#[test]
fn test_intelligent_platform_info_detect() {
    let info = PlatformInfo::detect();
    assert_eq!(info.os_name, std::env::consts::OS);
    assert_eq!(info.architecture, std::env::consts::ARCH);
}

#[test]
fn test_intelligent_platform_config_linux_optimizations() {
    let hardware = SystemCapabilities {
        cpu_cores: 16.0,
        memory_gb: 32.0,
        ..Default::default()
    };
    let optimizer = PlatformOptimizer::new();
    let result = optimizer.optimize_for_platform(&hardware);
    assert!(result.is_ok());
    let config = result.unwrap();
    assert!(!config.optimizations.is_empty());
    if config.platform_name == "linux" {
        assert!(config.supports_containers());
        assert!(config.supports_sandboxing());
        assert!(config.supports_network_isolation());
    }
}

#[test]
fn test_intelligent_platform_config_supports_methods() {
    let mut config = PlatformConfig {
        platform_name: "test".to_string(),
        supported_features: std::collections::HashSet::new(),
        optimizations: vec![],
    };
    config
        .supported_features
        .insert(PlatformSupport::Containers);
    config
        .supported_features
        .insert(PlatformSupport::Sandboxing);

    assert!(config.supports(&PlatformSupport::Containers));
    assert!(config.supports_containers());
    assert!(config.supports_sandboxing());
    assert!(!config.supports_network_isolation());
}

#[test]
fn test_intelligent_platform_support_variants() {
    let _ = PlatformSupport::Containers;
    let _ = PlatformSupport::Sandboxing;
    let _ = PlatformSupport::ProcessIsolation;
    let _ = PlatformSupport::NetworkIsolation;
}

#[test]
fn test_intelligent_platform_optimization_structure() {
    let opt = PlatformOptimization {
        optimization_type: "test".to_string(),
        description: "Test optimization".to_string(),
        performance_gain: 0.25,
    };
    assert_eq!(opt.performance_gain, 0.25);
}

#[test]
fn test_intelligent_usage_learner_new() {
    let learner = UsageLearner::new();
    assert!(learner.environment_hints.is_empty());
}

#[test]
fn test_intelligent_usage_learner_default() {
    let learner = UsageLearner::default();
    assert!(learner.environment_hints.is_empty());
}

#[test]
fn test_intelligent_usage_hints_cpu_intensive() {
    let hints = UsageHints {
        expected_cpu_usage: 0.9,
        ..Default::default()
    };
    assert!(hints.is_cpu_intensive());
}

#[test]
fn test_intelligent_usage_hints_memory_intensive() {
    let hints = UsageHints {
        expected_memory_usage: 0.85,
        ..Default::default()
    };
    assert!(hints.is_memory_intensive());
}

#[test]
fn test_intelligent_classify_performance_high_end() {
    let hw = SystemCapabilities {
        cpu_cores: 32.0,
        memory_gb: 64.0,
        gpu_count: 2,
        ..Default::default()
    };
    assert_eq!(classify_performance(&hw), PerformanceClass::HighEnd);
}

#[test]
fn test_intelligent_classify_performance_mainstream() {
    let hw = SystemCapabilities {
        cpu_cores: 8.0,
        memory_gb: 16.0,
        gpu_count: 0,
        ..Default::default()
    };
    assert_eq!(classify_performance(&hw), PerformanceClass::Mainstream);
}

#[test]
fn test_intelligent_classify_performance_low_end() {
    let hw = SystemCapabilities {
        cpu_cores: 2.0,
        memory_gb: 4.0,
        gpu_count: 0,
        ..Default::default()
    };
    assert_eq!(classify_performance(&hw), PerformanceClass::LowEnd);
}

#[test]
fn test_intelligent_environment_hint_structure() {
    let hint = EnvironmentHint {
        hint_type: "dev".to_string(),
        confidence: 0.9,
        description: "Development environment".to_string(),
    };
    assert_eq!(hint.confidence, 0.9);
}

#[test]
fn test_intelligent_config_snapshot_structure() {
    let snapshot = ConfigSnapshot {
        timestamp: std::time::SystemTime::now(),
        config: toadstool_config::ToadStoolConfig::default(),
        hardware: SystemCapabilities::default(),
        usage_hints: UsageHints::default(),
        performance_metrics: None,
    };
    let _ = snapshot.timestamp;
}

#[test]
fn test_intelligent_performance_metrics_structure() {
    let metrics = PerformanceMetrics {
        avg_execution_time: std::time::Duration::from_secs(1),
        memory_usage_peak: 0.5,
        cpu_usage_avg: 0.3,
        throughput_executions_per_sec: 10.0,
    };
    assert_eq!(metrics.throughput_executions_per_sec, 10.0);
}

#[test]
fn test_intelligent_config_generator_new() {
    let gen = ConfigGenerator::new();
    let _ = gen;
}

#[test]
fn test_intelligent_config_generator_default() {
    let _ = ConfigGenerator::default();
}

#[test]
fn test_intelligent_config_validator_new() {
    let validator = ConfigValidator::new();
    let config = toadstool_config::ToadStoolConfig::default();
    assert!(validator.validate_configuration(&config).is_ok());
}

#[test]
fn test_intelligent_config_validator_zero_concurrent() {
    let validator = ConfigValidator::new();
    let mut config = toadstool_config::ToadStoolConfig::default();
    config.runtime.max_concurrent_executions = 0;
    let result = validator.validate_configuration(&config);
    assert!(result.is_err());
}

#[test]
fn test_intelligent_config_validator_zero_memory() {
    let validator = ConfigValidator::new();
    let mut config = toadstool_config::ToadStoolConfig::default();
    config.runtime.resource_limits.max_memory_usage = 0.0;
    let result = validator.validate_configuration(&config);
    assert!(result.is_err());
}

#[test]
fn test_intelligent_auto_config_new() {
    let config = IntelligentAutoConfig::new();
    let _ = config.hardware_detector;
    let _ = config.platform_optimizer;
    let _ = config.ecosystem_discoverer;
}

#[test]
fn test_intelligent_auto_config_default() {
    let _ = IntelligentAutoConfig::default();
}

#[tokio::test]
async fn test_intelligent_usage_learner_analyze_environment() {
    let mut learner = UsageLearner::new();
    let result = learner.analyze_environment().await;
    assert!(result.is_ok());
    let hints = result.unwrap();
    assert!(hints.predicted_workload_types.len() <= 4);
}

#[tokio::test]
async fn test_intelligent_scan_system() {
    let mut config = IntelligentAutoConfig::new();
    let result = config.scan_system().await;
    assert!(result.is_ok());
    let capabilities = result.unwrap();
    assert!(capabilities.cpu_cores > 0.0);
}

#[tokio::test]
async fn test_intelligent_discover_services() {
    let mut config = IntelligentAutoConfig::new();
    let result = config.discover_services().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_intelligent_generate_config() {
    let mut config = IntelligentAutoConfig::new();
    let hardware = config.scan_system().await.unwrap();
    let platform = config
        .platform_optimizer
        .optimize_for_platform(&hardware)
        .unwrap();
    let ecosystem = config.discover_services().await.unwrap();
    let usage_hints = config.usage_learner.analyze_environment().await.unwrap();

    let mut generator = ConfigGenerator::new();
    let result = generator
        .generate_optimal_config(&hardware, &platform, &ecosystem, &usage_hints)
        .await;
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert!(cfg.runtime.max_concurrent_executions > 0);
}

#[tokio::test]
async fn test_intelligent_generate_intelligent_config() {
    let mut config = IntelligentAutoConfig::new();
    let result = config.generate_intelligent_config().await;
    assert!(result.is_ok());
}
