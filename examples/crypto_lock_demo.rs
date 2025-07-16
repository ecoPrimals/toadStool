//! # ToadStool Crypto Lock System Demo
//!
//! Demonstrates the BearDog cryptographic access control system:
//! - 🔓 Pure Rust ecosystem: Always unlocked
//! - 🔐 External integrations: Require BearDog crypto permissions
//! - 🤝 Permission delegation: Lending access to others
//! - 🎯 Granular control: Fine-grained permission management

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use toadstool::error::ToadStoolResult;
use toadstool_distributed::crypto_lock::*;
use toadstool_distributed::DistributedConfig;
use toadstool_distributed::DistributedCoordinator;
use toadstool_distributed::SongbirdConfig;
use toadstool_distributed::StandaloneConfig;

use uuid::Uuid;

#[tokio::main]
async fn main() -> ToadStoolResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🍄 ToadStool Crypto Lock System Demo");
    println!("=====================================");

    // Initialize crypto lock system
    let mut crypto_lock = ToadStoolCryptoLock::new().await?;

    // Demo 1: Pure Rust ecosystem access (always unlocked)
    println!("\n🔓 Demo 1: Pure Rust Ecosystem Access");
    println!("--------------------------------------");

    let toadstool_target = ExternalTarget::ExternalTool {
        tool_name: "toadstool".to_string(),
        api_endpoints: vec!["http://localhost:8080".to_string()],
        feature_set: vec!["execution".to_string(), "scheduling".to_string()],
    };

    let songbird_target = ExternalTarget::ExternalTool {
        tool_name: "songbird".to_string(),
        api_endpoints: vec!["http://localhost:8081".to_string()],
        feature_set: vec!["discovery".to_string(), "coordination".to_string()],
    };

    // Check access to pure Rust ecosystem
    let toadstool_access = crypto_lock.check_external_access(&toadstool_target).await?;
    let songbird_access = crypto_lock.check_external_access(&songbird_target).await?;

    println!("✅ ToadStool access: {toadstool_access:?}");
    println!("✅ Songbird access: {songbird_access:?}");

    // Demo 2: External cloud provider access (requires crypto permission)
    println!("\n🔐 Demo 2: External Cloud Provider Access");
    println!("------------------------------------------");

    let aws_target = ExternalTarget::CloudProvider {
        provider: CloudProvider::AWS,
        regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
        services: vec!["ec2".to_string(), "s3".to_string(), "lambda".to_string()],
    };

    let azure_target = ExternalTarget::CloudProvider {
        provider: CloudProvider::Azure,
        regions: vec!["eastus".to_string(), "westus2".to_string()],
        services: vec![
            "compute".to_string(),
            "storage".to_string(),
            "functions".to_string(),
        ],
    };

    // Check access without crypto permissions (should be denied)
    let aws_access = crypto_lock.check_external_access(&aws_target).await?;
    let azure_access = crypto_lock.check_external_access(&azure_target).await?;

    println!("❌ AWS access (no permission): {aws_access:?}");
    println!("❌ Azure access (no permission): {azure_access:?}");

    // Demo 3: Installing BearDog crypto permissions
    println!("\n📥 Demo 3: Installing BearDog Crypto Permissions");
    println!("--------------------------------------------------");

    // Create a crypto permission for AWS
    let aws_permission = create_demo_crypto_permission(
        &aws_target,
        PermissionHolder::Individual {
            user_id: "demo_user".to_string(),
            public_key: "demo_public_key".to_string(),
            verification_level: VerificationLevel::EmailVerified,
        },
        Duration::from_secs(30 * 24 * 60 * 60), // 30 days
    );

    // Install the permission
    crypto_lock
        .install_crypto_permission(aws_permission)
        .await?;

    // Check access again (should now be granted)
    let aws_access_after = crypto_lock.check_external_access(&aws_target).await?;
    println!("✅ AWS access (with permission): {aws_access_after:?}");

    // Demo 4: University gets free access
    println!("\n🎓 Demo 4: University Free Access");
    println!("----------------------------------");

    let university_permission = create_demo_crypto_permission(
        &azure_target,
        PermissionHolder::Organization {
            org_id: "stanford_university".to_string(),
            org_type: OrganizationType::University,
            authorized_users: vec!["prof_alice".to_string(), "student_bob".to_string()],
        },
        Duration::from_secs(365 * 24 * 60 * 60), // 1 year
    );

    crypto_lock
        .install_crypto_permission(university_permission)
        .await?;

    let azure_access_university = crypto_lock.check_external_access(&azure_target).await?;
    println!("✅ Azure access (university): {azure_access_university:?}");

    // Demo 5: Permission delegation (lending access)
    println!("\n🤝 Demo 5: Permission Delegation");
    println!("----------------------------------");

    let from_holder = PermissionHolder::Individual {
        user_id: "demo_user".to_string(),
        public_key: "demo_public_key".to_string(),
        verification_level: VerificationLevel::EmailVerified,
    };

    let to_holder = PermissionHolder::Individual {
        user_id: "collaborator".to_string(),
        public_key: "collaborator_public_key".to_string(),
        verification_level: VerificationLevel::EmailVerified,
    };

    let delegation_scope = DelegationScope {
        resource_limits: Some(ResourceLimits {
            max_cpu_cores: Some(10.0),
            max_memory_gb: Some(50.0),
            max_storage_gb: Some(100.0),
            max_network_bandwidth: Some(1000.0),
        }),
        time_limits: Some(Duration::from_secs(7 * 24 * 60 * 60)), // 7 days
        feature_subset: vec!["basic_compute".to_string()],
        geographic_subset: vec!["us-east-1".to_string()],
    };

    let delegation_request = crypto_lock
        .request_delegation(
            &from_holder,
            &to_holder,
            &aws_target,
            delegation_scope,
            Duration::from_secs(7 * 24 * 60 * 60), // 7 days
        )
        .await?;

    println!(
        "📋 Delegation request created: {:?}",
        delegation_request.request_id
    );

    // Demo 6: Crypto lock status report
    println!("\n📊 Demo 6: Crypto Lock Status Report");
    println!("--------------------------------------");

    let status = crypto_lock.get_crypto_lock_status().await?;
    println!("📈 Crypto lock status:");
    println!("  - Pure Rust unlocked: {}", status.pure_rust_unlocked);
    println!(
        "  - External permissions: {}",
        status.external_permissions.len()
    );
    println!("  - Delegation chains: {}", status.delegation_chains.len());
    println!(
        "  - Expiring permissions: {}",
        status.expiring_permissions.len()
    );

    // Demo 7: Massive job distribution with crypto permissions
    println!("\n🌊 Demo 7: Massive Job Distribution");
    println!("------------------------------------");

    let config = DistributedConfig {
        instance_id: "crypto_demo_instance".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 100,
            default_timeout_secs: 300,
            enable_job_queue: true,
            max_queue_size: 1000,
        },
        songbird_integration: Some(SongbirdConfig {
            endpoint: "http://localhost:8081".to_string(),
            auth_token: None,
            health_reporting_interval_secs: 30,
        }),
    };

    let _distributed_coordinator = DistributedCoordinator::new(config).await?;

    println!("🚀 Distributed coordinator initialized with crypto lock protection");
    println!("   - Pure Rust ecosystem: Always available");
    println!("   - External cloud providers: Protected by BearDog crypto permissions");
    println!("   - Permission delegation: Enabled for collaboration");
    println!("   - Granular access control: Per-service, per-resource permissions");

    // Demo 8: Enterprise vs Free model
    println!("\n💼 Demo 8: Enterprise vs Free Model");
    println!("------------------------------------");

    println!("🔓 FREE ACCESS:");
    println!("  - Pure Rust ecosystem (ToadStool, BearDog, NestGate, Songbird)");
    println!("  - Universities, research institutions, individuals");
    println!("  - Full power computing with no restrictions");

    println!("\n🔐 CRYPTO-LOCKED ACCESS:");
    println!("  - External cloud providers (AWS, Azure, GCP, etc.)");
    println!("  - Commercial tools and services");
    println!("  - Enterprise integrations");
    println!("  - Quantum computing platforms");

    println!("\n🎯 KEY FEATURES:");
    println!("  - No phone home - pure cryptographic proof");
    println!("  - Permission delegation - lend access to others");
    println!("  - Granular control - fine-grained permissions");
    println!("  - BearDog managed - all control on BearDog side");
    println!("  - Anti-exploitation - protects free ecosystem");

    println!("\n🎉 Demo completed successfully!");
    println!("ToadStool crypto lock system is ready for production use!");

    Ok(())
}

/// Create a demo crypto permission for testing
fn create_demo_crypto_permission(
    target: &ExternalTarget,
    holder: PermissionHolder,
    duration: Duration,
) -> BearDogCryptoPermission {
    BearDogCryptoPermission {
        permission_id: Uuid::new_v4(),
        holder,
        external_target: target.clone(),
        scope: PermissionScope {
            resource_limits: ResourceLimits {
                max_cpu_cores: Some(100.0),
                max_memory_gb: Some(500.0),
                max_storage_gb: Some(1000.0),
                max_network_bandwidth: Some(10000.0),
            },
            time_restrictions: TimeRestrictions {
                allowed_hours: None, // 24/7 access
                allowed_days: None,  // All days
                timezone: Some("UTC".to_string()),
            },
            usage_quotas: UsageQuotas {
                max_requests_per_hour: Some(10000),
                max_data_transfer_gb: Some(1000.0),
                max_compute_hours: Some(1000.0),
            },
            geographic_limits: vec![],    // No geographic restrictions
            feature_restrictions: vec![], // No feature restrictions
        },
        valid_from: SystemTime::now(),
        valid_until: SystemTime::now() + duration,
        crypto_proof: BearDogCryptoProof {
            signature: b"demo_signature".to_vec(),
            algorithm: CryptoAlgorithm::Ed25519,
            public_key_id: "demo_key_id".to_string(),
            timestamp: SystemTime::now(),
            metadata: ProofMetadata {
                issuer: "BearDog Demo".to_string(),
                purpose: "Demo permission".to_string(),
                additional_claims: HashMap::new(),
            },
        },
        delegation_chain: None,
        metadata: PermissionMetadata {
            issued_by: "BearDog Demo System".to_string(),
            notes: "Demo permission for testing crypto lock system".to_string(),
            features: vec!["full_access".to_string()],
        },
    }
}
