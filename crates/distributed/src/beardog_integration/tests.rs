//! BearDog integration tests

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::beardog_integration::types::{
        BearDogCapability, BearDogEndpoint, EncryptionOperation, EncryptionRequest,
        EncryptionResponse, KeyManagementRequest, KeyManagementResponse, KeyOperation,
        KeyOperationResult, SecurityLevel, SignatureRequest, VerificationRequest,
    };
    use crate::beardog_integration::{
        BearDogClient, BearDogConfig, BearDogDiscovery, ServiceLocation,
    };
    use toadstool::CryptoProvider;

    #[test]
    #[allow(deprecated)]
    fn test_beardog_discovery_new() {
        let config = BearDogConfig::default();
        let discovery = BearDogDiscovery::new(config);
        assert!(discovery.config().auto_discover);
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_new() {
        let config = BearDogConfig::default();
        let _client = BearDogClient::new(config);
        // Client created successfully
    }

    #[test]
    fn test_beardog_config_default() {
        let config = BearDogConfig::default();
        assert!(config.auto_discover);
        assert_eq!(config.discovery_timeout_ms, 5000);
        assert_eq!(config.preferred_location, ServiceLocation::Local);
        assert!(config.fallback_enabled);
    }

    #[test]
    fn test_service_location_variants() {
        assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
        assert_eq!(ServiceLocation::Network, ServiceLocation::Network);
        assert_eq!(ServiceLocation::Any, ServiceLocation::Any);
        assert_ne!(ServiceLocation::Local, ServiceLocation::Network);
    }

    #[tokio::test]
    async fn test_beardog_discovery_get_best_endpoint_empty() {
        let config = BearDogConfig::default();
        let discovery = BearDogDiscovery::new(config);
        let result = discovery.get_best_endpoint().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No BearDog endpoints"));
    }

    #[test]
    fn test_encryption_request_serialization() {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Encrypt,
            data: vec![1, 2, 3, 4, 5],
            key_id: Some("key-123".to_string()),
            algorithm: Some("aes-256-gcm".to_string()),
            security_level: SecurityLevel::Enhanced,
        };
        let json = serde_json::to_value(&request);
        assert!(json.is_ok());
        let parsed: Result<EncryptionRequest, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
        let p = parsed.unwrap();
        assert_eq!(p.data, vec![1, 2, 3, 4, 5]);
        assert_eq!(p.key_id.as_deref(), Some("key-123"));
    }

    #[test]
    fn test_encryption_response_serialization() {
        let response = EncryptionResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![10, 20, 30],
            key_id: "key-456".to_string(),
            algorithm: "chacha20".to_string(),
            metadata: serde_json::json!({"nonce": "abc"}),
        };
        let json = serde_json::to_value(&response);
        assert!(json.is_ok());
        let parsed: Result<EncryptionResponse, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_signature_request_serialization() {
        let request = SignatureRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3],
            key_id: None,
            algorithm: Some("ed25519".to_string()),
        };
        let json = serde_json::to_value(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_verification_request_serialization() {
        let request = VerificationRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3],
            signature: vec![4, 5, 6],
            public_key_id: "pub-key-1".to_string(),
        };
        let json = serde_json::to_value(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_key_management_request_serialization() {
        let request = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: KeyOperation::Generate,
            key_id: None,
            security_level: Some(SecurityLevel::Standard),
        };
        let json = serde_json::to_value(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_key_operation_result_serialization() {
        let result = KeyOperationResult::Generated {
            key_id: "gen-key-1".to_string(),
            algorithm: "aes-256".to_string(),
        };
        let json = serde_json::to_value(&result);
        assert!(json.is_ok());
    }

    #[test]
    fn test_bear_dog_endpoint_serialization() {
        let endpoint = BearDogEndpoint {
            service_id: "beardog-1".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8081".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::Encryption {
                algorithms: vec!["aes-256".to_string()],
            }],
            healthy: true,
            latency_ms: Some(5),
        };
        let json = serde_json::to_value(&endpoint);
        assert!(json.is_ok());
    }

    #[test]
    fn test_bear_dog_capability_variants() {
        let enc = BearDogCapability::Encryption {
            algorithms: vec!["aes".to_string()],
        };
        assert!(matches!(enc, BearDogCapability::Encryption { .. }));
        let key = BearDogCapability::KeyManagement;
        assert!(matches!(key, BearDogCapability::KeyManagement));
    }

    #[test]
    fn test_security_level_ordering() {
        assert!(SecurityLevel::Standard < SecurityLevel::Enhanced);
        assert!(SecurityLevel::Enhanced < SecurityLevel::HardwareSecured);
    }

    #[tokio::test]
    async fn test_beardog_discovery_preferred_location_local() {
        let config = BearDogConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = BearDogDiscovery::new(config);
        let result = discovery.discover().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_beardog_discovery_preferred_location_any() {
        let config = BearDogConfig {
            preferred_location: ServiceLocation::Any,
            ..Default::default()
        };
        let discovery = BearDogDiscovery::new(config);
        let result = discovery.discover().await;
        assert!(result.is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_provider_id() {
        let config = BearDogConfig::default();
        let client = BearDogClient::new(config).unwrap();
        assert_eq!(client.provider_id(), "beardog");
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_capabilities() {
        let config = BearDogConfig::default();
        let client = BearDogClient::new(config).unwrap();
        let caps = client.capabilities();
        assert!(!caps.algorithms.is_empty());
    }

    #[test]
    fn test_signature_response_serialization() {
        use crate::beardog_integration::types::SignatureResponse;

        let resp = SignatureResponse {
            request_id: uuid::Uuid::new_v4(),
            signature: vec![1, 2, 3, 4],
            key_id: "key-1".to_string(),
            algorithm: "ed25519".to_string(),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
        let parsed: Result<SignatureResponse, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_permission_response_serialization() {
        use crate::beardog_integration::types::PermissionResponse;

        let resp = PermissionResponse {
            request_id: uuid::Uuid::new_v4(),
            permission_id: uuid::Uuid::new_v4(),
            proof: vec![5, 6, 7],
            metadata: serde_json::json!({"scope": "read"}),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
    }

    #[test]
    fn test_validation_response_serialization() {
        use crate::beardog_integration::types::ValidationResponse;

        let resp = ValidationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: true,
            details: Some("ok".to_string()),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
        let parsed: Result<ValidationResponse, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
        assert!(parsed.unwrap().valid);
    }

    #[test]
    fn test_revocation_request_serialization() {
        use crate::beardog_integration::types::RevocationRequest;

        let req = RevocationRequest {
            reason: "expired".to_string(),
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_key_management_response_serialization() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Deleted {
                key_id: "del-key".to_string(),
            },
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
    }

    #[test]
    fn test_key_operation_result_all_variants_serialization() {
        let deleted = KeyOperationResult::Deleted {
            key_id: "k1".to_string(),
        };
        assert!(serde_json::to_value(&deleted).is_ok());

        let listed = KeyOperationResult::Listed {
            keys: vec!["k1".to_string(), "k2".to_string()],
        };
        assert!(serde_json::to_value(&listed).is_ok());

        let err = KeyOperationResult::Error {
            message: "failed".to_string(),
        };
        assert!(serde_json::to_value(&err).is_ok());

        let retrieved = KeyOperationResult::Retrieved {
            key_id: "k1".to_string(),
            key_material: vec![0, 1, 2],
            algorithm: "aes".to_string(),
        };
        let json = serde_json::to_value(&retrieved).unwrap();
        let back: KeyOperationResult = serde_json::from_value(json).unwrap();
        assert!(matches!(back, KeyOperationResult::Retrieved { .. }));
    }

    #[test]
    fn test_encryption_operation_serde_roundtrip() {
        let enc = EncryptionOperation::Encrypt;
        let json = serde_json::to_string(&enc).unwrap();
        let _: EncryptionOperation = serde_json::from_str(&json).unwrap();
        let dec = EncryptionOperation::Decrypt;
        let json2 = serde_json::to_string(&dec).unwrap();
        let _: EncryptionOperation = serde_json::from_str(&json2).unwrap();
    }

    #[test]
    fn test_bear_dog_config_variations() {
        let config = BearDogConfig {
            auto_discover: false,
            discovery_timeout_ms: 10000,
            preferred_location: ServiceLocation::Network,
            fallback_enabled: false,
        };
        assert!(!config.auto_discover);
        assert_eq!(config.discovery_timeout_ms, 10000);
        assert_eq!(config.preferred_location, ServiceLocation::Network);
    }

    #[test]
    fn test_bear_dog_endpoint_serde_roundtrip() {
        let endpoint = BearDogEndpoint {
            service_id: "ep-1".to_string(),
            protocol: "unix".to_string(),
            address: "127.0.0.1:9000".parse().unwrap(),
            api_version: "v2".to_string(),
            capabilities: vec![
                BearDogCapability::Encryption {
                    algorithms: vec!["aes-256-gcm".to_string()],
                },
                BearDogCapability::KeyManagement,
            ],
            healthy: false,
            latency_ms: None,
        };
        let json = serde_json::to_string(&endpoint).unwrap();
        let parsed: BearDogEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.service_id, endpoint.service_id);
        assert_eq!(parsed.capabilities.len(), 2);
    }

    #[test]
    fn test_bear_dog_capability_all_variants_serde() {
        let custom = BearDogCapability::Custom("my-cap".to_string());
        let json = serde_json::to_value(&custom).unwrap();
        let back: BearDogCapability = serde_json::from_value(json).unwrap();
        assert!(matches!(back, BearDogCapability::Custom(s) if s == "my-cap"));

        let key = BearDogCapability::KeyManagement;
        let json = serde_json::to_value(&key).unwrap();
        let _: BearDogCapability = serde_json::from_value(json).unwrap();
    }

    #[test]
    fn test_security_level_serde_roundtrip() {
        for level in [
            SecurityLevel::Standard,
            SecurityLevel::Enhanced,
            SecurityLevel::HardwareSecured,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let _: SecurityLevel = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_key_operation_variants_serde() {
        for op in [
            KeyOperation::Generate,
            KeyOperation::Get,
            KeyOperation::Delete,
            KeyOperation::List,
        ] {
            let json = serde_json::to_value(&op).unwrap();
            let _: KeyOperation = serde_json::from_value(json).unwrap();
        }
    }

    #[tokio::test]
    async fn test_beardog_discovery_preferred_location_network() {
        let config = BearDogConfig {
            preferred_location: ServiceLocation::Network,
            ..Default::default()
        };
        let discovery = BearDogDiscovery::new(config);
        let result = discovery.discover().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_verification_response_serialization() {
        use crate::beardog_integration::types::VerificationResponse;

        let resp = VerificationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: false,
            details: Some("invalid sig".to_string()),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
    }

    #[test]
    fn test_encryption_request_roundtrip() {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Decrypt,
            data: vec![0xFF, 0xFE],
            key_id: None,
            algorithm: Some("chacha20".to_string()),
            security_level: SecurityLevel::HardwareSecured,
        };
        let json = serde_json::to_string(&request).unwrap();
        let parsed: EncryptionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.data, request.data);
        assert_eq!(parsed.security_level, SecurityLevel::HardwareSecured);
    }

    #[test]
    fn test_key_management_response_generated_roundtrip() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Generated {
                key_id: "gen-123".to_string(),
                algorithm: "aes-256-gcm".to_string(),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_str(&json).unwrap();
        match &parsed.result {
            KeyOperationResult::Generated { key_id, algorithm } => {
                assert_eq!(key_id, "gen-123");
                assert_eq!(algorithm, "aes-256-gcm");
            }
            _ => panic!("expected Generated"),
        }
    }

    #[test]
    fn test_key_operation_result_listed_serde() {
        let listed = KeyOperationResult::Listed {
            keys: vec!["k1".to_string(), "k2".to_string()],
        };
        let json = serde_json::to_value(&listed).unwrap();
        let back: KeyOperationResult = serde_json::from_value(json).unwrap();
        if let KeyOperationResult::Listed { keys } = back {
            assert_eq!(keys.len(), 2);
        } else {
            panic!("expected Listed");
        }
    }

    #[test]
    fn test_bear_dog_endpoint_debug_clone() {
        let ep = BearDogEndpoint {
            service_id: "ep-1".to_string(),
            protocol: "unix".to_string(),
            address: "127.0.0.1:9000".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::KeyManagement],
            healthy: true,
            latency_ms: Some(1),
        };
        let ep2 = ep.clone();
        assert_eq!(ep.service_id, ep2.service_id);
        assert!(format!("{:?}", ep).contains("ep-1"));
    }

    #[test]
    fn test_bear_dog_capability_hardware_security() {
        let cap = BearDogCapability::HardwareSecurity;
        assert!(matches!(cap, BearDogCapability::HardwareSecurity));
    }

    #[test]
    fn test_bear_dog_capability_secure_storage() {
        let cap = BearDogCapability::SecureStorage;
        assert!(matches!(cap, BearDogCapability::SecureStorage));
    }

    #[test]
    fn test_bear_dog_capability_genetic_entropy() {
        let cap = BearDogCapability::GeneticEntropy;
        assert!(matches!(cap, BearDogCapability::GeneticEntropy));
    }

    #[test]
    fn test_toadstool_error_not_found_display() {
        let err = toadstool_common::ToadStoolError::not_found("No BearDog endpoints");
        let s = err.to_string();
        assert!(s.to_lowercase().contains("bear"));
    }

    #[test]
    fn test_beardog_config_timeout_variations() {
        let config = BearDogConfig {
            auto_discover: true,
            discovery_timeout_ms: 1,
            preferred_location: ServiceLocation::Local,
            fallback_enabled: true,
        };
        assert_eq!(config.discovery_timeout_ms, 1);
    }

    #[test]
    fn test_permission_response_roundtrip() {
        use crate::beardog_integration::types::PermissionResponse;

        let resp = PermissionResponse {
            request_id: uuid::Uuid::new_v4(),
            permission_id: uuid::Uuid::new_v4(),
            proof: vec![1, 2, 3, 4, 5],
            metadata: serde_json::json!({"scope": "read"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: PermissionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.proof, resp.proof);
    }

    #[test]
    fn test_validation_response_valid_false() {
        use crate::beardog_integration::types::ValidationResponse;

        let resp = ValidationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: false,
            details: Some("expired".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ValidationResponse = serde_json::from_str(&json).unwrap();
        assert!(!parsed.valid);
    }

    #[test]
    fn test_signature_response_roundtrip() {
        use crate::beardog_integration::types::SignatureResponse;

        let resp = SignatureResponse {
            request_id: uuid::Uuid::new_v4(),
            signature: vec![0xDE, 0xAD],
            key_id: "sig-key".to_string(),
            algorithm: "ed25519".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: SignatureResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.signature, resp.signature);
    }

    #[test]
    fn test_revocation_request_roundtrip() {
        use crate::beardog_integration::types::RevocationRequest;

        let req = RevocationRequest {
            reason: "user request".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: RevocationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.reason, req.reason);
    }

    #[tokio::test]
    async fn test_beardog_discovery_get_best_endpoint_returns_lowest_latency() {
        let config = BearDogConfig::default();
        let endpoints = vec![
            BearDogEndpoint {
                service_id: "ep-slow".to_string(),
                protocol: "http".to_string(),
                address: "127.0.0.1:8081".parse().unwrap(),
                api_version: "v1".to_string(),
                capabilities: vec![BearDogCapability::Encryption {
                    algorithms: vec!["aes-256".to_string()],
                }],
                healthy: true,
                latency_ms: Some(50),
            },
            BearDogEndpoint {
                service_id: "ep-fast".to_string(),
                protocol: "http".to_string(),
                address: "127.0.0.1:8082".parse().unwrap(),
                api_version: "v1".to_string(),
                capabilities: vec![BearDogCapability::Encryption {
                    algorithms: vec!["aes-256".to_string()],
                }],
                healthy: true,
                latency_ms: Some(5),
            },
        ];
        let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
        let best = discovery.get_best_endpoint().await.unwrap();
        assert_eq!(best.service_id, "ep-fast");
        assert_eq!(best.latency_ms, Some(5));
    }

    #[tokio::test]
    async fn test_beardog_discovery_get_best_endpoint_all_unhealthy_returns_error() {
        let config = BearDogConfig::default();
        let endpoints = vec![BearDogEndpoint {
            service_id: "ep-unhealthy".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8081".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::KeyManagement],
            healthy: false,
            latency_ms: Some(100),
        }];
        let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
        let result = discovery.get_best_endpoint().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("healthy"));
    }

    #[tokio::test]
    async fn test_beardog_discovery_get_best_endpoint_no_latency_uses_max() {
        let config = BearDogConfig::default();
        let endpoints = vec![
            BearDogEndpoint {
                service_id: "ep-a".to_string(),
                protocol: "http".to_string(),
                address: "127.0.0.1:8081".parse().unwrap(),
                api_version: "v1".to_string(),
                capabilities: vec![BearDogCapability::KeyManagement],
                healthy: true,
                latency_ms: None,
            },
            BearDogEndpoint {
                service_id: "ep-b".to_string(),
                protocol: "http".to_string(),
                address: "127.0.0.1:8082".parse().unwrap(),
                api_version: "v1".to_string(),
                capabilities: vec![BearDogCapability::KeyManagement],
                healthy: true,
                latency_ms: Some(1),
            },
        ];
        let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
        let best = discovery.get_best_endpoint().await.unwrap();
        assert_eq!(best.service_id, "ep-b");
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_creation_with_custom_config() {
        let config = BearDogConfig {
            auto_discover: false,
            discovery_timeout_ms: 10000,
            preferred_location: ServiceLocation::Network,
            fallback_enabled: false,
        };
        let result = BearDogClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encryption_request_construction_for_encrypt() {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Encrypt,
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            key_id: Some("enc-key-1".to_string()),
            algorithm: Some("aes-256-gcm".to_string()),
            security_level: SecurityLevel::Enhanced,
        };
        let params = serde_json::to_value(&request).unwrap();
        assert!(params.get("operation").is_some());
        assert_eq!(params["data"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn test_encryption_request_construction_for_decrypt() {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Decrypt,
            data: vec![0xAA, 0xBB, 0xCC],
            key_id: Some("dec-key".to_string()),
            algorithm: Some("chacha20poly1305".to_string()),
            security_level: SecurityLevel::Standard,
        };
        let params = serde_json::to_value(&request).unwrap();
        assert!(params.get("key_id").is_some());
    }

    #[test]
    fn test_key_management_request_construction_generate() {
        let request = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: KeyOperation::Generate,
            key_id: None,
            security_level: Some(SecurityLevel::HardwareSecured),
        };
        let params = serde_json::to_value(&request).unwrap();
        assert!(params.get("operation").is_some());
    }

    #[test]
    fn test_key_management_request_construction_get() {
        let request = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: KeyOperation::Get,
            key_id: Some("fetch-key-123".to_string()),
            security_level: None,
        };
        let params = serde_json::to_value(&request).unwrap();
        assert_eq!(params["key_id"].as_str(), Some("fetch-key-123"));
    }

    #[test]
    fn test_key_management_response_parsing_success_generated() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Generated {
                key_id: "new-key-id".to_string(),
                algorithm: "aes-256-gcm".to_string(),
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_value(json).unwrap();
        match &parsed.result {
            KeyOperationResult::Generated { key_id, algorithm } => {
                assert_eq!(key_id, "new-key-id");
                assert_eq!(algorithm, "aes-256-gcm");
            }
            _ => panic!("expected Generated"),
        }
    }

    #[test]
    fn test_key_management_response_parsing_error() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Error {
                message: "key not found".to_string(),
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_value(json).unwrap();
        match &parsed.result {
            KeyOperationResult::Error { message } => assert_eq!(message, "key not found"),
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn test_encryption_response_parsing_success() {
        let resp = EncryptionResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![0x11, 0x22, 0x33],
            key_id: "key-456".to_string(),
            algorithm: "chacha20".to_string(),
            metadata: serde_json::json!({"iv": "abc123"}),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: EncryptionResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.data, vec![0x11, 0x22, 0x33]);
        assert_eq!(parsed.algorithm, "chacha20");
    }

    #[test]
    fn test_beardog_config_timeout_affects_discovery_timeout() {
        let config = BearDogConfig {
            discovery_timeout_ms: 2500,
            ..Default::default()
        };
        assert_eq!(config.discovery_timeout_ms, 2500);
    }

    #[test]
    fn test_beardog_config_fallback_disabled() {
        let config = BearDogConfig {
            fallback_enabled: false,
            ..Default::default()
        };
        assert!(!config.fallback_enabled);
    }

    #[test]
    fn test_signature_request_construction_with_algorithm() {
        let request = SignatureRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3, 4, 5],
            key_id: Some("sig-key".to_string()),
            algorithm: Some("ed25519".to_string()),
        };
        let params = serde_json::to_value(&request).unwrap();
        assert_eq!(params["algorithm"].as_str(), Some("ed25519"));
    }

    #[test]
    fn test_verification_request_construction() {
        let request = VerificationRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3],
            signature: vec![4, 5, 6, 7, 8],
            public_key_id: "pub-key-99".to_string(),
        };
        let params = serde_json::to_value(&request).unwrap();
        assert_eq!(params["public_key_id"].as_str(), Some("pub-key-99"));
    }

    #[test]
    fn test_revocation_request_construction() {
        use crate::beardog_integration::types::RevocationRequest;

        let request = RevocationRequest {
            reason: "security incident".to_string(),
        };
        let params = serde_json::to_value(&request).unwrap();
        assert_eq!(params["reason"].as_str(), Some("security incident"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_creation_default_config() {
        let config = BearDogConfig::default();
        let result = BearDogClient::new(config);
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.provider_id(), "beardog");
    }

    #[tokio::test]
    async fn test_beardog_discovery_with_endpoints_injects_mock_data() {
        let config = BearDogConfig::default();
        let mock_endpoints = vec![BearDogEndpoint {
            service_id: "mock-1".to_string(),
            protocol: "unix".to_string(),
            address: "127.0.0.1:9090".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::KeyManagement],
            healthy: true,
            latency_ms: Some(2),
        }];
        let discovery = BearDogDiscovery::with_endpoints(config, mock_endpoints);
        let best = discovery.get_best_endpoint().await.unwrap();
        assert_eq!(best.service_id, "mock-1");
    }

    #[test]
    fn test_toadstool_error_configuration_display() {
        let err = toadstool_common::ToadStoolError::configuration("Config invalid");
        assert!(err.to_string().to_lowercase().contains("config"));
    }

    #[test]
    fn test_toadstool_error_runtime_display() {
        let err = toadstool_common::ToadStoolError::runtime("Runtime failure");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_toadstool_error_network_display() {
        let err = toadstool_common::ToadStoolError::network("Network error");
        assert!(err.to_string().len() > 0);
    }

    #[test]
    fn test_toadstool_error_security_display() {
        let err = toadstool_common::ToadStoolError::security("Security violation");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_verification_response_roundtrip() {
        use crate::beardog_integration::types::VerificationResponse;

        let resp = VerificationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: true,
            details: Some("signature valid".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: VerificationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.valid, resp.valid);
        assert_eq!(parsed.details, resp.details);
    }

    #[test]
    fn test_key_operation_result_deleted_serde_roundtrip() {
        let deleted = KeyOperationResult::Deleted {
            key_id: "k-deleted".to_string(),
        };
        let json = serde_json::to_value(&deleted).unwrap();
        let back: KeyOperationResult = serde_json::from_value(json).unwrap();
        assert!(matches!(back, KeyOperationResult::Deleted { key_id } if key_id == "k-deleted"));
    }

    #[test]
    fn test_bear_dog_config_debug_and_fields() {
        let config = BearDogConfig {
            auto_discover: true,
            discovery_timeout_ms: 3000,
            preferred_location: ServiceLocation::Any,
            fallback_enabled: true,
        };
        let dbg = format!("{:?}", config);
        assert!(!dbg.is_empty());
        assert_eq!(config.discovery_timeout_ms, 3000);
    }

    #[test]
    fn test_service_location_all_variants_eq() {
        assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
        assert_eq!(ServiceLocation::Network, ServiceLocation::Network);
        assert_eq!(ServiceLocation::Any, ServiceLocation::Any);
    }

    #[test]
    fn test_encryption_operation_all_variants() {
        let enc = EncryptionOperation::Encrypt;
        let dec = EncryptionOperation::Decrypt;
        assert_eq!(enc, EncryptionOperation::Encrypt);
        assert_ne!(enc, dec);
    }

    #[test]
    fn test_bear_dog_endpoint_all_capability_variants_serde() {
        let enc = BearDogCapability::Encryption {
            algorithms: vec!["aes".to_string()],
        };
        let _ = serde_json::to_value(&enc).unwrap();
        let hw = BearDogCapability::HardwareSecurity;
        let _ = serde_json::to_value(&hw).unwrap();
        let ss = BearDogCapability::SecureStorage;
        let _ = serde_json::to_value(&ss).unwrap();
        let ge = BearDogCapability::GeneticEntropy;
        let _ = serde_json::to_value(&ge).unwrap();
    }

    #[test]
    fn test_key_management_response_deleted_roundtrip() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Deleted {
                key_id: "del-123".to_string(),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed.result, KeyOperationResult::Deleted { .. }));
    }

    #[test]
    fn test_encryption_response_full_roundtrip_with_metadata() {
        let resp = EncryptionResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            key_id: "key-789".to_string(),
            algorithm: "aes-256-gcm".to_string(),
            metadata: serde_json::json!({"nonce": "abc123", "tag": "xyz"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: EncryptionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.data, resp.data);
        assert_eq!(parsed.metadata["nonce"], "abc123");
    }

    #[test]
    fn test_signature_request_with_key_and_algorithm() {
        let req = SignatureRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3, 4, 5, 6],
            key_id: Some("sig-key".to_string()),
            algorithm: Some("ed25519".to_string()),
        };
        let params = serde_json::to_value(&req).unwrap();
        assert_eq!(params["key_id"].as_str(), Some("sig-key"));
        assert_eq!(params["algorithm"].as_str(), Some("ed25519"));
    }

    #[test]
    fn test_validation_response_details_none() {
        use crate::beardog_integration::types::ValidationResponse;

        let resp = ValidationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: true,
            details: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ValidationResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.valid);
        assert!(parsed.details.is_none());
    }
}
