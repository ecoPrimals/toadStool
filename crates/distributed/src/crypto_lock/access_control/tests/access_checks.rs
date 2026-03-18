// SPDX-License-Identifier: AGPL-3.0-or-later
//! Access check tests

use crate::crypto_lock::access_control::{AccessResult, ToadStoolCryptoLock};
use crate::security_provider::types::{
    CloudProvider, ContainerPlatform, ExternalTarget, HPCScheduler, QuantumProvider, ServiceTier,
};

use super::helpers::{
    cloud_target, external_tool_target_no_trust, primal_toadstool_target, pure_rust_target,
    trusted_target,
};

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
            assert!(matches!(
                permission_level,
                crate::crypto_lock::access_control::PermissionLevel::Full
            ));
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
            how_to_get_access, ..
        } => assert!(how_to_get_access.contains("external tool")),
    }
}

#[tokio::test]
async fn test_get_crypto_lock_status() {
    let lock = ToadStoolCryptoLock::new().await.unwrap();
    let status = lock.get_crypto_lock_status().await.unwrap();
    assert!(status.pure_rust_unlocked);
    assert!(status.external_permissions.is_empty());
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
        } => assert!(how_to_get_access.contains("container")),
        AccessResult::Granted { .. } => panic!("Expected Denied for container without permission"),
    }
}

#[tokio::test]
async fn test_get_crypto_lock_status_with_expiring() {
    let lock = ToadStoolCryptoLock::new().await.unwrap();
    let status = lock.get_crypto_lock_status().await.unwrap();
    assert!(status.pure_rust_unlocked);
    assert!(status.delegation_chains.is_empty());
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
        } => assert!(how_to_get_access.contains("security provider")),
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
        } => assert!(how_to_get_access.contains("security provider")),
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
        } => assert!(how_to_get_access.contains("security provider")),
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
        } => assert!(how_to_get_access.contains("AWS")),
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
        } => assert!(how_to_get_access.contains("GCP")),
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
        } => assert!(how_to_get_access.contains("container")),
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
        } => assert!(how_to_get_access.contains("custom-tool")),
        AccessResult::Granted { .. } => panic!("Expected Denied"),
    }
}
