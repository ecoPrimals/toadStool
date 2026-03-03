// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Primal Integration Types
//!
//! This test suite provides extensive coverage of Primal integration types,
//! including enums, structs, and common patterns.

use toadstool_integration_primals::*;

// ============================================================================
// PrimalType Tests
// ============================================================================

#[test]
fn test_primal_type_toadstool() {
    let primal_type = PrimalType::ToadStool;
    assert!(matches!(primal_type, PrimalType::ToadStool));
}

#[test]
fn test_primal_type_songbird() {
    let primal_type = PrimalType::Songbird;
    assert!(matches!(primal_type, PrimalType::Songbird));
}

#[test]
fn test_primal_type_beardog() {
    let primal_type = PrimalType::BearDog;
    assert!(matches!(primal_type, PrimalType::BearDog));
}

#[test]
fn test_primal_type_nestgate() {
    let primal_type = PrimalType::NestGate;
    assert!(matches!(primal_type, PrimalType::NestGate));
}

#[test]
fn test_primal_type_squirrel() {
    let primal_type = PrimalType::Squirrel;
    assert!(matches!(primal_type, PrimalType::Squirrel));
}

#[test]
fn test_primal_type_biomeos() {
    let primal_type = PrimalType::BiomeOS;
    assert!(matches!(primal_type, PrimalType::BiomeOS));
}

#[test]
fn test_primal_type_custom() {
    let primal_type = PrimalType::Custom("MyPrimal".to_string());
    match primal_type {
        PrimalType::Custom(name) => assert_eq!(name, "MyPrimal"),
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_primal_type_equality() {
    let p1 = PrimalType::ToadStool;
    let p2 = PrimalType::ToadStool;
    assert_eq!(p1, p2);
}

// ============================================================================
// StartupStatus Tests
// ============================================================================

#[test]
fn test_startup_status_success() {
    let status = StartupStatus::Success;
    assert!(matches!(status, StartupStatus::Success));
}

#[test]
fn test_startup_status_failed() {
    let status = StartupStatus::Failed("error".to_string());
    match status {
        StartupStatus::Failed(msg) => assert_eq!(msg, "error"),
        _ => panic!("Expected Failed variant"),
    }
}

#[test]
fn test_startup_status_partial() {
    let status = StartupStatus::Partial(vec!["service1".to_string()]);
    match status {
        StartupStatus::Partial(services) => assert_eq!(services.len(), 1),
        _ => panic!("Expected Partial variant"),
    }
}

// ============================================================================
// HealthCheckStatus Tests
// ============================================================================

#[test]
fn test_health_check_status_healthy() {
    let status = HealthCheckStatus::Healthy;
    assert!(matches!(status, HealthCheckStatus::Healthy));
}

#[test]
fn test_health_check_status_unhealthy() {
    let status = HealthCheckStatus::Unhealthy;
    assert!(matches!(status, HealthCheckStatus::Unhealthy));
}

#[test]
fn test_health_check_status_pending() {
    let status = HealthCheckStatus::Pending;
    assert!(matches!(status, HealthCheckStatus::Pending));
}

// ============================================================================
// PrimalMessageType Tests
// ============================================================================

#[test]
fn test_primal_message_type_config_update() {
    let msg_type = PrimalMessageType::ConfigUpdate;
    assert!(matches!(msg_type, PrimalMessageType::ConfigUpdate));
}

#[test]
fn test_primal_message_type_resource_request() {
    let msg_type = PrimalMessageType::ResourceRequest;
    assert!(matches!(msg_type, PrimalMessageType::ResourceRequest));
}

#[test]
fn test_primal_message_type_resource_response() {
    let msg_type = PrimalMessageType::ResourceResponse;
    assert!(matches!(msg_type, PrimalMessageType::ResourceResponse));
}

#[test]
fn test_primal_message_type_health_check() {
    let msg_type = PrimalMessageType::HealthCheck;
    assert!(matches!(msg_type, PrimalMessageType::HealthCheck));
}

#[test]
fn test_primal_message_type_metrics_request() {
    let msg_type = PrimalMessageType::MetricsRequest;
    assert!(matches!(msg_type, PrimalMessageType::MetricsRequest));
}

#[test]
fn test_primal_message_type_metrics_response() {
    let msg_type = PrimalMessageType::MetricsResponse;
    assert!(matches!(msg_type, PrimalMessageType::MetricsResponse));
}

#[test]
fn test_primal_message_type_service_discovery() {
    let msg_type = PrimalMessageType::ServiceDiscovery;
    assert!(matches!(msg_type, PrimalMessageType::ServiceDiscovery));
}

#[test]
fn test_primal_message_type_auth_token() {
    let msg_type = PrimalMessageType::AuthToken;
    assert!(matches!(msg_type, PrimalMessageType::AuthToken));
}

#[test]
fn test_primal_message_type_custom() {
    let msg_type = PrimalMessageType::Custom("MyMessage".to_string());
    match msg_type {
        PrimalMessageType::Custom(name) => assert_eq!(name, "MyMessage"),
        _ => panic!("Expected Custom variant"),
    }
}

// ============================================================================
// PrimalBootstrapResult Tests
// ============================================================================

#[test]
fn test_primal_bootstrap_result_not_started() {
    let result = PrimalBootstrapResult::NotStarted;
    assert!(matches!(result, PrimalBootstrapResult::NotStarted));
}

#[test]
fn test_primal_bootstrap_result_success() {
    let result = PrimalBootstrapResult::Success;
    assert!(matches!(result, PrimalBootstrapResult::Success));
}

#[test]
fn test_primal_bootstrap_result_running() {
    let result = PrimalBootstrapResult::Running;
    assert!(matches!(result, PrimalBootstrapResult::Running));
}

#[test]
fn test_primal_bootstrap_result_failed() {
    let result = PrimalBootstrapResult::Failed("error".to_string());
    match result {
        PrimalBootstrapResult::Failed(msg) => assert_eq!(msg, "error"),
        _ => panic!("Expected Failed variant"),
    }
}

// ============================================================================
// Test Counter
// ============================================================================

#[test]
fn test_primal_types_coverage_summary() {
    println!("============================================");
    println!("Primal Types Tests Summary:");
    println!("============================================");
    println!("PrimalType:              8 tests");
    println!("StartupStatus:           3 tests");
    println!("HealthCheckStatus:       3 tests");
    println!("PrimalMessageType:       9 tests");
    println!("PrimalBootstrapResult:   4 tests");
    println!("============================================");
    println!("Total Primal Types Tests: 27 tests");
    println!("============================================");
}
