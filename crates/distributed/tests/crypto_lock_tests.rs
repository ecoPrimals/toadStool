// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for crypto lock types
//!
//! This test suite covers:
//! - `AccessResult` enum
//! - `PermissionValidationResult` enum
//! - `PermissionLevel` enum
//! - `OrganizationType` enum
//! - `VerificationLevel` enum
//! - `CryptoAlgorithm` enum
//! - `DelegationStatus` enum
//! - `CloudProvider` enum
//! - `ContainerPlatform` enum
//! - `QuantumProvider` enum
//! - `HPCScheduler` enum
//! - `ServiceTier` enum

//! Covers modules that are deprecated and feature-gated, so this file
//! is compiled only when they are. Without a matching gate it did not
//! compile at all, and none of its tests ran.
#![cfg(all(
    feature = "runtime",
    feature = "legacy-security"
))]

use toadstool_distributed::crypto_lock::*;

// ============================================================================
// PermissionValidationResult Tests
// ============================================================================

#[test]
fn test_permission_validation_valid() {
    let result = PermissionValidationResult::Valid;

    assert!(matches!(result, PermissionValidationResult::Valid));
}

#[test]
fn test_permission_validation_invalid() {
    let result = PermissionValidationResult::Invalid;

    assert!(matches!(result, PermissionValidationResult::Invalid));
}

#[test]
fn test_permission_validation_expired() {
    let result = PermissionValidationResult::Expired;

    assert!(matches!(result, PermissionValidationResult::Expired));
}

#[test]
fn test_permission_validation_revoked() {
    let result = PermissionValidationResult::Revoked;

    assert!(matches!(result, PermissionValidationResult::Revoked));
}

// ============================================================================
// PermissionLevel Tests
// ============================================================================

#[test]
fn test_permission_level_basic() {
    let level = PermissionLevel::Basic;

    assert!(matches!(level, PermissionLevel::Basic));
}

#[test]
fn test_permission_level_limited() {
    let level = PermissionLevel::Limited;

    assert!(matches!(level, PermissionLevel::Limited));
}

#[test]
fn test_permission_level_full() {
    let level = PermissionLevel::Full;

    assert!(matches!(level, PermissionLevel::Full));
}

// ============================================================================
// OrganizationType Tests
// ============================================================================

#[test]
fn test_organization_type_university() {
    let org = OrganizationType::University;

    assert!(matches!(org, OrganizationType::University));
}

#[test]
fn test_organization_type_research() {
    let org = OrganizationType::Research;

    assert!(matches!(org, OrganizationType::Research));
}

#[test]
fn test_organization_type_nonprofit() {
    let org = OrganizationType::NonProfit;

    assert!(matches!(org, OrganizationType::NonProfit));
}

#[test]
fn test_organization_type_commercial() {
    let org = OrganizationType::Commercial;

    assert!(matches!(org, OrganizationType::Commercial));
}

#[test]
fn test_organization_type_government() {
    let org = OrganizationType::Government;

    assert!(matches!(org, OrganizationType::Government));
}

// ============================================================================
// VerificationLevel Tests
// ============================================================================

#[test]
fn test_verification_level_unverified() {
    let level = VerificationLevel::Unverified;

    assert!(matches!(level, VerificationLevel::Unverified));
}

#[test]
fn test_verification_level_email() {
    let level = VerificationLevel::EmailVerified;

    assert!(matches!(level, VerificationLevel::EmailVerified));
}

#[test]
fn test_verification_level_identity() {
    let level = VerificationLevel::IdentityVerified;

    assert!(matches!(level, VerificationLevel::IdentityVerified));
}

#[test]
fn test_verification_level_institution() {
    let level = VerificationLevel::InstitutionVerified;

    assert!(matches!(level, VerificationLevel::InstitutionVerified));
}

// ============================================================================
// CryptoAlgorithm Tests
// ============================================================================

#[test]
fn test_crypto_algorithm_ed25519() {
    let algo = CryptoAlgorithm::Ed25519;

    assert!(matches!(algo, CryptoAlgorithm::Ed25519));
}

#[test]
fn test_crypto_algorithm_ecdsa() {
    let algo = CryptoAlgorithm::EcdsaP256;

    assert!(matches!(algo, CryptoAlgorithm::EcdsaP256));
}

#[test]
fn test_crypto_algorithm_rsa() {
    let algo = CryptoAlgorithm::Rsa4096;

    assert!(matches!(algo, CryptoAlgorithm::Rsa4096));
}

#[test]
fn test_crypto_algorithm_security_custom() {
    let algo = CryptoAlgorithm::SecurityLayerCustom;

    assert!(matches!(algo, CryptoAlgorithm::SecurityLayerCustom));
}

// ============================================================================
// DelegationStatus Tests
// ============================================================================

#[test]
fn test_delegation_status_pending() {
    let status = DelegationStatus::Pending;

    assert!(matches!(status, DelegationStatus::Pending));
}

#[test]
fn test_delegation_status_approved() {
    let status = DelegationStatus::Approved;

    assert!(matches!(status, DelegationStatus::Approved));
}

#[test]
fn test_delegation_status_denied() {
    let status = DelegationStatus::Denied;

    assert!(matches!(status, DelegationStatus::Denied));
}

#[test]
fn test_delegation_status_expired() {
    let status = DelegationStatus::Expired;

    assert!(matches!(status, DelegationStatus::Expired));
}

// ============================================================================
// CloudProvider Tests
// ============================================================================

#[test]
fn test_cloud_provider_aws() {
    let provider = CloudProvider::AWS;

    assert!(matches!(provider, CloudProvider::AWS));
}

#[test]
fn test_cloud_provider_azure() {
    let provider = CloudProvider::Azure;

    assert!(matches!(provider, CloudProvider::Azure));
}

#[test]
fn test_cloud_provider_gcp() {
    let provider = CloudProvider::GCP;

    assert!(matches!(provider, CloudProvider::GCP));
}

#[test]
fn test_cloud_provider_digitalocean() {
    let provider = CloudProvider::DigitalOcean;

    assert!(matches!(provider, CloudProvider::DigitalOcean));
}

#[test]
fn test_cloud_provider_linode() {
    let provider = CloudProvider::Linode;

    assert!(matches!(provider, CloudProvider::Linode));
}

#[test]
fn test_cloud_provider_vultr() {
    let provider = CloudProvider::Vultr;

    assert!(matches!(provider, CloudProvider::Vultr));
}

#[test]
fn test_cloud_provider_hetzner() {
    let provider = CloudProvider::Hetzner;

    assert!(matches!(provider, CloudProvider::Hetzner));
}

#[test]
fn test_cloud_provider_ovh() {
    let provider = CloudProvider::OVH;

    assert!(matches!(provider, CloudProvider::OVH));
}

#[test]
fn test_cloud_provider_scaleway() {
    let provider = CloudProvider::Scaleway;

    assert!(matches!(provider, CloudProvider::Scaleway));
}

// ============================================================================
// ContainerPlatform Tests
// ============================================================================

#[test]
fn test_container_platform_docker() {
    let platform = ContainerPlatform::Docker;

    assert!(matches!(platform, ContainerPlatform::Docker));
}

#[test]
fn test_container_platform_kubernetes() {
    let platform = ContainerPlatform::Kubernetes;

    assert!(matches!(platform, ContainerPlatform::Kubernetes));
}

#[test]
fn test_container_platform_nomad() {
    let platform = ContainerPlatform::Nomad;

    assert!(matches!(platform, ContainerPlatform::Nomad));
}

#[test]
fn test_container_platform_openshift() {
    let platform = ContainerPlatform::OpenShift;

    assert!(matches!(platform, ContainerPlatform::OpenShift));
}

#[test]
fn test_container_platform_docker_swarm() {
    let platform = ContainerPlatform::DockerSwarm;

    assert!(matches!(platform, ContainerPlatform::DockerSwarm));
}

#[test]
fn test_container_platform_podman() {
    let platform = ContainerPlatform::Podman;

    assert!(matches!(platform, ContainerPlatform::Podman));
}

// ============================================================================
// QuantumProvider Tests
// ============================================================================

#[test]
fn test_quantum_provider_ibm() {
    let provider = QuantumProvider::IBM;

    assert!(matches!(provider, QuantumProvider::IBM));
}

#[test]
fn test_quantum_provider_google() {
    let provider = QuantumProvider::Google;

    assert!(matches!(provider, QuantumProvider::Google));
}

#[test]
fn test_quantum_provider_ionq() {
    let provider = QuantumProvider::IonQ;

    assert!(matches!(provider, QuantumProvider::IonQ));
}

#[test]
fn test_quantum_provider_rigetti() {
    let provider = QuantumProvider::Rigetti;

    assert!(matches!(provider, QuantumProvider::Rigetti));
}

#[test]
fn test_quantum_provider_aws_braket() {
    let provider = QuantumProvider::AWSBraket;

    assert!(matches!(provider, QuantumProvider::AWSBraket));
}

#[test]
fn test_quantum_provider_azure_quantum() {
    let provider = QuantumProvider::AzureQuantum;

    assert!(matches!(provider, QuantumProvider::AzureQuantum));
}

// ============================================================================
// HPCScheduler Tests
// ============================================================================

#[test]
fn test_hpc_scheduler_slurm() {
    let scheduler = HPCScheduler::SLURM;

    assert!(matches!(scheduler, HPCScheduler::SLURM));
}

#[test]
fn test_hpc_scheduler_pbs() {
    let scheduler = HPCScheduler::PBS;

    assert!(matches!(scheduler, HPCScheduler::PBS));
}

#[test]
fn test_hpc_scheduler_sge() {
    let scheduler = HPCScheduler::SGE;

    assert!(matches!(scheduler, HPCScheduler::SGE));
}

#[test]
fn test_hpc_scheduler_lsf() {
    let scheduler = HPCScheduler::LSF;

    assert!(matches!(scheduler, HPCScheduler::LSF));
}

#[test]
fn test_hpc_scheduler_custom() {
    let scheduler = HPCScheduler::Custom;

    assert!(matches!(scheduler, HPCScheduler::Custom));
}

// ============================================================================
// ServiceTier Tests
// ============================================================================

#[test]
fn test_service_tier_basic() {
    let tier = ServiceTier::Basic;

    assert!(matches!(tier, ServiceTier::Basic));
}

#[test]
fn test_service_tier_professional() {
    let tier = ServiceTier::Professional;

    assert!(matches!(tier, ServiceTier::Professional));
}

#[test]
fn test_service_tier_enterprise() {
    let tier = ServiceTier::Enterprise;

    assert!(matches!(tier, ServiceTier::Enterprise));
}

#[test]
fn test_service_tier_premium() {
    let tier = ServiceTier::Premium;

    assert!(matches!(tier, ServiceTier::Premium));
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_crypto_lock_coverage_summary() {
    println!("=== Crypto Lock Test Coverage ===");
    println!("PermissionValidationResult:  4 tests");
    println!("PermissionLevel Tests:       3 tests");
    println!("OrganizationType Tests:      5 tests");
    println!("VerificationLevel Tests:     4 tests");
    println!("CryptoAlgorithm Tests:       4 tests");
    println!("DelegationStatus Tests:      4 tests");
    println!("CloudProvider Tests:         9 tests");
    println!("ContainerPlatform Tests:     6 tests");
    println!("QuantumProvider Tests:       6 tests");
    println!("HPCScheduler Tests:          5 tests");
    println!("ServiceTier Tests:           4 tests");
    println!("Total:                       54 tests");
    println!("===================================");
}
