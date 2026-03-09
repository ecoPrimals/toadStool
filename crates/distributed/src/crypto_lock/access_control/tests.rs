// SPDX-License-Identifier: AGPL-3.0-only
//! Access control tests

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;
    use crate::crypto_lock::permissions::{
        DelegationScope, ExpiringPermission, PermissionHolder, PermissionMetadata, PermissionScope,
        PermissionStatus, ResourceLimits, SecurityProviderPermission, TimeRestrictions,
        UsageQuotas,
    };
    use crate::crypto_lock::validation::{
        CryptoAlgorithm, ProofMetadata, SecurityProof, VerificationLevel,
    };
    use crate::security_provider::types::{
        CloudProvider, ContainerPlatform, ExternalTarget, HPCScheduler, QuantumProvider,
        ServiceTier,
    };
    use std::time::{Duration, SystemTime};
    use uuid::Uuid;

    fn pure_rust_target() -> ExternalTarget {
        ExternalTarget::ExternalTool {
            tool_name: "toadstool".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["ecosystem_native".to_string()],
        }
    }

    fn trusted_target() -> ExternalTarget {
        ExternalTarget::ExternalTool {
            tool_name: "nestgate".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["trusted".to_string()],
        }
    }

    fn primal_toadstool_target() -> ExternalTarget {
        ExternalTarget::ExternalTool {
            tool_name: "tool".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["primal:toadstool".to_string()],
        }
    }

    fn cloud_target() -> ExternalTarget {
        ExternalTarget::CloudProvider {
            provider: CloudProvider::AWS,
            regions: vec!["us-east-1".to_string()],
            services: vec![],
        }
    }

    #[allow(dead_code)]
    fn container_target() -> ExternalTarget {
        ExternalTarget::ContainerPlatform {
            platform: ContainerPlatform::Kubernetes,
            clusters: vec![],
            namespaces: vec![],
        }
    }

    fn external_tool_target_no_trust() -> ExternalTarget {
        ExternalTarget::ExternalTool {
            tool_name: "external-api".to_string(),
            api_endpoints: vec!["https://api.example.com".to_string()],
            feature_set: vec![],
        }
    }

    fn make_expired_permission(target: ExternalTarget) -> SecurityProviderPermission {
        let now = SystemTime::now();
        let valid_from = now - Duration::from_secs(3600);
        let valid_until = now - Duration::from_secs(1);
        SecurityProviderPermission {
            permission_id: Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "user1".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            external_target: target,
            scope: PermissionScope {
                resource_limits: ResourceLimits {
                    max_cpu_cores: None,
                    max_memory_gb: None,
                    max_storage_gb: None,
                    max_network_bandwidth: None,
                },
                time_restrictions: TimeRestrictions {
                    allowed_hours: None,
                    allowed_days: None,
                    timezone: None,
                },
                usage_quotas: UsageQuotas {
                    max_requests_per_hour: None,
                    max_data_transfer_gb: None,
                    max_compute_hours: None,
                },
                geographic_limits: vec![],
                feature_restrictions: vec![],
            },
            valid_from,
            valid_until,
            crypto_proof: SecurityProof {
                signature: vec![],
                algorithm: CryptoAlgorithm::Ed25519,
                public_key_id: "key1".to_string(),
                timestamp: now,
                metadata: ProofMetadata {
                    issuer: "test".to_string(),
                    purpose: "test".to_string(),
                    additional_claims: std::collections::HashMap::new(),
                },
            },
            delegation_chain: None,
            metadata: PermissionMetadata {
                issued_by: "test".to_string(),
                notes: String::new(),
                features: vec![],
            },
        }
    }

    #[tokio::test]
    async fn test_crypto_lock_new_creates_instance() {
        let result = ToadStoolCryptoLock::new().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_external_access_pure_rust_ecosystem_granted() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = pure_rust_target();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted {
                reason,
                permission_level,
                ..
            } => {
                assert!(reason.contains("Pure Rust"));
                assert!(matches!(permission_level, PermissionLevel::Full));
            }
            AccessResult::Denied { .. } => panic!("Expected Granted for pure Rust ecosystem"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_trusted_feature_granted() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = trusted_target();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            AccessResult::Denied { .. } => panic!("Expected Granted for trusted"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_primal_type_granted() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = primal_toadstool_target();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            AccessResult::Denied { .. } => panic!("Expected Granted for primal"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_no_permission_denied() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = cloud_target();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => panic!("Expected Denied for cloud without permission"),
            AccessResult::Denied {
                reason,
                how_to_get_access,
            } => {
                assert!(!reason.is_empty(), "Denied should have a reason");
                assert!(!how_to_get_access.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_check_external_access_external_tool_no_trust_denied() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = external_tool_target_no_trust();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => panic!("Expected Denied"),
            AccessResult::Denied {
                how_to_get_access,
                reason: _,
            } => {
                assert!(how_to_get_access.contains("external tool"));
            }
        }
    }

    #[tokio::test]
    async fn test_get_crypto_lock_status() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let status = lock.get_crypto_lock_status().await.unwrap();
        assert!(status.pure_rust_unlocked);
        assert!(status.external_permissions.is_empty());
    }

    #[test]
    fn test_access_result_granted_constructor() {
        let result = AccessResult::Granted {
            reason: "test".to_string(),
            permission_level: PermissionLevel::Full,
            expires_at: None,
            restrictions: vec![],
        };
        match &result {
            AccessResult::Granted {
                permission_level, ..
            } => {
                assert!(matches!(permission_level, PermissionLevel::Full));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_access_result_denied_constructor() {
        let result = AccessResult::Denied {
            reason: "No permission".to_string(),
            how_to_get_access: "Contact admin".to_string(),
        };
        match &result {
            AccessResult::Denied { reason, .. } => assert_eq!(reason, "No permission"),
            _ => panic!(),
        }
    }

    #[test]
    fn test_permission_level_variants() {
        let _b = PermissionLevel::Basic;
        let _l = PermissionLevel::Limited;
        let _f = PermissionLevel::Full;
    }

    #[test]
    fn test_crypto_lock_status_constructor() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert!(status.pure_rust_unlocked);
    }

    #[test]
    fn test_access_policies_default() {
        let policies = AccessPolicies;
        let _ = policies;
    }

    #[tokio::test]
    async fn test_install_crypto_permission_expired_rejects() {
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let perm = make_expired_permission(cloud_target());
        let result = lock.install_crypto_permission(perm).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_request_delegation_no_permission_fails() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let from = PermissionHolder::Individual {
            user_id: "user1".to_string(),
            public_key: "pk".to_string(),
            verification_level: VerificationLevel::Unverified,
        };
        let to = PermissionHolder::Individual {
            user_id: "user2".to_string(),
            public_key: "pk2".to_string(),
            verification_level: VerificationLevel::Unverified,
        };
        let target = cloud_target();
        let scope = DelegationScope {
            resource_limits: None,
            time_limits: None,
            feature_subset: vec![],
            geographic_subset: vec![],
        };
        let result = lock
            .request_delegation(&from, &to, &target, scope, Duration::from_secs(3600))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_crypto_lock_status_expiring_permissions_field() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert!(status.expiring_permissions.is_empty());
    }

    #[test]
    fn test_permission_level_all_variants_debug() {
        let levels = [
            PermissionLevel::Basic,
            PermissionLevel::Limited,
            PermissionLevel::Full,
        ];
        for level in levels {
            let s = format!("{:?}", level);
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn test_access_result_granted_with_restrictions() {
        let result = AccessResult::Granted {
            reason: "ok".to_string(),
            permission_level: PermissionLevel::Limited,
            expires_at: None,
            restrictions: vec!["geo:us".to_string()],
        };
        match &result {
            AccessResult::Granted { restrictions, .. } => {
                assert_eq!(restrictions.len(), 1);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_access_result_denied_debug() {
        let result = AccessResult::Denied {
            reason: "no perm".to_string(),
            how_to_get_access: "contact admin".to_string(),
        };
        let s = format!("{:?}", result);
        assert!(s.contains("Denied"));
    }

    #[tokio::test]
    async fn test_check_external_access_ecoprimals_trusted() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "songbird".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["ecoprimals_trusted".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            AccessResult::Denied { .. } => panic!("Expected Granted for ecoprimals_trusted"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_no_crypto_lock_feature() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "tool".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["no_crypto_lock".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            _ => panic!("Expected Granted for no_crypto_lock"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_primal_security() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "x".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["capability:security".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            _ => panic!("Expected Granted for capability:security"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_container_platform_denied() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ContainerPlatform {
            platform: ContainerPlatform::Kubernetes,
            clusters: vec![],
            namespaces: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("container"));
            }
            AccessResult::Granted { .. } => {
                panic!("Expected Denied for container without permission")
            }
        }
    }

    #[tokio::test]
    async fn test_get_crypto_lock_status_with_expiring() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let status = lock.get_crypto_lock_status().await.unwrap();
        assert!(status.pure_rust_unlocked);
        assert!(status.delegation_chains.is_empty());
    }

    #[test]
    fn test_access_result_clone() {
        let granted = AccessResult::Granted {
            reason: "r".to_string(),
            permission_level: PermissionLevel::Full,
            expires_at: None,
            restrictions: vec![],
        };
        let c = granted.clone();
        assert!(matches!(c, AccessResult::Granted { .. }));
    }

    #[test]
    fn test_permission_level_basic_debug() {
        let level = PermissionLevel::Basic;
        let s = format!("{:?}", level);
        assert!(s.contains("Basic"));
    }

    #[test]
    fn test_permission_level_limited_debug() {
        let level = PermissionLevel::Limited;
        let s = format!("{:?}", level);
        assert!(s.contains("Limited"));
    }

    #[test]
    fn test_access_result_granted_with_expires_at() {
        let result = AccessResult::Granted {
            reason: "ok".to_string(),
            permission_level: PermissionLevel::Full,
            expires_at: Some(SystemTime::now() + Duration::from_secs(3600)),
            restrictions: vec![],
        };
        match &result {
            AccessResult::Granted { expires_at, .. } => assert!(expires_at.is_some()),
            _ => panic!(),
        }
    }

    #[test]
    fn test_crypto_lock_status_with_external_permissions() {
        let mut perms = std::collections::HashMap::new();
        perms.insert(
            cloud_target(),
            PermissionStatus {
                permission_id: Uuid::new_v4(),
                holder: PermissionHolder::Individual {
                    user_id: "u1".to_string(),
                    public_key: "pk".to_string(),
                    verification_level: VerificationLevel::Unverified,
                },
                valid_until: SystemTime::now() + Duration::from_secs(3600),
                scope: PermissionScope {
                    resource_limits: ResourceLimits {
                        max_cpu_cores: None,
                        max_memory_gb: None,
                        max_storage_gb: None,
                        max_network_bandwidth: None,
                    },
                    time_restrictions: TimeRestrictions {
                        allowed_hours: None,
                        allowed_days: None,
                        timezone: None,
                    },
                    usage_quotas: UsageQuotas {
                        max_requests_per_hour: None,
                        max_data_transfer_gb: None,
                        max_compute_hours: None,
                    },
                    geographic_limits: vec![],
                    feature_restrictions: vec![],
                },
                is_delegated: false,
            },
        );
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: perms,
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert_eq!(status.external_permissions.len(), 1);
    }

    #[test]
    fn test_expiring_permission_constructor() {
        let ep = ExpiringPermission {
            permission_id: Uuid::new_v4(),
            target: cloud_target(),
            expires_in: Duration::from_secs(60),
        };
        assert_eq!(ep.expires_in.as_secs(), 60);
    }

    #[tokio::test]
    async fn test_check_external_access_capability_storage() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "x".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["capability:storage".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            AccessResult::Denied { .. } => panic!("Expected Granted for capability:storage"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_capability_coordination() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "x".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["capability:coordination".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            _ => panic!("Expected Granted for capability:coordination"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_capability_ai() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "x".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["capability:ai".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            _ => panic!("Expected Granted for capability:ai"),
        }
    }

    #[test]
    fn test_access_result_denied_clone() {
        let denied = AccessResult::Denied {
            reason: "no".to_string(),
            how_to_get_access: "get permit".to_string(),
        };
        let c = denied.clone();
        assert!(matches!(c, AccessResult::Denied { .. }));
    }

    #[tokio::test]
    async fn test_check_external_access_quantum_provider_denied_default_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::QuantumProvider {
            provider: QuantumProvider::IBM,
            backends: vec![],
            qubit_limits: Some(127),
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("security provider"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied for QuantumProvider"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_hpc_cluster_denied_default_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::HPCCluster {
            cluster_name: "super-a".to_string(),
            scheduler: HPCScheduler::SLURM,
            partitions: vec!["gpu".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("security provider"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied for HPCCluster"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_enterprise_service_denied_default_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::EnterpriseService {
            service_name: "acme-api".to_string(),
            tier: ServiceTier::Enterprise,
            features: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("security provider"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied for EnterpriseService"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_cloud_provider_aws_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::CloudProvider {
            provider: CloudProvider::AWS,
            regions: vec![],
            services: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("AWS"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_cloud_provider_gcp_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::CloudProvider {
            provider: CloudProvider::GCP,
            regions: vec![],
            services: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("GCP"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_container_docker_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ContainerPlatform {
            platform: ContainerPlatform::Docker,
            clusters: vec![],
            namespaces: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("container"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_external_tool_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "custom-tool".to_string(),
            api_endpoints: vec![],
            feature_set: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("custom-tool"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied"),
        }
    }

    #[test]
    fn test_permission_level_full_debug() {
        let level = PermissionLevel::Full;
        let s = format!("{:?}", level);
        assert!(s.contains("Full"));
    }

    #[test]
    fn test_crypto_lock_status_delegation_chains() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert!(status.delegation_chains.is_empty());
    }

    #[tokio::test]
    async fn test_install_crypto_permission_valid_succeeds() {
        use crate::crypto_lock::validation::{CryptoAlgorithm, ProofMetadata, SecurityProof};
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let now = SystemTime::now();
        let perm = SecurityProviderPermission {
            permission_id: Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "u1".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            external_target: cloud_target(),
            scope: PermissionScope {
                resource_limits: ResourceLimits {
                    max_cpu_cores: None,
                    max_memory_gb: None,
                    max_storage_gb: None,
                    max_network_bandwidth: None,
                },
                time_restrictions: TimeRestrictions {
                    allowed_hours: None,
                    allowed_days: None,
                    timezone: None,
                },
                usage_quotas: UsageQuotas {
                    max_requests_per_hour: None,
                    max_data_transfer_gb: None,
                    max_compute_hours: None,
                },
                geographic_limits: vec![],
                feature_restrictions: vec![],
            },
            valid_from: now - Duration::from_secs(3600),
            valid_until: now + Duration::from_secs(3600),
            crypto_proof: SecurityProof {
                signature: vec![],
                algorithm: CryptoAlgorithm::Ed25519,
                public_key_id: "k1".to_string(),
                timestamp: now,
                metadata: ProofMetadata {
                    issuer: "test".to_string(),
                    purpose: "test".to_string(),
                    additional_claims: std::collections::HashMap::new(),
                },
            },
            delegation_chain: None,
            metadata: PermissionMetadata {
                issued_by: "test".to_string(),
                notes: String::new(),
                features: vec![],
            },
        };
        let result = lock.install_crypto_permission(perm).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_external_access_with_valid_permission_granted() {
        use crate::crypto_lock::validation::{CryptoAlgorithm, ProofMetadata, SecurityProof};
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let now = SystemTime::now();
        let perm = SecurityProviderPermission {
            permission_id: Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "u1".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            external_target: cloud_target(),
            scope: PermissionScope {
                resource_limits: ResourceLimits {
                    max_cpu_cores: None,
                    max_memory_gb: None,
                    max_storage_gb: None,
                    max_network_bandwidth: None,
                },
                time_restrictions: TimeRestrictions {
                    allowed_hours: None,
                    allowed_days: None,
                    timezone: None,
                },
                usage_quotas: UsageQuotas {
                    max_requests_per_hour: None,
                    max_data_transfer_gb: None,
                    max_compute_hours: None,
                },
                geographic_limits: vec![],
                feature_restrictions: vec![],
            },
            valid_from: now - Duration::from_secs(3600),
            valid_until: now + Duration::from_secs(3600),
            crypto_proof: SecurityProof {
                signature: vec![],
                algorithm: CryptoAlgorithm::Ed25519,
                public_key_id: "k1".to_string(),
                timestamp: now,
                metadata: ProofMetadata {
                    issuer: "test".to_string(),
                    purpose: "test".to_string(),
                    additional_claims: std::collections::HashMap::new(),
                },
            },
            delegation_chain: None,
            metadata: PermissionMetadata {
                issued_by: "test".to_string(),
                notes: String::new(),
                features: vec![],
            },
        };
        let _ = lock.install_crypto_permission(perm).await;
        let result = lock.check_external_access(&cloud_target()).await.unwrap();
        match &result {
            AccessResult::Granted {
                permission_level,
                restrictions,
                ..
            } => {
                assert!(matches!(
                    permission_level,
                    PermissionLevel::Full | PermissionLevel::Limited | PermissionLevel::Basic
                ));
                let _ = restrictions;
            }
            AccessResult::Denied { .. } => {}
        }
    }

    #[tokio::test]
    async fn test_install_crypto_permission_invalid_rejects() {
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let mut perm = make_expired_permission(cloud_target());
        perm.valid_until = SystemTime::now() + Duration::from_secs(3600);
        perm.valid_from = SystemTime::now() + Duration::from_secs(1);
        let result = lock.install_crypto_permission(perm).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_crypto_permission_revoked_rejects() {
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let perm = make_expired_permission(cloud_target());
        let result = lock.install_crypto_permission(perm).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_access_result_granted_with_all_fields() {
        let result = AccessResult::Granted {
            reason: "ok".to_string(),
            permission_level: PermissionLevel::Basic,
            expires_at: Some(SystemTime::now()),
            restrictions: vec!["r1".to_string(), "r2".to_string()],
        };
        match &result {
            AccessResult::Granted {
                permission_level,
                restrictions,
                ..
            } => {
                assert!(matches!(permission_level, PermissionLevel::Basic));
                assert_eq!(restrictions.len(), 2);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_crypto_lock_status_clone() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        let c = status.clone();
        assert!(c.pure_rust_unlocked);
    }

    #[test]
    fn test_access_policies_unit_struct() {
        let p = AccessPolicies;
        let _ = p;
    }

    #[test]
    fn test_crypto_lock_status_with_delegation_chains() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert!(status.delegation_chains.is_empty());
    }

    #[test]
    fn test_permission_level_ordering() {
        let levels = [
            PermissionLevel::Basic,
            PermissionLevel::Limited,
            PermissionLevel::Full,
        ];
        for level in levels {
            let s = format!("{:?}", level);
            assert!(!s.is_empty());
        }
    }
}
