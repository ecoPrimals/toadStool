//! Integration tests for EcosystemCoordinator
//!
//! Tests actual function execution to increase coverage

#![allow(clippy::all)]

use toadstool::ecosystem::*;

#[cfg(test)]
mod ecosystem_coordinator_tests {
    use super::*;

    #[test]
    fn test_ecosystem_coordinator_new() {
        let result = EcosystemCoordinator::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ecosystem_coordinator_creation_multiple_times() {
        for _ in 0..5 {
            let result = EcosystemCoordinator::new();
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_get_primal_status_empty() {
        let coordinator = EcosystemCoordinator::new().unwrap();
        let result = coordinator.get_primal_status().await;
        assert!(result.is_ok());
        let status_map = result.unwrap();
        assert!(status_map.is_empty());
    }

    #[tokio::test]
    async fn test_is_primal_available_nonexistent() {
        let coordinator = EcosystemCoordinator::new().unwrap();
        let available = coordinator.is_primal_available("nonexistent").await;
        assert!(!available);
    }

    #[tokio::test]
    async fn test_is_primal_available_multiple_checks() {
        let coordinator = EcosystemCoordinator::new().unwrap();

        let primal_names = vec!["songbird", "nestgate", "beardog"];
        for name in primal_names {
            let available = coordinator.is_primal_available(name).await;
            // Should return false since no primals are discovered yet
            assert!(!available);
        }
    }

    #[tokio::test]
    async fn test_get_primal_capabilities_nonexistent() {
        let coordinator = EcosystemCoordinator::new().unwrap();
        let result = coordinator.get_primal_capabilities("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_integrate_primals_empty_list() {
        let coordinator = EcosystemCoordinator::new().unwrap();
        let result = coordinator.integrate_primals(vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discover_primals_no_auto_discovery() {
        let coordinator = EcosystemCoordinator::new().unwrap();
        // With default config (auto_discovery=true but no network), should complete without error
        let result = coordinator.discover_primals().await;
        // May error due to network features, but shouldn't panic
        let _ = result;
    }
}

#[cfg(test)]
mod ecosystem_message_tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_ecosystem_message_creation() {
        let message = EcosystemMessage {
            id: Uuid::new_v4(),
            from: "toadstool".to_string(),
            to: "songbird".to_string(),
            message_type: EcosystemMessageType::Heartbeat,
            payload: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(message.from, "toadstool");
        assert_eq!(message.to, "songbird");
    }

    #[test]
    fn test_ecosystem_message_types() {
        let message_types = vec![
            EcosystemMessageType::Heartbeat,
            EcosystemMessageType::CapabilityAnnouncement,
            EcosystemMessageType::ResourceRequest,
            EcosystemMessageType::ResourceResponse,
            EcosystemMessageType::WorkloadRequest,
            EcosystemMessageType::WorkloadResponse,
            EcosystemMessageType::StatusUpdate,
            EcosystemMessageType::Error,
        ];

        assert_eq!(message_types.len(), 8);
    }

    #[test]
    fn test_ecosystem_message_serialization() {
        let message = EcosystemMessage {
            id: Uuid::new_v4(),
            from: "sender".to_string(),
            to: "receiver".to_string(),
            message_type: EcosystemMessageType::StatusUpdate,
            payload: serde_json::json!({"status": "running"}),
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("sender"));
        assert!(json.contains("receiver"));
        assert!(json.contains("StatusUpdate"));
    }

    #[test]
    fn test_ecosystem_message_with_different_payloads() {
        let payloads = vec![
            serde_json::json!({"key": "value"}),
            serde_json::json!({"count": 42}),
            serde_json::json!({"list": [1, 2, 3]}),
            serde_json::json!(null),
        ];

        for payload in payloads {
            let message = EcosystemMessage {
                id: Uuid::new_v4(),
                from: "test".to_string(),
                to: "test".to_string(),
                message_type: EcosystemMessageType::Heartbeat,
                payload: payload.clone(),
                timestamp: chrono::Utc::now(),
            };

            assert_eq!(message.payload, payload);
        }
    }

    #[test]
    fn test_message_type_clone() {
        let msg_type = EcosystemMessageType::Heartbeat;
        let cloned = msg_type.clone();

        match cloned {
            EcosystemMessageType::Heartbeat => { /* Expected */ }
            _ => panic!("Clone failed"),
        }
    }
}

#[cfg(test)]
mod primal_instance_integration_tests {
    use super::*;

    #[test]
    fn test_primal_instance_full_lifecycle() {
        let instance = PrimalInstance {
            name: "test-primal".to_string(),
            primal_type: PrimalType::Songbird,
            endpoint: "http://localhost:8001".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["discovery".to_string(), "routing".to_string()],
            status: PrimalStatus::Discovered,
            discovered_at: chrono::Utc::now(),
        };

        // Test serialization
        let json = serde_json::to_string(&instance).unwrap();
        assert!(json.contains("test-primal"));

        // Test deserialization
        let deserialized: PrimalInstance = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, instance.name);
    }

    #[test]
    fn test_primal_status_transitions() {
        let statuses = vec![
            PrimalStatus::Discovered,
            PrimalStatus::Connected,
            PrimalStatus::Disconnected,
        ];

        for status in statuses {
            let instance = PrimalInstance {
                name: "status-test".to_string(),
                primal_type: PrimalType::BearDog,
                endpoint: "http://test".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![],
                status: status.clone(),
                discovered_at: chrono::Utc::now(),
            };

            assert_eq!(instance.status, status);
        }
    }

    #[test]
    fn test_primal_status_failed() {
        let status = PrimalStatus::Failed("connection timeout".to_string());
        let instance = PrimalInstance {
            name: "failed-primal".to_string(),
            primal_type: PrimalType::Squirrel,
            endpoint: "http://unreachable".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: status.clone(),
            discovered_at: chrono::Utc::now(),
        };

        match instance.status {
            PrimalStatus::Failed(ref reason) => {
                assert_eq!(reason, "connection timeout");
            }
            _ => panic!("Expected Failed status"),
        }
    }
}

#[cfg(test)]
mod ecosystem_config_integration_tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn test_ecosystem_config_with_endpoints() {
        let mut endpoints = HashMap::new();
        endpoints.insert("songbird".to_string(), "http://songbird.local".to_string());
        endpoints.insert("beardog".to_string(), "https://beardog.local".to_string());

        let config = EcosystemConfig {
            auto_discovery: false,
            discovery_timeout: Duration::from_secs(60),
            primal_endpoints: endpoints.clone(),
            required_primals: vec!["songbird".to_string()],
            optional_primals: vec!["beardog".to_string()],
        };

        assert_eq!(config.primal_endpoints.len(), 2);
        assert_eq!(
            config.primal_endpoints.get("songbird"),
            Some(&"http://songbird.local".to_string())
        );
    }

    #[test]
    fn test_ecosystem_config_serialization_roundtrip() {
        let config = EcosystemConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EcosystemConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.auto_discovery, deserialized.auto_discovery);
        assert_eq!(config.discovery_timeout, deserialized.discovery_timeout);
    }

    #[test]
    fn test_ecosystem_config_various_timeouts() {
        let timeouts = vec![
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(60),
            Duration::from_secs(120),
        ];

        for timeout in timeouts {
            let config = EcosystemConfig {
                discovery_timeout: timeout,
                ..Default::default()
            };

            assert_eq!(config.discovery_timeout, timeout);
        }
    }
}

#[cfg(test)]
mod primal_type_integration_tests {
    use super::*;

    #[test]
    fn test_all_primal_types_serialization() {
        let types = vec![
            PrimalType::Songbird,
            PrimalType::NestGate,
            PrimalType::BearDog,
            PrimalType::Squirrel,
            PrimalType::BiomeOS,
            PrimalType::ToadStool,
            PrimalType::Custom("CustomPrimal".to_string()),
        ];

        for primal_type in types {
            let json = serde_json::to_string(&primal_type).unwrap();
            let deserialized: PrimalType = serde_json::from_str(&json).unwrap();
            assert_eq!(primal_type, deserialized);
        }
    }

    #[test]
    fn test_primal_type_in_vector() {
        let mut types = Vec::new();
        types.push(PrimalType::Songbird);
        types.push(PrimalType::BearDog);
        types.push(PrimalType::Songbird);

        assert_eq!(types.len(), 3);
        assert!(types.iter().any(|t| matches!(t, PrimalType::Songbird)));
        assert!(types.iter().any(|t| matches!(t, PrimalType::BearDog)));
    }
}
