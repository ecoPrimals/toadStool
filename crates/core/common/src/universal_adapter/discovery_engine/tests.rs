// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from mod.rs (S334).

use super::*;
use std::collections::HashMap;

#[test]
fn environment_provider_config_default_is_empty() {
    let config = EnvironmentProviderConfig::default();
    assert!(config.security_provider_url.is_none());
    assert!(config.storage_provider_url.is_none());
    assert!(config.coordination_provider_url.is_none());
    assert!(config.intelligence_provider_url.is_none());
}

#[test]
fn parse_endpoint_empty_string_uses_custom_fallback() {
    let ep = EnvironmentSource::parse_endpoint("").unwrap();
    assert!(matches!(
        ep,
        ServiceEndpoint::Custom {
            protocol,
            address
        } if protocol == "unknown" && address.is_empty()
    ));
}

#[test]
fn capability_from_str_empty_defaults_to_coordination() {
    let cap = LocalRegistrySource::capability_from_str("");
    assert!(matches!(cap, CapabilityType::Coordination { .. }));
}

#[test]
fn capability_from_str_is_case_insensitive() {
    assert!(matches!(
        LocalRegistrySource::capability_from_str("SECURITY"),
        CapabilityType::Security { .. }
    ));
    assert!(matches!(
        LocalRegistrySource::capability_from_str("Storage"),
        CapabilityType::Storage { .. }
    ));
}

#[test]
fn parse_endpoint_tcp_missing_port_rejected() {
    assert!(EnvironmentSource::parse_endpoint("tcp://localhost").is_err());
    assert!(LocalRegistrySource::parse_endpoint("tcp://localhost").is_err());
}

#[test]
fn registry_service_entry_deserializes_field_aliases() {
    let json = r#"{"id":"alias-id","url":"http://localhost:1","capability":"network"}"#;
    let entry: RegistryServiceEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.provider_id, "alias-id");
    assert_eq!(entry.endpoint, "http://localhost:1");
    assert_eq!(entry.capability.as_deref(), Some("network"));
}

#[test]
fn registry_service_entry_malformed_json_rejected() {
    let result: Result<RegistryServiceEntry, _> = serde_json::from_str("not json");
    assert!(result.is_err());
}

#[test]
fn registry_service_entry_defaults_capability_and_metadata() {
    let json = r#"{"provider_id":"p1","endpoint":"http://localhost:0"}"#;
    let entry: RegistryServiceEntry = serde_json::from_str(json).unwrap();
    assert!(entry.capability.is_none());
    assert!(entry.metadata.is_empty());
}

#[cfg(feature = "mdns")]
#[test]
fn mdns_parse_txt_records_without_provider_id_uses_service_name() {
    let txt = HashMap::new();
    let info = MDnsSource::parse_txt_records("my-service._tcp.local", "host.local", 8080, &txt);
    assert_eq!(info.provider_id, "my-service._tcp.local");
    assert!(matches!(
        info.endpoint,
        ServiceEndpoint::Http(ref url) if url == "http://host.local:8080"
    ));
    assert!(matches!(
        info.capability,
        CapabilityType::Coordination { .. }
    ));
}

#[cfg(feature = "mdns")]
#[test]
fn mdns_parse_txt_records_strips_reserved_keys_from_metadata() {
    let txt = HashMap::from([
        ("provider_id".to_string(), "custom-id".to_string()),
        ("endpoint".to_string(), "http://override:1".to_string()),
        ("capability".to_string(), "storage".to_string()),
        ("region".to_string(), "us-east".to_string()),
    ]);
    let info = MDnsSource::parse_txt_records("svc", "h", 80, &txt);
    assert_eq!(info.provider_id, "custom-id");
    assert!(matches!(info.capability, CapabilityType::Storage { .. }));
    assert!(!info.metadata.contains_key("provider_id"));
    assert!(!info.metadata.contains_key("endpoint"));
    assert!(!info.metadata.contains_key("capability"));
    assert_eq!(info.metadata.get("region"), Some(&"us-east".to_string()));
}

#[cfg(feature = "mdns")]
#[test]
fn mdns_parse_txt_records_invalid_endpoint_falls_back_to_host_port() {
    let txt = HashMap::from([
        ("endpoint".to_string(), "tcp://bad".to_string()),
        ("capability".to_string(), "compute".to_string()),
    ]);
    let info = MDnsSource::parse_txt_records("svc", "mesh.local", 9090, &txt);
    assert!(matches!(
        info.endpoint,
        ServiceEndpoint::Http(ref url) if url == "http://mesh.local:9090"
    ));
    assert!(matches!(info.capability, CapabilityType::Compute { .. }));
}
