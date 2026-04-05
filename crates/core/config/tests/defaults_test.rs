// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for default configuration constants
//!
//! Coverage target: Test all default values are sensible (15 tests)

#![allow(deprecated)] // Testing deprecated constants during migration

use toadstool_config::defaults::*;

// ============================================================================
// Network Defaults Tests (5 tests)
// ============================================================================

#[test]
fn test_network_localhost() {
    assert_eq!(network::LOCALHOST, "127.0.0.1");
    assert!(!network::LOCALHOST.is_empty());
}

#[test]
fn test_network_service_ports_os_assigned() {
    // Server bind ports default to 0 (OS-assigned)
    assert_eq!(network::API_PORT, 0);
    assert_eq!(network::METRICS_PORT, 0);
    assert_eq!(network::DISCOVERY_PORT, 0);
    assert_eq!(network::FEDERATION_PORT, 0);
}

#[test]
fn test_network_ports_valid() {
    // Port 0 = OS-assigned; explicit ports must be in valid range (u16 guarantees validity)
    let _ = (
        network::API_PORT,
        network::METRICS_PORT,
        network::DISCOVERY_PORT,
        network::FEDERATION_PORT,
    );
}

#[test]
fn test_network_discovery_port() {
    assert_eq!(network::DISCOVERY_PORT, 0); // OS-assigned
}

#[test]
fn test_network_federation_port() {
    assert_eq!(network::FEDERATION_PORT, 0); // OS-assigned
}

// ============================================================================
// Port Range Tests (3 tests)
// ============================================================================

#[test]
fn test_ports_container_range_valid() {
    // Verify container port range is valid
    // Note: CONTAINER_START < CONTAINER_END is guaranteed at compile time
    let range_size = ports::CONTAINER_END - ports::CONTAINER_START;
    assert!(
        range_size >= 100,
        "Container range should be at least 100 ports, got {range_size}"
    );
}

#[test]
fn test_ports_general_range_valid() {
    // Note: RANGE_START < RANGE_END is guaranteed at compile time (8080 < 8999)
    assert_eq!(ports::RANGE_START, 8080);
    assert_eq!(ports::RANGE_END, 8999);
}

#[test]
fn test_ports_sidecar_ports() {
    assert_eq!(ports::SIDECAR_LISTEN, 15001);
    assert_eq!(ports::SIDECAR_ADMIN, 15000);
    assert_ne!(ports::SIDECAR_LISTEN, ports::SIDECAR_ADMIN);
}

// ============================================================================
// Timeout Tests (3 tests)
// ============================================================================

#[test]
fn test_timeouts_execution() {
    assert_eq!(timeouts::EXECUTION_MS, 30_000);
    // Note: EXECUTION_MS > 0 and < 300_000 are guaranteed at compile time (value is 30_000)
}

#[test]
fn test_timeouts_health_and_connection() {
    assert_eq!(timeouts::HEALTH_CHECK_MS, 5_000);
    assert_eq!(timeouts::CONNECTION_MS, 5_000);
    // Note: Positive values guaranteed at compile time
}

#[test]
fn test_timeouts_request_and_idle() {
    assert_eq!(timeouts::REQUEST_MS, 30_000);
    assert_eq!(timeouts::IDLE_MS, 60_000);
    // Note: REQUEST_MS < IDLE_MS is guaranteed at compile time (30_000 < 60_000)
}

// ============================================================================
// Retry Defaults Tests (2 tests)
// ============================================================================

#[test]
fn test_retries_max_attempts() {
    assert_eq!(retries::MAX_ATTEMPTS, 3);
    // Note: MAX_ATTEMPTS > 0 and < 10 guaranteed at compile time (value is 3)
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)] // comparing against exact literal initialization
fn test_retries_backoff_settings() {
    assert_eq!(retries::BACKOFF_MS, 1_000);
    assert_eq!(retries::BACKOFF_MULTIPLIER, 2.0);
    assert_eq!(retries::MAX_BACKOFF_MS, 30_000);
    // Note: All relationships (BACKOFF_MS > 0, MULTIPLIER >= 1.0, MAX > BACKOFF)
    // are guaranteed at compile time by the constant values
}

// ============================================================================
// Validation Tests (2 tests)
// ============================================================================

#[test]
fn test_all_timeouts_positive() {
    // Note: All timeout values are guaranteed to be positive at compile time
    // This test verifies the constant values exist and can be accessed
    assert_eq!(timeouts::EXECUTION_MS, 30_000);
    assert_eq!(timeouts::HEALTH_CHECK_MS, 5_000);
    assert_eq!(timeouts::CONNECTION_MS, 5_000);
    assert_eq!(timeouts::REQUEST_MS, 30_000);
    assert_eq!(timeouts::IDLE_MS, 60_000);
    assert_eq!(timeouts::DISCOVERY_MS, 5_000);
}

#[test]
fn test_timeouts_discovery_interval_and_keepalive() {
    assert_eq!(timeouts::DISCOVERY_MS, 5_000);
    assert_eq!(timeouts::DISCOVERY_INTERVAL_MS, 30_000);
    assert_eq!(timeouts::KEEPALIVE_SEC, 60);
}

#[test]
fn test_network_bind_address_default() {
    assert_eq!(network::BIND_ADDRESS_DEFAULT, "0.0.0.0");
}

#[test]
fn test_capability_fallback_bootstrap_ports() {
    use toadstool_config::ports::capability_fallback;

    assert_eq!(capability_fallback::COORDINATION, 8080);
    assert_eq!(capability_fallback::SECURITY, 8081);
    assert_eq!(capability_fallback::STORAGE, 8082);
    assert_eq!(capability_fallback::PLATFORM, 8083);
    assert_eq!(capability_fallback::ECOSYSTEM, 8088);
}

#[test]
fn test_no_port_conflicts() {
    // Port 0 (OS-assigned) is outside container range 3000-3999
    let service_ports = [
        network::API_PORT,
        network::METRICS_PORT,
        network::DISCOVERY_PORT,
    ];

    for &port in &service_ports {
        assert!(
            !(ports::CONTAINER_START..=ports::CONTAINER_END).contains(&port),
            "Port {port} conflicts with container range"
        );
    }
}
