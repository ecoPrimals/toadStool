// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for under-covered modules in toadstool-distributed
//!
//! Targets: crypto_integration/client, cloud/federation, coordination,
//! security_provider (software_hsm, provider, factory), network/load_balancer,
//! crypto_lock/validation, hosting/resources, cloud/cost/pricing

//! Covers modules that are deprecated and feature-gated: `cloud`,
//! `crypto_lock`, and `security_provider` are compiled only with their
//! `legacy-*` features. Without a matching gate this file did not compile,
//! so none of its tests ran — including those covering live code.
#![cfg(all(
    feature = "runtime",
    feature = "legacy-security",
    feature = "legacy-cloud",
    feature = "legacy-coordination"
))]

#![allow(clippy::pedantic)]
#![allow(deprecated)]

use std::collections::HashMap;
use std::time::SystemTime;

use toadstool_distributed::cloud::types::{DataReplica, FederationNode};
use toadstool_distributed::cloud::{
    CloudCapabilities, CloudCostModel, CloudCostOptimizer, CloudFederationManager, ComputeType,
    CostConfig, FederationConfig, NetworkingFeature, PricingTier, ReplicaStatus, SecurityFeature,
    StorageType,
};
use toadstool_distributed::crypto_lock::validation::{
    CryptoAlgorithm, CryptoValidator, DelegationValidator, PermissionRevocationList,
    PermissionValidationResult, ProofMetadata, SecurityProof, VerificationLevel,
};
use toadstool_distributed::hosting::{HostingResourceConfig, HostingResourceManager};
use toadstool_distributed::network::load_balancer::{
    CircuitBreaker, CircuitBreakerState, FaultToleranceManager, NetworkLoadBalancer, NodeHealth,
};
use toadstool_distributed::security_provider::provider::SecurityCapability;
use toadstool_distributed::types::{
    CpuRequirements, MemoryRequirements, NetworkRequirements, ResourceRequirements,
    StorageRequirements,
};

// ============================================================================
// Cloud Federation
// ============================================================================

fn make_federation_config(id: &str) -> FederationConfig {
    FederationConfig {
        federation_id: id.to_string(),
        discovery_endpoints: vec![],
        trust_anchors: vec![],
    }
}

fn make_federation_node(id: &str, provider: &str) -> FederationNode {
    FederationNode {
        id: id.to_string(),
        provider: provider.to_string(),
        region: "us-east-1".to_string(),
        capabilities: vec!["compute".to_string()],
    }
}

#[tokio::test]
async fn test_federation_manager_creation() {
    let mgr = CloudFederationManager::new(make_federation_config("fed-s155"))
        .await
        .expect("create federation");
    assert_eq!(mgr.federation_id(), "fed-s155");
    assert_eq!(mgr.member_count(), 0);
    assert_eq!(mgr.node_ids().count(), 0);
}

#[tokio::test]
async fn test_federation_add_node_success() {
    let mut mgr = CloudFederationManager::new(make_federation_config("fed-add"))
        .await
        .unwrap();
    mgr.add_node(make_federation_node("node-1", "aws"), vec![])
        .expect("add node");
    assert_eq!(mgr.member_count(), 1);
    assert!(mgr.node_ids().any(|id| id == "node-1"));
}

#[tokio::test]
async fn test_federation_add_node_empty_id_fails() {
    let mut mgr = CloudFederationManager::new(make_federation_config("fed-empty"))
        .await
        .unwrap();
    let node = FederationNode {
        id: String::new(),
        provider: "aws".to_string(),
        region: "us-east-1".to_string(),
        capabilities: vec![],
    };
    let result = mgr.add_node(node, vec![]);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_federation_add_duplicate_node_fails() {
    let mut mgr = CloudFederationManager::new(make_federation_config("fed-dup"))
        .await
        .unwrap();
    mgr.add_node(make_federation_node("node-a", "aws"), vec![])
        .unwrap();
    let result = mgr.add_node(make_federation_node("node-a", "gcp"), vec![]);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_federation_remove_node() {
    let mut mgr = CloudFederationManager::new(make_federation_config("fed-rm"))
        .await
        .unwrap();
    mgr.add_node(make_federation_node("node-x", "aws"), vec![])
        .unwrap();
    mgr.remove_node("node-x").expect("remove node");
    assert_eq!(mgr.member_count(), 0);
}

#[tokio::test]
async fn test_federation_remove_nonexistent_fails() {
    let mut mgr = CloudFederationManager::new(make_federation_config("fed-rm-none"))
        .await
        .unwrap();
    let result = mgr.remove_node("nonexistent");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_federation_discover_nodes_empty_endpoints() {
    let mgr = CloudFederationManager::new(make_federation_config("fed-disc"))
        .await
        .unwrap();
    let discovered = mgr.discover_nodes().await.unwrap();
    assert!(discovered.is_empty());
}

#[tokio::test]
async fn test_federation_get_member_capabilities() {
    let mut mgr = CloudFederationManager::new(make_federation_config("fed-caps"))
        .await
        .unwrap();
    mgr.add_node(make_federation_node("n1", "aws"), vec![])
        .unwrap();
    let caps = mgr.get_member_capabilities("n1").unwrap();
    assert_eq!(caps, vec!["compute"]);
}

#[tokio::test]
async fn test_federation_get_member_capabilities_nonexistent_fails() {
    let mgr = CloudFederationManager::new(make_federation_config("fed-caps-none"))
        .await
        .unwrap();
    let result = mgr.get_member_capabilities("nonexistent");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_federation_register_replica() {
    let mut mgr = CloudFederationManager::new(make_federation_config("fed-repl"))
        .await
        .unwrap();
    let replica = DataReplica {
        id: "r1".to_string(),
        location: "s3://bucket/key".to_string(),
        status: ReplicaStatus::Synced,
    };
    mgr.register_replica(replica);
    assert_eq!(mgr.replica_count(), 1);
}

// ============================================================================
// Cloud Cost Pricing
// ============================================================================

#[test]
fn test_pricing_tier_cpu_costs() {
    assert_eq!(PricingTier::StandardCompute.cpu_cost_per_core_hour(), 0.08);
    assert_eq!(
        PricingTier::HighMemoryCompute.cpu_cost_per_core_hour(),
        0.12
    );
    assert_eq!(PricingTier::GpuAccelerated.cpu_cost_per_core_hour(), 0.15);
    assert_eq!(PricingTier::EdgeLocal.cpu_cost_per_core_hour(), 0.01);
}

#[test]
fn test_pricing_tier_memory_costs() {
    assert_eq!(
        PricingTier::StandardCompute.memory_cost_per_gb_hour(),
        0.012
    );
    assert_eq!(PricingTier::Serverless.memory_cost_per_gb_hour(), 0.000016);
}

#[test]
fn test_pricing_tier_storage_costs() {
    assert_eq!(
        PricingTier::StandardCompute.storage_cost_per_gb_month(),
        0.08
    );
    assert_eq!(PricingTier::EdgeLocal.storage_cost_per_gb_month(), 0.0);
}

#[test]
fn test_pricing_tier_network_costs() {
    assert_eq!(PricingTier::StandardCompute.network_cost_per_gb(), 0.05);
    assert_eq!(PricingTier::EdgeLocal.network_cost_per_gb(), 0.0);
}

#[test]
fn test_pricing_tier_gpu_costs() {
    assert_eq!(PricingTier::GpuAccelerated.gpu_cost_per_gpu_hour(), 2.50);
    assert_eq!(
        PricingTier::BareMetalDedicated.gpu_cost_per_gpu_hour(),
        3.00
    );
    assert_eq!(PricingTier::StandardCompute.gpu_cost_per_gpu_hour(), 0.0);
}

#[test]
fn test_cloud_cost_model_constructors() {
    let standard = CloudCostModel::standard_compute();
    assert!(standard.cpu_rate > 0.0);
    assert!(standard.memory_rate > 0.0);

    let gpu = CloudCostModel::gpu_accelerated();
    assert!(gpu.cpu_rate > 0.0);

    let edge = CloudCostModel::edge_local();
    assert!(edge.cpu_rate > 0.0);
}

#[test]
fn test_cloud_cost_model_legacy_constructors() {
    let aws = CloudCostModel::new_aws();
    assert!(aws.cpu_rate > 0.0);
    let azure = CloudCostModel::new_azure();
    assert!(azure.cpu_rate > 0.0);
    let gcp = CloudCostModel::new_gcp();
    assert!(gcp.cpu_rate > 0.0);
}

// ============================================================================
// Cloud Cost Optimizer
// ============================================================================

fn minimal_resource_requirements() -> ResourceRequirements {
    ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: None,
        },
        memory: MemoryRequirements {
            min_bytes: 1024 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements {
            min_bytes: 1024 * 1024 * 1024,
            max_bytes: None,
        },
        network: NetworkRequirements {
            bandwidth_mbps: None,
            latency_ms: None,
        },
        gpu: None,
    }
}

fn gpu_capabilities() -> CloudCapabilities {
    CloudCapabilities {
        compute_types: vec![ComputeType::VM, ComputeType::GPU],
        storage_types: vec![StorageType::BlockStorage],
        networking_features: vec![NetworkingFeature::VPC],
        security_features: vec![SecurityFeature::Encryption],
        compliance_certifications: vec![],
        regions: vec![],
        max_cpu_cores: Some(64),
        max_memory_gb: Some(256),
        gpu_support: true,
        kubernetes_support: false,
        serverless_support: false,
    }
}

#[tokio::test]
async fn test_cloud_cost_optimizer_estimate_cost() {
    let cfg = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: false,
        spot_instance_preference: 0.0,
    };
    let mut optimizer: CloudCostOptimizer = CloudCostOptimizer::new(cfg).await.unwrap();
    optimizer
        .add_provider_cost_model("provider1", &gpu_capabilities())
        .await
        .unwrap();

    let req = minimal_resource_requirements();
    let est = optimizer
        .estimate_cost("provider1", &req, 1.0, 0.0)
        .expect("estimate cost");
    assert!(est.total_cost > 0.0);
    assert_eq!(est.duration_hours, 1.0);
    assert!(!est.line_items.is_empty());
}

#[tokio::test]
async fn test_cloud_cost_optimizer_invalid_duration() {
    let cfg = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: false,
        spot_instance_preference: 0.0,
    };
    let mut optimizer: CloudCostOptimizer = CloudCostOptimizer::new(cfg).await.unwrap();
    optimizer
        .add_provider_cost_model("p", &gpu_capabilities())
        .await
        .unwrap();
    let req = minimal_resource_requirements();
    let result = optimizer.estimate_cost("p", &req, 0.0, 0.0);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cloud_cost_optimizer_model_not_found() {
    let cfg = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: false,
        spot_instance_preference: 0.0,
    };
    let optimizer: CloudCostOptimizer = CloudCostOptimizer::new(cfg).await.unwrap();
    let req = minimal_resource_requirements();
    let result = optimizer.estimate_cost("nonexistent", &req, 1.0, 0.0);
    assert!(result.is_err());
}

// ============================================================================
// Network Load Balancer
// ============================================================================

#[tokio::test]
async fn test_load_balancer_register_and_select_node() {
    let lb = NetworkLoadBalancer::new();
    lb.register_node(
        "node-1".to_string(),
        NodeHealth {
            healthy: true,
            cpu_usage: 0.3,
            memory_usage: 0.4,
            response_time_ms: 50,
        },
    )
    .await;
    lb.register_node(
        "node-2".to_string(),
        NodeHealth {
            healthy: true,
            cpu_usage: 0.8,
            memory_usage: 0.9,
            response_time_ms: 200,
        },
    )
    .await;

    let selected = lb.select_node().await;
    assert_eq!(selected, Some("node-1".to_string()));
}

#[tokio::test]
async fn test_load_balancer_select_excludes_unhealthy() {
    let lb = NetworkLoadBalancer::new();
    lb.register_node(
        "unhealthy".to_string(),
        NodeHealth {
            healthy: false,
            cpu_usage: 0.95,
            memory_usage: 0.98,
            response_time_ms: 5000,
        },
    )
    .await;

    let selected = lb.select_node().await;
    assert!(selected.is_none());
}

#[tokio::test]
async fn test_load_balancer_deregister_node() {
    let lb = NetworkLoadBalancer::new();
    lb.register_node(
        "node-a".to_string(),
        NodeHealth {
            healthy: true,
            cpu_usage: 0.5,
            memory_usage: 0.5,
            response_time_ms: 100,
        },
    )
    .await;
    lb.deregister_node("node-a").await;
    let selected = lb.select_node().await;
    assert!(selected.is_none());
}

#[tokio::test]
async fn test_load_balancer_node_health_snapshot() {
    let lb = NetworkLoadBalancer::new();
    lb.register_node(
        "snap-node".to_string(),
        NodeHealth {
            healthy: true,
            cpu_usage: 0.25,
            memory_usage: 0.35,
            response_time_ms: 25,
        },
    )
    .await;
    let snapshot = lb.node_health_snapshot().await;
    assert_eq!(snapshot.len(), 1);
    assert!(snapshot.contains_key("snap-node"));
}

#[test]
fn test_fault_tolerance_manager_creation() {
    let manager = FaultToleranceManager::new();
    let _ = manager;
    let default = FaultToleranceManager::default();
    let _ = default;
}

#[test]
fn test_circuit_breaker_state_variants() {
    let closed = CircuitBreaker {
        state: CircuitBreakerState::Closed,
        failure_count: 0,
        last_failure_time: None,
    };
    assert!(matches!(closed.state, CircuitBreakerState::Closed));

    let open = CircuitBreaker {
        state: CircuitBreakerState::Open,
        failure_count: 5,
        last_failure_time: Some(std::time::Instant::now()),
    };
    assert!(matches!(open.state, CircuitBreakerState::Open));
}

// ============================================================================
// Crypto Lock Validation
// ============================================================================

#[test]
fn test_crypto_validator_construction() {
    let v = CryptoValidator::new();
    assert!(v.validate_signature(b"sig", std::time::SystemTime::now()));
    assert!(!v.validate_signature(b"", std::time::SystemTime::now()));
    let d = CryptoValidator::default();
    let _ = d;
}

#[test]
fn test_delegation_validator_construction() {
    let v = DelegationValidator::new();
    let _ = v;
    let d = DelegationValidator::default();
    let _ = d;
}

#[test]
fn test_permission_revocation_list_construction() {
    let mut v = PermissionRevocationList::new();
    let id = uuid::Uuid::new_v4();
    assert!(!v.is_revoked(&id));
    v.revoke(id);
    assert!(v.is_revoked(&id));
    let d = PermissionRevocationList::default();
    assert!(!d.is_revoked(&id));
}

#[test]
fn test_security_proof_serialization() {
    let proof = SecurityProof {
        signature: vec![1, 2, 3],
        algorithm: CryptoAlgorithm::Ed25519,
        public_key_id: "key-1".to_string(),
        timestamp: SystemTime::now(),
        metadata: ProofMetadata {
            issuer: "test".to_string(),
            purpose: "validation".to_string(),
            additional_claims: HashMap::new(),
        },
    };
    let json = serde_json::to_value(&proof).unwrap();
    assert!(json.get("algorithm").is_some());
    assert!(json.get("public_key_id").is_some());
}

#[test]
fn test_proof_metadata_serialization() {
    let meta = ProofMetadata {
        issuer: "issuer".to_string(),
        purpose: "purpose".to_string(),
        additional_claims: HashMap::new(),
    };
    let json = serde_json::to_value(&meta).unwrap();
    assert_eq!(json.get("issuer").and_then(|v| v.as_str()), Some("issuer"));
}

#[test]
fn test_verification_level_serialization() {
    let levels = [
        VerificationLevel::Unverified,
        VerificationLevel::EmailVerified,
        VerificationLevel::IdentityVerified,
        VerificationLevel::InstitutionVerified,
    ];
    for level in &levels {
        let json = serde_json::to_value(level).unwrap();
        let parsed: VerificationLevel = serde_json::from_value(json).unwrap();
        assert_eq!(format!("{level:?}"), format!("{parsed:?}"));
    }
}

#[test]
fn test_permission_validation_result_variants() {
    let _ = PermissionValidationResult::Valid;
    let _ = PermissionValidationResult::Invalid;
    let _ = PermissionValidationResult::Expired;
    let _ = PermissionValidationResult::Revoked;
}

// ============================================================================
// Security Provider Types
// ============================================================================

#[test]
fn test_security_capability_variants() {
    let caps = [
        SecurityCapability::SymmetricEncryption,
        SecurityCapability::AsymmetricEncryption,
        SecurityCapability::DigitalSignatures,
        SecurityCapability::KeyManagement,
        SecurityCapability::PermissionIssuance,
        SecurityCapability::CertificateAuthority,
    ];
    for cap in caps {
        let json = serde_json::to_value(&cap).unwrap();
        let parsed: SecurityCapability = serde_json::from_value(json).unwrap();
        assert_eq!(cap, parsed);
    }
}

// ============================================================================
// Hosting Resources
// ============================================================================

#[test]
fn test_hosting_resource_manager_available() {
    let mut manager = HostingResourceManager::new(HostingResourceConfig::default());
    manager.total_resources.insert("cpu_cores".to_string(), 8);
    manager
        .allocated_resources
        .insert("cpu_cores".to_string(), 2);
    assert_eq!(manager.available("cpu_cores"), 6);
}

#[test]
fn test_hosting_resource_manager_available_unknown_type() {
    let manager = HostingResourceManager::new(HostingResourceConfig::default());
    assert_eq!(manager.available("unknown"), 0);
}

#[test]
fn test_hosting_resource_manager_utilization() {
    let mut manager = HostingResourceManager::new(HostingResourceConfig::default());
    manager.total_resources.insert("cpu_cores".to_string(), 8);
    manager
        .allocated_resources
        .insert("cpu_cores".to_string(), 4);
    assert!((manager.utilization("cpu_cores") - 0.5).abs() < 0.01);
}

#[test]
fn test_hosting_resource_manager_utilization_zero_total() {
    let manager = HostingResourceManager::new(HostingResourceConfig::default());
    assert_eq!(manager.utilization("cpu_cores"), 0.0);
}

#[test]
fn test_hosting_resource_manager_can_allocate_disabled() {
    let config = HostingResourceConfig {
        enabled: false,
        ..Default::default()
    };
    let manager = HostingResourceManager::new(config);
    let mut req = HashMap::new();
    req.insert("cpu_cores".to_string(), 9999);
    assert!(manager.can_allocate(&req));
}

#[test]
fn test_hosting_resource_manager_serialization() {
    let config = HostingResourceConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("enabled"));
    let parsed: HostingResourceConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.enabled, parsed.enabled);
}

// ============================================================================
// Coordination Integration Types (deprecated but still exported)
// ============================================================================

#[cfg(feature = "legacy-coordination")]
#[test]
fn test_coordination_job_complexity_variants() {
    use toadstool_distributed::coordination::JobComplexity;
    let _ = JobComplexity::Simple;
    let _ = JobComplexity::Moderate;
    let _ = JobComplexity::Complex;
    let _ = JobComplexity::UltraMassive;
}

#[cfg(feature = "legacy-coordination")]
#[test]
fn test_coordination_distribution_strategy_variants() {
    use toadstool_distributed::coordination::JobDistributionStrategy;
    let _ = JobDistributionStrategy::LocalOnly;
    let _ = JobDistributionStrategy::LoadBalanced;
    let _ = JobDistributionStrategy::SplitAndDistribute;
}

// ============================================================================
// Software HSM (requires dev-crypto feature)
// ============================================================================

#[cfg(feature = "dev-crypto")]
#[tokio::test]
async fn test_software_hsm_provider_capabilities() {
    use toadstool_distributed::security_provider::{SecurityProvider, SoftwareHsmProvider};
    let provider = SoftwareHsmProvider::new();
    let caps = provider.capabilities().await.unwrap();
    assert!(caps.contains(&SecurityCapability::SymmetricEncryption));
    assert!(caps.contains(&SecurityCapability::DigitalSignatures));
}

#[cfg(feature = "dev-crypto")]
#[tokio::test]
async fn test_software_hsm_encrypt_decrypt_roundtrip() {
    use toadstool_distributed::security_provider::{SecurityProvider, SoftwareHsmProvider};
    let provider = SoftwareHsmProvider::new();
    let data = b"test secret data";
    let encrypted = provider.encrypt(data, None).await.unwrap();
    assert!(!encrypted.ciphertext.is_empty());
    let decrypted = provider
        .decrypt(&encrypted.ciphertext, &encrypted.metadata)
        .await
        .unwrap();
    assert_eq!(decrypted.plaintext.as_slice(), data);
}

#[cfg(feature = "dev-crypto")]
#[tokio::test]
async fn test_software_hsm_sign_verify_roundtrip() {
    use toadstool_distributed::security_provider::{
        SecurityProvider, SoftwareHsmProvider, VerificationResult,
    };
    let provider = SoftwareHsmProvider::new();
    let data = b"data to sign";
    let sig_result = provider.sign(data, None).await.unwrap();
    let verify = provider
        .verify(data, &sig_result.signature, &sig_result.key_id)
        .await
        .unwrap();
    assert!(matches!(verify, VerificationResult::Valid));
}
