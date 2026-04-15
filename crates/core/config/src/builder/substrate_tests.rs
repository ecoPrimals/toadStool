// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_substrate_config_default() {
    let config = SubstrateConfig::default();
    assert!(matches!(config.preferred, SubstratePreference::Auto));
    assert_eq!(config.fallback_order.len(), 2);
    assert_eq!(config.power_budget_watts, None);
    assert_eq!(config.performance_target, PerformanceTarget::Balanced);
    assert!(config.auto_discover);
}

#[test]
fn test_substrate_config_builder() {
    let config = SubstrateConfigBuilder::new()
        .prefer_npu()
        .power_budget_watts(5.0)
        .target_energy()
        .build()
        .expect("valid config");

    assert_eq!(
        config.preferred,
        SubstratePreference::Specific(SubstrateType::Npu)
    );
    assert_eq!(config.power_budget_watts, Some(5.0));
    assert_eq!(config.performance_target, PerformanceTarget::Energy);
}

#[test]
fn test_substrate_config_builder_prefer_auto() {
    let config = SubstrateConfigBuilder::new()
        .prefer_gpu()
        .prefer_auto()
        .build()
        .expect("valid config");
    assert!(matches!(config.preferred, SubstratePreference::Auto));
}

#[test]
fn test_substrate_config_builder_prefer_cpu() {
    let config = SubstrateConfigBuilder::new()
        .prefer_cpu()
        .build()
        .expect("valid config");
    assert_eq!(
        config.preferred,
        SubstratePreference::Specific(SubstrateType::Cpu)
    );
}

#[test]
fn test_substrate_config_builder_prefer_gpu() {
    let config = SubstrateConfigBuilder::new()
        .prefer_gpu()
        .build()
        .expect("valid config");
    assert_eq!(
        config.preferred,
        SubstratePreference::Specific(SubstrateType::Gpu)
    );
}

#[test]
fn test_substrate_config_builder_target_latency() {
    let config = SubstrateConfigBuilder::new()
        .target_latency()
        .build()
        .expect("valid config");
    assert_eq!(config.performance_target, PerformanceTarget::Latency);
}

#[test]
fn test_substrate_config_builder_target_throughput() {
    let config = SubstrateConfigBuilder::new()
        .target_throughput()
        .build()
        .expect("valid config");
    assert_eq!(config.performance_target, PerformanceTarget::Throughput);
}

#[test]
fn test_substrate_config_builder_default() {
    let config = SubstrateConfigBuilder::default()
        .build()
        .expect("valid config");
    assert!(matches!(config.preferred, SubstratePreference::Auto));
    assert_eq!(config.performance_target, PerformanceTarget::Balanced);
}

#[test]
fn test_substrate_config_builder_full_chain() {
    let config = SubstrateConfigBuilder::new()
        .prefer_npu()
        .power_budget_watts(10.0)
        .target_throughput()
        .build()
        .expect("valid config");
    assert_eq!(
        config.preferred,
        SubstratePreference::Specific(SubstrateType::Npu)
    );
    assert_eq!(config.power_budget_watts, Some(10.0));
    assert_eq!(config.performance_target, PerformanceTarget::Throughput);
}

#[test]
fn test_substrate_config_presets() {
    let edge = SubstrateConfig::edge();
    assert_eq!(edge.power_budget_watts, Some(5.0));
    assert_eq!(edge.performance_target, PerformanceTarget::Energy);

    let server = SubstrateConfig::server();
    assert_eq!(server.power_budget_watts, None);
    assert_eq!(server.performance_target, PerformanceTarget::Throughput);
}

#[test]
fn test_substrate_config_with_defaults() {
    let config = SubstrateConfig::default();
    let merged = config.with_defaults();
    assert!(matches!(merged.preferred, SubstratePreference::Auto));
}

#[test]
fn test_substrate_config_from_env() {
    let config = SubstrateConfig::from_env().expect("from_env returns Ok");
    assert!(!config.fallback_order.is_empty());
}

#[test]
fn test_substrate_builder_override_chain() {
    let config = SubstrateConfigBuilder::new()
        .prefer_cpu()
        .prefer_gpu()
        .prefer_npu()
        .power_budget_watts(5.0)
        .power_budget_watts(10.0)
        .target_latency()
        .target_throughput()
        .build()
        .expect("valid config");
    assert_eq!(
        config.preferred,
        SubstratePreference::Specific(SubstrateType::Npu)
    );
    assert_eq!(config.power_budget_watts, Some(10.0));
    assert_eq!(config.performance_target, PerformanceTarget::Throughput);
}

#[test]
fn test_substrate_config_validate_empty_fallback() {
    let config = SubstrateConfig {
        fallback_order: vec![],
        ..SubstrateConfig::default()
    };
    let err = config.validate().expect_err("empty fallback_order");
    assert!(matches!(err, ConfigError::Validation(s) if s.contains("fallback_order")));
}

#[test]
fn test_substrate_config_validate_by_capability_empty() {
    let config = SubstrateConfig {
        preferred: SubstratePreference::ByCapability(vec![]),
        ..SubstrateConfig::default()
    };
    let err = config.validate().expect_err("empty ByCapability");
    assert!(matches!(err, ConfigError::Validation(s) if s.contains("ByCapability")));
}

#[test]
fn test_substrate_config_validate_bad_power_budget() {
    let config = SubstrateConfig {
        power_budget_watts: Some(-1.0),
        ..SubstrateConfig::default()
    };
    config.validate().expect_err("negative power budget");
}

#[test]
fn test_substrate_config_validate_default_ok() {
    SubstrateConfig::default()
        .validate()
        .expect("default substrate config valid");
}

#[test]
fn test_substrate_builder_build_unchecked() {
    let config = SubstrateConfigBuilder::new().build_unchecked();
    assert!(!config.fallback_order.is_empty());
}
