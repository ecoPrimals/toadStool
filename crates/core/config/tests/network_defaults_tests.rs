// SPDX-License-Identifier: AGPL-3.0-only
//! Backward compatibility tests for deprecated network configuration functions
#![allow(deprecated)]

//! Tests for network configuration defaults
//!
//! Verifies that network defaults are valid and follow best practices.

use toadstool_config::defaults::network;
use toadstool_config::defaults::resources;

#[test]
fn test_network_ports_os_assigned() {
    // Server bind ports default to 0 (OS-assigned at bind time)
    assert_eq!(network::API_PORT, 0);
    assert_eq!(network::METRICS_PORT, 0);
    assert_eq!(network::DISCOVERY_PORT, 0);
    assert_eq!(network::FEDERATION_PORT, 0);
}

#[test]
fn test_ecosystem_ports_os_assigned() {
    // All server bind ports default to 0 (OS-assigned)
    assert_eq!(network::API_PORT, 0);
    assert_eq!(network::METRICS_PORT, 0);
    assert_eq!(network::DISCOVERY_PORT, 0);
    assert_eq!(network::FEDERATION_PORT, 0);
}

#[test]
fn test_localhost_is_valid_address() {
    assert_eq!(network::LOCALHOST, "127.0.0.1");

    // Should be parseable as IP address
    assert!(network::LOCALHOST.parse::<std::net::IpAddr>().is_ok());
}

#[test]
fn test_default_songbird_endpoint() {
    // network::default_songbird_endpoint uses ports::fallback::SONGBIRD (8080)
    let endpoint = toadstool_config::network::default_songbird_endpoint();
    assert!(
        endpoint.contains("8080"),
        "Endpoint should contain Songbird port"
    );
    assert!(
        endpoint.contains("127.0.0.1")
            || endpoint.contains("localhost")
            || endpoint.contains("0.0.0.0")
    );
    assert!(endpoint.starts_with("http://"), "Should be HTTP URL");
}

#[test]
fn test_port_constants_os_assigned() {
    // Server bind ports default to 0 (OS-assigned)
    assert_eq!(network::API_PORT, 0);
    assert_eq!(network::METRICS_PORT, 0);
}

#[test]
fn test_max_connections_is_reasonable() {
    // Note: These are compile-time constant checks
    // Verify MAX_CONNECTIONS is within expected range at runtime
    const _: () = assert!(resources::MAX_CONNECTIONS > 0);
    const _: () = assert!(resources::MAX_CONNECTIONS <= 10000);

    // Runtime verification for dynamic scenarios
    let max_connections = resources::MAX_CONNECTIONS;
    assert!(max_connections > 0, "MAX_CONNECTIONS should be positive");
    assert!(
        max_connections <= 10000,
        "MAX_CONNECTIONS should be reasonable"
    );
}
