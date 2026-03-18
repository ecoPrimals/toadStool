// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for CLI `network_config` module
//! Exercises `OrchestrationConfigurator`, validation, and configuration summary.

use toadstool_cli::network_config::OrchestrationConfigurator;

#[test]
fn test_configurator_new() {
    let config = OrchestrationConfigurator::new();
    drop(config);
}

#[test]
fn test_configurator_default() {
    let config = OrchestrationConfigurator::default();
    drop(config);
}

#[test]
fn test_generate_configuration_summary_enabled() {
    let config = OrchestrationConfigurator::new();
    let summary = config.generate_configuration_summary();
    assert!(summary.contains("Songbird Network Configuration"));
    assert!(summary.contains("Service Mesh"));
    assert!(summary.contains("Proxy"));
    assert!(summary.contains("Traffic Management"));
    assert!(summary.contains("Status"));
}

#[test]
fn test_validate_configuration_succeeds() {
    let config = OrchestrationConfigurator::new();
    let result = config.validate_configuration();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_apply_configuration_succeeds() {
    let config = OrchestrationConfigurator::new();
    let result = config.apply_configuration().await;
    assert!(result.is_ok());
}
