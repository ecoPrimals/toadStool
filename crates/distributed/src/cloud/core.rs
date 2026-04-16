// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core cloud abstractions
//!
//! This module contains the fundamental cloud provider abstractions, including
//! the CloudProvider enum, CloudProviderInterface trait, and UniversalCloudOrchestrator.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use toadstool::error::ToadStoolResult;
use tokio::sync::RwLock;

use crate::{ResourceRequirements, UniversalJob};

use super::credentials::{
    AWSCredentials, AuthMethod, AzureCredentials, EdgeMeshConfig, EncryptionLevel, GCPCredentials,
    KubernetesConfig,
};
use super::types::{
    AvailabilityInfo, CloudCapabilities, CloudJobHandle, CloudJobStatus, CloudProviderMetadata,
    PricingInfo, ResourceSpec, ScaleConfig,
};

// Import related modules (will be created in later phases)
use super::compliance::CloudComplianceEnforcer;
use super::cost::CloudCostOptimizer;
use super::federation::CloudFederationManager;
use super::load_balancing::MultiCloudLoadBalancer;
use super::scheduling::HybridCloudScheduler;

// ============================================================================
// Cloud Provider Enum
// ============================================================================

/// Universal cloud provider abstraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    /// Amazon Web Services.
    AWS {
        /// AWS region.
        region: String,
        /// AWS credentials.
        credentials: AWSCredentials,
        /// Cost budget (optional).
        cost_budget: Option<f64>,
    },
    /// Microsoft Azure.
    Azure {
        /// Subscription ID.
        subscription: String,
        /// Azure credentials.
        credentials: AzureCredentials,
        /// Resource group.
        resource_group: String,
    },
    /// Google Cloud Platform.
    GCP {
        /// GCP project ID.
        project: String,
        /// GCP credentials.
        credentials: GCPCredentials,
        /// Zone.
        zone: String,
    },
    /// DigitalOcean.
    DigitalOcean {
        /// API token.
        token: String,
        /// Region.
        region: String,
    },
    /// Linode.
    Linode {
        /// API token.
        token: String,
        /// Region.
        region: String,
    },
    /// Vultr.
    Vultr {
        /// API key.
        api_key: String,
        /// Region.
        region: String,
    },
    /// Hetzner Cloud.
    Hetzner {
        /// API token.
        token: String,
        /// Location.
        location: String,
    },
    /// OVH Cloud.
    OVH {
        /// Application key.
        application_key: String,
        /// Application secret.
        application_secret: String,
        /// Consumer key.
        consumer_key: String,
        /// Region.
        region: String,
    },
    /// Scaleway.
    Scaleway {
        /// Access key.
        access_key: String,
        /// Secret key.
        secret_key: String,
        /// Organization ID.
        organization_id: String,
        /// Zone.
        zone: String,
    },
    /// Self-managed cloud with strong encryption and token auth.
    #[serde(alias = "BearDogCloud")] // legacy alias
    PrivateSecurityCloud {
        /// Endpoint URL.
        endpoint: String,
        /// Auth token.
        token: String,
        /// Encryption level.
        encryption_level: EncryptionLevel,
    },
    /// Self-hosted infrastructure.
    SelfHosted {
        /// Endpoint URLs.
        endpoints: Vec<String>,
        /// Auth method.
        auth_method: AuthMethod,
    },
    /// Kubernetes cluster.
    Kubernetes {
        /// K8s config.
        config: KubernetesConfig,
        /// Namespace.
        namespace: String,
        /// Storage class (optional).
        storage_class: Option<String>,
    },
    /// Edge/IoT devices.
    EdgeDevices {
        /// Device registry endpoint.
        device_registry: String,
        /// Mesh network config.
        mesh_network: EdgeMeshConfig,
    },
}

impl Default for CloudProvider {
    fn default() -> Self {
        Self::AWS {
            region: "us-east-1".to_string(),
            credentials: AWSCredentials::default(),
            cost_budget: None,
        }
    }
}

// ============================================================================
// Cloud Provider Interface
// ============================================================================

/// Cloud provider interface - every cloud must implement this
pub trait CloudProviderInterface: Send + Sync {
    /// Deploy a job to this cloud provider
    fn deploy_job<'a>(
        &'a self,
        job: &'a UniversalJob,
    ) -> impl Future<Output = ToadStoolResult<CloudJobHandle>> + Send + 'a;

    /// Get job status from this provider
    fn get_job_status<'a>(
        &'a self,
        handle: &'a CloudJobHandle,
    ) -> impl Future<Output = ToadStoolResult<CloudJobStatus>> + Send + 'a;

    /// Scale resources for a job
    fn scale_job<'a>(
        &'a self,
        handle: &'a CloudJobHandle,
        scale_config: ScaleConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Terminate a job
    fn terminate_job<'a>(
        &'a self,
        handle: &'a CloudJobHandle,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Get current pricing for resources
    fn get_pricing<'a>(
        &'a self,
        resource_spec: &'a ResourceSpec,
    ) -> impl Future<Output = ToadStoolResult<PricingInfo>> + Send + 'a;

    /// Get current resource availability
    fn get_availability<'a>(
        &'a self,
        region: Option<String>,
    ) -> impl Future<Output = ToadStoolResult<AvailabilityInfo>> + Send + 'a;

    /// Validate compliance requirements
    fn validate_compliance<'a>(
        &'a self,
        requirements: &'a ResourceRequirements,
    ) -> impl Future<Output = ToadStoolResult<bool>> + Send + 'a;

    /// Get provider capabilities
    fn get_capabilities(&self) -> CloudCapabilities;

    /// Get provider metadata
    fn get_metadata(&self) -> CloudProviderMetadata;
}

// ============================================================================
// Universal Cloud Orchestrator
// ============================================================================

/// Universal Cloud Orchestrator - the brain of cloud operations
pub struct UniversalCloudOrchestrator<P: CloudProviderInterface> {
    /// Available cloud providers
    pub(crate) providers: RwLock<HashMap<String, P>>,
    /// Hybrid cloud scheduler
    pub(crate) hybrid_scheduler: HybridCloudScheduler,
    /// Cost optimizer across all clouds
    pub(crate) cost_optimizer: CloudCostOptimizer,
    /// Compliance enforcer (security integration)
    pub(crate) compliance_enforcer: CloudComplianceEnforcer,
    /// Multi-cloud load balancer
    pub(crate) _load_balancer: MultiCloudLoadBalancer,
    /// Federation manager for cloud-to-cloud communication (Phase 2+)
    pub(crate) _federation_manager: CloudFederationManager,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_provider_default_is_aws() {
        let provider = CloudProvider::default();
        assert!(matches!(provider, CloudProvider::AWS { .. }));
    }

    #[test]
    fn test_cloud_provider_default_us_east_1() {
        if let CloudProvider::AWS {
            region,
            cost_budget,
            ..
        } = CloudProvider::default()
        {
            assert_eq!(region, "us-east-1");
            assert!(cost_budget.is_none());
        } else {
            unreachable!("expected AWS variant");
        }
    }

    #[test]
    fn test_cloud_provider_variants_constructible() {
        let do_provider = CloudProvider::DigitalOcean {
            token: "test-token".to_string(),
            region: "nyc1".to_string(),
        };
        assert!(matches!(do_provider, CloudProvider::DigitalOcean { .. }));

        let hetzner = CloudProvider::Hetzner {
            token: "hetzner-token".to_string(),
            location: "nbg1".to_string(),
        };
        assert!(matches!(hetzner, CloudProvider::Hetzner { .. }));
    }
}
