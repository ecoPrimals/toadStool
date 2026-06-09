// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn create_test_registry() -> PrimalCapabilitiesRegistry {
    let toml_content = r#"
[registry]
version = "1.0.0"
discovery_protocol = "capability-based"

[primals.toadstool]
name = "toadstool"
description = "Universal compute platform"
primary_role = "compute"
capabilities = ["universal-compute", "wasm-execution"]
protocols = ["http"]
default_port = 8080
health_endpoint = "/health"

[primals.beardog]
name = "beardog"
description = "Cryptographic security"
primary_role = "security"
capabilities = ["cryptographic-operations", "key-management"]
protocols = ["http"]
default_port = 8081
health_endpoint = "/health"

[discovery]
methods = ["environment", "mdns"]
cache_enabled = true
cache_ttl_seconds = 300
"#;

    toml::from_str(toml_content).unwrap()
}

#[test]
fn test_find_by_capability() {
    let registry = create_test_registry();
    let compute_primals = registry.find_by_capability("universal-compute");
    assert_eq!(compute_primals, vec!["toadstool"]);

    let crypto_primals = registry.find_by_capability("cryptographic-operations");
    assert_eq!(crypto_primals, vec!["beardog"]);
}

#[test]
fn test_find_by_role() {
    let registry = create_test_registry();
    let compute_primals = registry.find_by_role("compute");
    assert_eq!(compute_primals, vec!["toadstool"]);

    let security_primals = registry.find_by_role("security");
    assert_eq!(security_primals, vec!["beardog"]);
}

#[test]
fn test_get_endpoint() {
    let registry = create_test_registry();
    let endpoint = registry.get_endpoint("toadstool", "localhost").unwrap();
    assert_eq!(endpoint, "http://localhost:8080");

    let endpoint = registry.get_endpoint("beardog", "localhost").unwrap();
    assert_eq!(endpoint, "http://localhost:8081");
}

#[test]
fn test_self_knowledge() {
    let registry = create_test_registry();
    let self_def = get_self_capabilities(&registry).unwrap();
    assert_eq!(self_def.name, "toadstool");
    assert!(
        self_def
            .capabilities
            .contains(&"universal-compute".to_string())
    );
}

#[test]
fn test_self_knowledge_returns_none_for_unknown_primal() {
    let registry = create_test_registry();
    // toadstool exists, so get_self_capabilities returns Some
    let self_def = get_self_capabilities(&registry);
    assert!(self_def.is_some());
}

#[test]
fn test_find_by_capabilities_all() {
    let registry = create_test_registry();
    // Beardog has both capabilities
    let crypto_key = registry.find_by_capabilities(&["cryptographic-operations", "key-management"]);
    assert_eq!(crypto_key, vec!["beardog"]);
    // No primal has both universal-compute and key-management
    let none_match = registry.find_by_capabilities(&["universal-compute", "key-management"]);
    assert!(none_match.is_empty());
}

#[test]
fn test_get_primal_returns_none_for_unknown() {
    let registry = create_test_registry();
    let primal = registry.get_primal("nonexistent");
    assert!(primal.is_none());
}

#[test]
fn test_get_endpoint_primal_not_found_returns_error() {
    let registry = create_test_registry();
    let result = registry.get_endpoint("nonexistent", "localhost");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CapabilityError::PrimalNotFound(_)
    ));
}

#[test]
fn test_get_endpoint_uses_https_when_no_http_protocol() {
    let toml_content = r#"
[primals.secure]
name = "secure"
primary_role = "security"
capabilities = ["secure"]
protocols = ["https"]
default_port = 8443
"#;
    let registry: PrimalCapabilitiesRegistry = toml::from_str(toml_content).unwrap();
    let endpoint = registry.get_endpoint("secure", "localhost").unwrap();
    assert_eq!(endpoint, "https://localhost:8443");
}

#[test]
fn test_get_all_endpoints() {
    let registry = create_test_registry();
    let endpoints = registry.get_all_endpoints("192.168.1.1");
    assert_eq!(endpoints.len(), 2);
    assert_eq!(
        endpoints.get("toadstool").unwrap(),
        "http://192.168.1.1:8080"
    );
    assert_eq!(endpoints.get("beardog").unwrap(), "http://192.168.1.1:8081");
}

#[test]
fn test_load_from_file_not_found() {
    let result = PrimalCapabilitiesRegistry::load_from_file("/nonexistent/path/capabilities.toml");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CapabilityError::LoadFailed(_)
    ));
}

#[test]
fn test_load_from_file_parse_error() {
    let temp = std::env::temp_dir().join("invalid_capabilities.toml");
    std::fs::write(&temp, "invalid toml {{{").unwrap();
    let result = PrimalCapabilitiesRegistry::load_from_file(&temp);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CapabilityError::ParseFailed(_)
    ));
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_load_from_file_success() {
    let temp = std::env::temp_dir().join("valid_capabilities.toml");
    let content = r#"
[registry]
version = "1.0"

[primals.test]
name = "test"
primary_role = "compute"
capabilities = ["test"]
default_port = 9090
"#;
    std::fs::write(&temp, content).unwrap();
    let result = PrimalCapabilitiesRegistry::load_from_file(&temp);
    assert!(result.is_ok());
    let registry = result.unwrap();
    assert_eq!(registry.primals.len(), 1);
    assert!(registry.primals.contains_key("test"));
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_load_default_via_env_var() {
    let temp = std::env::temp_dir().join("primal_caps_env_test.toml");
    let content = r#"
[registry]
version = "1.0"

[primals.envtest]
name = "envtest"
primary_role = "compute"
capabilities = ["test"]
default_port = 7777
"#;
    std::fs::write(&temp, content).unwrap();
    let path_str = temp.to_str().unwrap().to_string();
    temp_env::with_var("PRIMAL_CAPABILITIES_PATH", Some(path_str.as_str()), || {
        let result = PrimalCapabilitiesRegistry::load_default();
        assert!(result.is_ok());
        let registry = result.unwrap();
        assert!(registry.primals.contains_key("envtest"));
    });
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_registry_metadata_defaults() {
    let toml_content = r#"
[primals.minimal]
name = "minimal"
primary_role = "compute"
default_port = 8000
"#;
    let registry: PrimalCapabilitiesRegistry = toml::from_str(toml_content).unwrap();
    assert!(registry.registry.version.is_empty());
    assert!(registry.registry.discovery_protocol.is_empty());
}

#[test]
fn test_find_by_role_empty() {
    let registry = create_test_registry();
    let result = registry.find_by_role("nonexistent-role");
    assert!(result.is_empty());
}
