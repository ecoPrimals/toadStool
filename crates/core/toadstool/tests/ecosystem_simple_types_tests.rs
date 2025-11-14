//! Simple type tests for ecosystem.rs
//!
//! Tests cover basic type creation and validation

use std::collections::HashMap;
use std::time::Duration;
use toadstool::ecosystem::*;

#[cfg(test)]
mod ecosystem_config_tests {
    use super::*;

    #[test]
    fn test_ecosystem_config_default() {
        let config = EcosystemConfig::default();

        assert!(config.auto_discovery);
        assert_eq!(config.discovery_timeout, Duration::from_secs(30));
        assert!(config.primal_endpoints.is_empty());
        assert!(config.required_primals.is_empty());
        assert_eq!(config.optional_primals.len(), 5);
    }

    #[test]
    fn test_ecosystem_config_optional_primals() {
        let config = EcosystemConfig::default();

        assert!(config.optional_primals.contains(&"songbird".to_string()));
        assert!(config.optional_primals.contains(&"nestgate".to_string()));
        assert!(config.optional_primals.contains(&"beardog".to_string()));
        assert!(config.optional_primals.contains(&"squirrel".to_string()));
        assert!(config.optional_primals.contains(&"biomeos".to_string()));
    }

    #[test]
    fn test_ecosystem_config_custom() {
        let mut endpoints = HashMap::new();
        endpoints.insert("songbird".to_string(), "http://localhost:8001".to_string());

        let config = EcosystemConfig {
            auto_discovery: false,
            discovery_timeout: Duration::from_secs(60),
            primal_endpoints: endpoints,
            required_primals: vec!["songbird".to_string()],
            optional_primals: vec!["squirrel".to_string()],
        };

        assert!(!config.auto_discovery);
        assert_eq!(config.discovery_timeout, Duration::from_secs(60));
        assert_eq!(config.required_primals.len(), 1);
    }

    #[test]
    fn test_ecosystem_config_clone() {
        let config = EcosystemConfig::default();
        let cloned = config.clone();

        assert_eq!(config.auto_discovery, cloned.auto_discovery);
        assert_eq!(config.discovery_timeout, cloned.discovery_timeout);
    }
}

#[cfg(test)]
mod primal_type_tests {
    use super::*;

    #[test]
    fn test_primal_type_variants() {
        let types = vec![
            PrimalType::Songbird,
            PrimalType::NestGate,
            PrimalType::BearDog,
            PrimalType::Squirrel,
            PrimalType::BiomeOS,
            PrimalType::ToadStool,
        ];

        assert_eq!(types.len(), 6);
    }

    #[test]
    fn test_primal_type_custom() {
        let custom1 = PrimalType::Custom("Service1".to_string());
        let custom2 = PrimalType::Custom("Service1".to_string());
        let custom3 = PrimalType::Custom("Service2".to_string());

        assert_eq!(custom1, custom2);
        assert_ne!(custom1, custom3);
    }

    #[test]
    fn test_primal_type_equality() {
        assert_eq!(PrimalType::Songbird, PrimalType::Songbird);
        assert_ne!(PrimalType::Songbird, PrimalType::NestGate);
    }

    #[test]
    fn test_primal_type_clone() {
        let primal_type = PrimalType::Squirrel;
        let cloned = primal_type.clone();
        assert_eq!(primal_type, cloned);
    }
}

#[cfg(test)]
mod primal_status_tests {
    use super::*;

    #[test]
    fn test_primal_status_discovered() {
        let status = PrimalStatus::Discovered;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_primal_status_connected() {
        let status = PrimalStatus::Connected;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_primal_status_failed() {
        let status = PrimalStatus::Failed("connection error".to_string());
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_primal_status_disconnected() {
        let status = PrimalStatus::Disconnected;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_primal_status_transitions() {
        // Test typical status flow
        let statuses = vec![
            PrimalStatus::Discovered,
            PrimalStatus::Connected,
            PrimalStatus::Disconnected,
        ];

        assert_eq!(statuses.len(), 3);
    }

    #[test]
    fn test_primal_status_equality() {
        assert_eq!(PrimalStatus::Connected, PrimalStatus::Connected);
        assert_ne!(PrimalStatus::Connected, PrimalStatus::Disconnected);
    }
}

#[cfg(test)]
mod primal_instance_tests {
    use super::*;

    #[test]
    fn test_primal_instance_creation() {
        let now = chrono::Utc::now();

        let instance = PrimalInstance {
            name: "songbird-01".to_string(),
            primal_type: PrimalType::Songbird,
            endpoint: "http://localhost:8001".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["discovery".to_string()],
            status: PrimalStatus::Connected,
            discovered_at: now,
        };

        assert_eq!(instance.name, "songbird-01");
        assert_eq!(instance.primal_type, PrimalType::Songbird);
    }

    #[test]
    fn test_primal_instance_capabilities() {
        let instance = PrimalInstance {
            name: "beardog-01".to_string(),
            primal_type: PrimalType::BearDog,
            endpoint: "https://security.local".to_string(),
            version: "2.0.0".to_string(),
            capabilities: vec!["authentication".to_string(), "encryption".to_string()],
            status: PrimalStatus::Connected,
            discovered_at: chrono::Utc::now(),
        };

        assert_eq!(instance.capabilities.len(), 2);
        assert!(instance
            .capabilities
            .contains(&"authentication".to_string()));
    }

    #[test]
    fn test_primal_instance_clone() {
        let instance = PrimalInstance {
            name: "test-primal".to_string(),
            primal_type: PrimalType::Squirrel,
            endpoint: "http://ai.local".to_string(),
            version: "3.0.0".to_string(),
            capabilities: vec![],
            status: PrimalStatus::Discovered,
            discovered_at: chrono::Utc::now(),
        };

        let cloned = instance.clone();
        assert_eq!(instance.name, cloned.name);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_primal_discovery_workflow() {
        let mut discovered: HashMap<String, PrimalInstance> = HashMap::new();

        let instance = PrimalInstance {
            name: "songbird".to_string(),
            primal_type: PrimalType::Songbird,
            endpoint: "http://localhost:8001".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["discovery".to_string()],
            status: PrimalStatus::Discovered,
            discovered_at: chrono::Utc::now(),
        };

        discovered.insert("songbird".to_string(), instance);

        assert_eq!(discovered.len(), 1);
        assert!(discovered.contains_key("songbird"));
    }

    #[test]
    fn test_primal_status_update() {
        let mut primals: HashMap<String, PrimalStatus> = HashMap::new();

        primals.insert("test".to_string(), PrimalStatus::Discovered);
        assert_eq!(primals.get("test"), Some(&PrimalStatus::Discovered));

        primals.insert("test".to_string(), PrimalStatus::Connected);
        assert_eq!(primals.get("test"), Some(&PrimalStatus::Connected));
    }

    #[test]
    fn test_ecosystem_setup() {
        let config = EcosystemConfig {
            auto_discovery: true,
            discovery_timeout: Duration::from_secs(30),
            primal_endpoints: HashMap::new(),
            required_primals: vec!["songbird".to_string()],
            optional_primals: vec!["beardog".to_string()],
        };

        assert!(config.required_primals.contains(&"songbird".to_string()));
        assert!(config.optional_primals.contains(&"beardog".to_string()));
    }
}
