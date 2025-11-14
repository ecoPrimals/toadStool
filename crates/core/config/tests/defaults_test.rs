//! Comprehensive tests for default configuration constants
//!
//! Coverage target: Test all default values are sensible (15 tests)

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
fn test_network_service_ports_unique() {
    let ports = [
        network::SONGBIRD_PORT,
        network::BEARDOG_PORT,
        network::NESTGATE_PORT,
        network::SQUIRREL_PORT,
        network::API_PORT,
    ];

    // Check all ports are unique
    for (i, &port1) in ports.iter().enumerate() {
        for &port2 in ports.iter().skip(i + 1) {
            assert_ne!(port1, port2, "Ports should be unique");
        }
    }
}

#[test]
fn test_network_ports_in_valid_range() {
    // All ports are constants defined in code as > 1024 (non-privileged range)
    // We verify they're accessible and in expected ranges
    let ports = [
        ("SONGBIRD_PORT", network::SONGBIRD_PORT),
        ("BEARDOG_PORT", network::BEARDOG_PORT),
        ("NESTGATE_PORT", network::NESTGATE_PORT),
        ("SQUIRREL_PORT", network::SQUIRREL_PORT),
        ("API_PORT", network::API_PORT),
        ("METRICS_PORT", network::METRICS_PORT),
    ];

    for (name, port) in &ports {
        assert!(
            *port > 1024,
            "{} should be non-privileged port (>1024), got {}",
            name,
            port
        );
    }
}

#[test]
fn test_network_discovery_port() {
    assert_eq!(network::DISCOVERY_PORT, 8085); // Default is 8085
                                               // Note: DISCOVERY_PORT > 0 is guaranteed at compile time by the constant definition
}

#[test]
fn test_network_federation_port() {
    assert_eq!(network::FEDERATION_PORT, 7777);
    // Note: FEDERATION_PORT > 1024 is guaranteed at compile time (value is 7777)
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
        "Container range should be at least 100 ports, got {}",
        range_size
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
fn test_retries_backoff_settings() {
    assert_eq!(retries::BACKOFF_MS, 1_000);
    assert_eq!(retries::BACKOFF_MULTIPLIER, 2.0);
    assert_eq!(retries::MAX_BACKOFF_MS, 30_000);
    // Note: All relationships (BACKOFF_MS > 0, MULTIPLIER >= 1.0, MAX > BACKOFF)
    // are guaranteed at compile time by the constant values
}

// ============================================================================
// Sanity Check Tests (2 tests)
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
fn test_no_port_conflicts() {
    // Verify no overlap between service ports and port ranges
    let service_ports = [
        network::SONGBIRD_PORT,
        network::BEARDOG_PORT,
        network::NESTGATE_PORT,
        network::SQUIRREL_PORT,
        network::API_PORT,
        network::METRICS_PORT,
        network::DISCOVERY_PORT,
    ];

    for &port in &service_ports {
        // Service ports should not be in container allocation range
        assert!(
            !(ports::CONTAINER_START..=ports::CONTAINER_END).contains(&port),
            "Port {} conflicts with container range",
            port
        );
    }
}
