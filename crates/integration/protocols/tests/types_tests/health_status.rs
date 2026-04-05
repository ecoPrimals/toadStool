// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_health_status_healthy() {
    let status = HealthStatus::Healthy;
    assert_eq!(status, HealthStatus::Healthy);
}

#[test]
fn test_health_status_degraded() {
    let status = HealthStatus::Degraded;
    assert_eq!(status, HealthStatus::Degraded);
}

#[test]
fn test_health_status_unhealthy() {
    let status = HealthStatus::Unhealthy;
    assert_eq!(status, HealthStatus::Unhealthy);
}

#[test]
fn test_health_status_unknown() {
    let status = HealthStatus::Unknown;
    assert_eq!(status, HealthStatus::Unknown);
}

#[test]
fn test_health_status_comparison() {
    assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
    assert_ne!(HealthStatus::Degraded, HealthStatus::Unhealthy);
    assert_ne!(HealthStatus::Unhealthy, HealthStatus::Unknown);
}

#[test]
fn test_health_status_serialization() {
    let status = HealthStatus::Healthy;
    let serialized = serde_json::to_string(&status).expect("Failed to serialize");
    let deserialized: HealthStatus =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(status, deserialized);
}
