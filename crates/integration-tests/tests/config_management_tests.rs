// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration management tests
//!
//! Tests use the real `ToadStoolConfig` and `NetworkConfig` APIs.
//! `NetworkConfig` contains `bind_address` (`SocketAddr`), endpoints, connection, tls.

use toadstool_config::{NetworkConfig, ToadStoolConfig};

#[test]
fn test_config_default() {
    let config = ToadStoolConfig::default();
    // network is a direct field, not an Option
    assert!(
        !config.app.name.is_empty(),
        "App name should be non-empty by default"
    );
    assert!(
        !config.logging.level.is_empty(),
        "Log level should be set by default"
    );
}

#[test]
fn test_network_config_default_has_valid_bind_address() {
    let network = NetworkConfig::default();
    let addr = network.bind_address;
    // Port 0 = OS-assigned at bind time (sovereignty: runtime discovery)
    // IP can be 0.0.0.0 (all interfaces) or 127.0.0.1 (localhost) depending on env
    assert!(
        addr.ip().is_unspecified() || addr.ip().is_loopback(),
        "Default bind address should be 0.0.0.0 or 127.0.0.1, got: {addr}"
    );
}

#[test]
fn test_network_config_tls_disabled_by_default() {
    let network = NetworkConfig::default();
    assert!(network.tls.is_none(), "TLS should be disabled by default");
}

#[test]
fn test_config_validation() {
    let config = ToadStoolConfig::default();
    let validation = config.validate();
    assert!(validation.is_ok(), "Default config should pass validation");
}

#[test]
fn test_config_serialization() {
    let config = ToadStoolConfig::default();
    let json = serde_json::to_string(&config);
    assert!(json.is_ok(), "Config should serialize to JSON");
}

#[test]
fn test_config_deserialization_round_trip() {
    let config = ToadStoolConfig::default();
    let json = serde_json::to_string(&config).expect("Failed to serialize config");
    let restored: Result<ToadStoolConfig, _> = serde_json::from_str(&json);
    assert!(
        restored.is_ok(),
        "Config should deserialize from its own JSON"
    );
}

#[test]
fn test_config_cloning() {
    let config = ToadStoolConfig::default();
    let cloned = config.clone();
    assert_eq!(config.app.name, cloned.app.name);
    assert_eq!(
        config.network.bind_address, cloned.network.bind_address,
        "Cloned config should have the same bind address"
    );
}

#[test]
fn test_network_config_endpoints_non_empty() {
    let network = NetworkConfig::default();
    // Deprecated endpoints are still populated by default for backward compatibility
    #[expect(deprecated)]
    let coordination_non_empty = !network.endpoints.coordination.is_empty();
    assert!(
        coordination_non_empty,
        "Coordination endpoint should have a default value"
    );
}

#[test]
fn test_config_for_environment() {
    let dev_config = ToadStoolConfig::default().for_environment("development");
    assert_eq!(dev_config.app.environment, "development");

    let prod_config = ToadStoolConfig::default().for_environment("production");
    assert_eq!(prod_config.app.environment, "production");
}
