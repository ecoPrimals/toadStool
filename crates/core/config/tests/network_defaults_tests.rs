//! Tests for network configuration defaults
//!
//! Verifies that network defaults are valid and follow best practices.

use toadstool_config::defaults::network;
use toadstool_config::defaults::resources;

#[test]
fn test_network_ports_are_non_privileged() {
    // All default ports should be >= 1024 (non-privileged)
    // Convert consts to runtime values to avoid clippy::assertions_on_constants
    let songbird_port = network::SONGBIRD_PORT;
    let beardog_port = network::BEARDOG_PORT;
    let nestgate_port = network::NESTGATE_PORT;
    let squirrel_port = network::SQUIRREL_PORT;
    let api_port = network::API_PORT;
    let metrics_port = network::METRICS_PORT;
    let discovery_port = network::DISCOVERY_PORT;
    let federation_port = network::FEDERATION_PORT;

    assert!(
        songbird_port >= 1024,
        "Songbird port should be non-privileged"
    );
    assert!(
        beardog_port >= 1024,
        "BearDog port should be non-privileged"
    );
    assert!(
        nestgate_port >= 1024,
        "NestGate port should be non-privileged"
    );
    assert!(
        squirrel_port >= 1024,
        "Squirrel port should be non-privileged"
    );
    assert!(api_port >= 1024, "API port should be non-privileged");
    assert!(
        metrics_port >= 1024,
        "Metrics port should be non-privileged"
    );
    assert!(
        discovery_port >= 1024,
        "Discovery port should be non-privileged"
    );
    assert!(
        federation_port >= 1024,
        "Federation port should be non-privileged"
    );
}

#[test]
fn test_ecosystem_ports_are_unique() {
    // Verify all ecosystem primal ports are different
    let ports = [
        network::SONGBIRD_PORT,
        network::BEARDOG_PORT,
        network::NESTGATE_PORT,
        network::SQUIRREL_PORT,
        network::API_PORT,
        network::METRICS_PORT,
        network::DISCOVERY_PORT,
        network::FEDERATION_PORT,
    ];

    // Check for duplicates
    for (i, port1) in ports.iter().enumerate() {
        for port2 in ports.iter().skip(i + 1) {
            assert_ne!(
                port1, port2,
                "Ports should be unique, found duplicate: {}",
                port1
            );
        }
    }
}

#[test]
fn test_localhost_is_valid_address() {
    assert_eq!(network::LOCALHOST, "127.0.0.1");

    // Should be parseable as IP address
    assert!(network::LOCALHOST.parse::<std::net::IpAddr>().is_ok());
}

#[test]
fn test_default_songbird_endpoint() {
    let endpoint = toadstool_config::network::default_songbird_endpoint();
    assert!(
        endpoint.contains("8080"),
        "Endpoint should contain Songbird port"
    );
    assert!(endpoint.contains("127.0.0.1") || endpoint.contains("localhost"));
    assert!(endpoint.starts_with("http://"), "Should be HTTP URL");
}

#[test]
fn test_port_constants_match_expected_values() {
    // Document expected port values
    assert_eq!(network::SONGBIRD_PORT, 8080);
    assert_eq!(network::BEARDOG_PORT, 8081);
    assert_eq!(network::NESTGATE_PORT, 8082);
    assert_eq!(network::SQUIRREL_PORT, 8083);
    assert_eq!(network::API_PORT, 8084);
    assert_eq!(network::METRICS_PORT, 9090);
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
