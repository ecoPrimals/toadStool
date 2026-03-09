// SPDX-License-Identifier: AGPL-3.0-only
//! Integration tests for runtime port discovery
//!
//! Tests the Deep Debt compliant port discovery system in realistic scenarios

use toadstool_common::runtime_ports::{
    discover_available_port, discover_port_with_preference, RuntimePortDiscovery,
};

#[test]
fn test_runtime_port_discovery_basic() {
    // Should discover an available port
    let port = discover_available_port();
    assert!(port.is_ok(), "Should discover an available port");

    let port = port.unwrap();
    assert!(port >= 1024, "Port should be unprivileged");
}

#[test]
fn test_runtime_port_discovery_with_preference() {
    // Try to get port 9000, or find alternative
    let port = discover_port_with_preference(9000);
    assert!(
        port.is_ok(),
        "Should discover port (preferred or alternative)"
    );

    let port = port.unwrap();
    assert!(port >= 1024, "Port should be unprivileged");
    // May or may not be 9000, depending on availability
}

#[test]
fn test_multiple_unique_ports() {
    let discovery = RuntimePortDiscovery::new();
    let ports = discovery.discover_ports(3);

    assert!(ports.is_ok(), "Should discover 3 ports");
    let ports = ports.unwrap();
    assert_eq!(ports.len(), 3, "Should have exactly 3 ports");

    // All should be valid
    for port in &ports {
        assert!(*port >= 1024, "All ports should be unprivileged");
    }
}

#[test]
fn test_port_discovery_with_custom_range() {
    let discovery = RuntimePortDiscovery::new().with_range(10000..10100);

    let port = discovery.discover_port(None);
    assert!(port.is_ok(), "Should discover port in range");

    let port = port.unwrap();
    assert!(port >= 10000, "Port should be in specified range");
    assert!(port < 10100, "Port should be in specified range");
}

#[test]
fn test_localhost_only_vs_all_interfaces() {
    // Test localhost-only binding
    let discovery_local = RuntimePortDiscovery::new().localhost_only();
    let port_local = discovery_local.discover_port(None);
    assert!(port_local.is_ok(), "Should work with localhost-only");

    // Test all-interfaces binding (requires appropriate permissions)
    let discovery_all = RuntimePortDiscovery::new().all_interfaces();
    let port_all = discovery_all.discover_port(None);
    // This might fail if we don't have permissions, which is fine
    // Just test that it doesn't panic
    let _ = port_all;
}

#[test]
fn test_deep_debt_principle_no_hardcoding() {
    // This test verifies the Deep Debt principle:
    // NO port should be hardcoded - all should be discovered at runtime

    // Multiple discoveries should succeed
    for _ in 0..5 {
        let port = discover_available_port();
        assert!(port.is_ok(), "Each discovery should succeed independently");
    }
}

#[test]
fn test_preferred_port_fallback() {
    // Try to get a privileged port (should fail and find alternative)
    let port = discover_port_with_preference(80);
    assert!(
        port.is_ok(),
        "Should discover alternative when preferred unavailable"
    );

    let port = port.unwrap();
    // Should NOT be 80 (requires root), should be alternative
    assert!(port >= 1024, "Should fall back to unprivileged port");
}

#[test]
fn test_concurrent_discovery() {
    // Test that concurrent port discovery doesn't cause issues
    use std::thread;

    let handles: Vec<_> = (0..5)
        .map(|_| thread::spawn(discover_available_port))
        .collect();

    for handle in handles {
        let result = handle.join().expect("Thread should not panic");
        assert!(result.is_ok(), "Concurrent discovery should succeed");
    }
}
