//! Core cloud abstractions
//!
//! This module contains the fundamental cloud provider abstractions, including
//! the CloudProvider enum, CloudProviderInterface trait, and UniversalCloudOrchestrator.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Universal cloud provider abstraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    /// Amazon Web Services
    AWS {
        region: String,
        credentials: AWSCredentials,
        cost_budget: Option<f64>,
    },
    /// Microsoft Azure
    Azure {
        subscription: String,
        credentials: AzureCredentials,
        resource_group: String,
    },
    /// Google Cloud Platform
    GCP {
        project: String,
        credentials: GCPCredentials,
        zone: String,
    },
    /// DigitalOcean
    DigitalOcean { token: String, region: String },
    /// Linode
    Linode { token: String, region: String },
    /// Vultr
    Vultr { api_key: String, region: String },
    /// Hetzner Cloud
    Hetzner { token: String, location: String },
    /// OVH Cloud
    OVH {
        application_key: String,
        application_secret: String,
        consumer_key: String,
        region: String,
    },
    /// Scaleway
    Scaleway {
        access_key: String,
        secret_key: String,
        organization_id: String,
        zone: String,
    },
    /// BearDog Cloud (our own self-owned cloud!)
    BearDogCloud {
        endpoint: String,
        token: String,
        encryption_level: EncryptionLevel,
    },
    /// Self-hosted infrastructure
    SelfHosted {
        endpoints: Vec<String>,
        auth_method: AuthMethod,
    },
    /// Kubernetes cluster (any K8s, anywhere)
    Kubernetes {
        config: KubernetesConfig,
        namespace: String,
        storage_class: Option<String>, // nestGate backing
    },
    /// Edge/IoT devices
    EdgeDevices {
        device_registry: String,
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
#[async_trait]
pub trait CloudProviderInterface: Send + Sync {
    /// Deploy a job to this cloud provider
    async fn deploy_job(&self, job: &UniversalJob) -> ToadStoolResult<CloudJobHandle>;

    /// Get job status from this provider
    async fn get_job_status(&self, handle: &CloudJobHandle) -> ToadStoolResult<CloudJobStatus>;

    /// Scale resources for a job
    async fn scale_job(
        &self,
        handle: &CloudJobHandle,
        scale_config: ScaleConfig,
    ) -> ToadStoolResult<()>;

    /// Terminate a job
    async fn terminate_job(&self, handle: &CloudJobHandle) -> ToadStoolResult<()>;

    /// Get current pricing for resources
    async fn get_pricing(&self, resource_spec: &ResourceSpec) -> ToadStoolResult<PricingInfo>;

    /// Get current resource availability
    async fn get_availability(&self, region: Option<String>) -> ToadStoolResult<AvailabilityInfo>;

    /// Validate compliance requirements
    async fn validate_compliance(
        &self,
        requirements: &ResourceRequirements,
    ) -> ToadStoolResult<bool>;

    /// Get provider capabilities
    fn get_capabilities(&self) -> CloudCapabilities;

    /// Get provider metadata
    fn get_metadata(&self) -> CloudProviderMetadata;
}

// ============================================================================
// Universal Cloud Orchestrator
// ============================================================================

/// Universal Cloud Orchestrator - the brain of cloud operations
pub struct UniversalCloudOrchestrator {
    /// Available cloud providers
    pub(crate) providers: RwLock<HashMap<String, Box<dyn CloudProviderInterface>>>,
    /// Hybrid cloud scheduler
    pub(crate) hybrid_scheduler: HybridCloudScheduler,
    /// Cost optimizer across all clouds
    pub(crate) cost_optimizer: CloudCostOptimizer,
    /// Compliance enforcer (bearDog integration)
    pub(crate) compliance_enforcer: CloudComplianceEnforcer,
    /// Multi-cloud load balancer
    pub(crate) _load_balancer: MultiCloudLoadBalancer,
    /// Federation manager for cloud-to-cloud communication
    #[allow(dead_code)]
    pub(crate) federation_manager: CloudFederationManager,
}
