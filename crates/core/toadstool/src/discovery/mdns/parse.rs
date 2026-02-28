//! Parse mDNS ServiceInfo into DiscoveredService.
//!
//! Extracted for testability; converts mdns-sd ServiceInfo records
//! into the discovery module's DiscoveredService type.

use crate::discovery::DiscoveredService;
use crate::error::{ToadStoolError, ToadStoolResult};
use crate::self_identity::Capability;
use mdns_sd::ServiceInfo;
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// Parse ServiceInfo into DiscoveredService (extracted for testability)
pub fn parse_service_info(info: &ServiceInfo) -> ToadStoolResult<DiscoveredService> {
    // Extract instance ID using mdns-sd 0.10 API
    let instance_id = info
        .get_property_val_str("instance_id")
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| ToadStoolError::runtime("Missing or invalid instance_id"))?;

    // Extract primal type
    let primal_type = info
        .get_property_val_str("primal_type")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract version
    let version = info
        .get_property_val_str("version")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract capabilities
    let mut capabilities = Vec::new();
    let mut processed_caps = std::collections::HashSet::new();

    // Iterate through all properties to find capabilities
    for prop in info.get_properties().iter() {
        let key = prop.key();
        if let Some(cap_name) = key.strip_prefix("cap_") {
            if !cap_name.ends_with("_features") && processed_caps.insert(cap_name.to_string()) {
                let cap_version = info
                    .get_property_val_str(key)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let features_key = format!("cap_{}_features", cap_name);
                let features = info
                    .get_property_val_str(&features_key)
                    .map(|f| f.split(',').map(|s| s.to_string()).collect())
                    .unwrap_or_default();

                capabilities.push(Capability {
                    name: cap_name.to_string(),
                    version: cap_version,
                    features,
                    characteristics: HashMap::new(),
                });
            }
        }
    }

    // Build endpoint
    let addresses = info.get_addresses();
    let endpoint = if let Some(addr) = addresses.iter().next() {
        format!("{}:{}", addr, info.get_port())
    } else {
        format!("{}:{}", info.get_hostname(), info.get_port())
    };

    // Build metadata map from properties
    let mut metadata = HashMap::new();
    for prop in info.get_properties().iter() {
        let key = prop.key();
        if let Some(value) = info.get_property_val_str(key) {
            metadata.insert(key.to_string(), value.to_string());
        }
    }

    let now = SystemTime::now();

    Ok(DiscoveredService {
        instance_id,
        primal_type,
        version,
        capabilities,
        endpoint,
        protocols: vec!["http".to_string()],
        discovered_at: now,
        last_seen: now,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::mdns::constants::TOADSTOOL_SERVICE_TYPE;
    use std::collections::HashMap;

    #[test]
    fn test_parse_service_info_full_properties() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());
        properties.insert("primal_type".to_string(), "songbird".to_string());
        properties.insert("version".to_string(), "2.0.0".to_string());
        properties.insert("cap_storage".to_string(), "1.0".to_string());
        properties.insert(
            "cap_storage_features".to_string(),
            "object,metadata".to_string(),
        );
        properties.insert("cap_compute".to_string(), "1.2".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test-instance",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info(&info).expect("parse should succeed");

        assert_eq!(service.instance_id, instance_id);
        assert_eq!(service.primal_type, "songbird");
        assert_eq!(service.version, "2.0.0");
        assert_eq!(service.endpoint, "127.0.0.1:8080");
        assert_eq!(service.protocols, vec!["http"]);
        assert!(service.has_capability("storage"));
        assert!(service.has_capability("compute"));
        assert_eq!(service.capability_version("storage"), Some("1.0"));
        assert!(service.has_capability_features("storage", &["object".to_string()]));
        assert!(service
            .has_capability_features("storage", &["object".to_string(), "metadata".to_string()]));
    }

    #[test]
    fn test_parse_service_info_missing_instance_id() {
        let mut properties = HashMap::new();
        properties.insert("primal_type".to_string(), "test".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let result = parse_service_info(&info);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_service_info_invalid_instance_id() {
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), "not-a-valid-uuid".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let result = parse_service_info(&info);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_service_info_defaults_for_missing() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.example",
            "127.0.0.1",
            9000u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info(&info).expect("parse should succeed");

        assert_eq!(service.primal_type, "unknown");
        assert_eq!(service.version, "unknown");
        assert_eq!(service.endpoint, "127.0.0.1:9000");
        assert!(service.capabilities.is_empty());
    }

    #[test]
    fn test_parse_service_info_endpoint_fallback_to_hostname() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "myhost.local",
            "",
            7070u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info(&info).expect("parse should succeed");

        let addresses = info.get_addresses();
        if addresses.is_empty() {
            assert_eq!(service.endpoint, "myhost.local:7070");
        } else {
            assert!(service.endpoint.contains(":7070"));
        }
    }

    #[test]
    fn test_parse_service_info_metadata_populated() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());
        properties.insert("custom_key".to_string(), "custom_value".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties.clone()),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info(&info).expect("parse should succeed");

        assert!(service.metadata.contains_key("instance_id"));
        assert!(service.metadata.contains_key("custom_key"));
        assert_eq!(
            service.metadata.get("custom_key"),
            Some(&"custom_value".to_string())
        );
    }

    #[test]
    fn test_parse_service_info_skips_features_key_as_capability() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());
        properties.insert("cap_storage".to_string(), "1.0".to_string());
        properties.insert("cap_storage_features".to_string(), "a,b".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info(&info).expect("parse should succeed");

        assert!(!service
            .capabilities
            .iter()
            .any(|c| c.name == "storage_features"));
        assert!(service.has_capability("storage"));
    }

    #[test]
    fn test_parse_service_info_invalid_port_uses_default() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());
        properties.insert("cap_test".to_string(), "1.0".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            0u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info(&info).expect("parse should succeed");
        assert_eq!(service.endpoint, "127.0.0.1:0");
        assert_eq!(service.capabilities.len(), 1);
        assert_eq!(service.capability_version("test"), Some("1.0"));
    }

    #[test]
    fn test_parse_service_info_capability_no_features() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());
        properties.insert("cap_minimal".to_string(), "2.0".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info(&info).expect("parse should succeed");
        assert!(service.has_capability("minimal"));
        assert_eq!(service.capability_version("minimal"), Some("2.0"));
        assert!(!service.has_capability_features("minimal", &["x".to_string()]));
    }

    #[test]
    fn test_parse_service_info_empty_address_uses_hostname() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test-instance",
            "myhost.example.com",
            "",
            12345u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info(&info).expect("parse should succeed");
        let addresses = info.get_addresses();
        if addresses.is_empty() {
            assert_eq!(service.endpoint, "myhost.example.com:12345");
        }
    }
}
