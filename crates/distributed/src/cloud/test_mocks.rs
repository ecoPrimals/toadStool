// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared [`MockCloudProvider`] for unit tests and `tests/*.rs` integration tests.

use std::future::Future;
use std::time::SystemTime;
use uuid::Uuid;

use toadstool::error::ToadStoolResult;

use crate::cloud::CloudProviderInterface;
use crate::cloud::types::{
    AvailabilityInfo, CloudCapabilities, CloudJobHandle, CloudJobStatus, CloudProviderMetadata,
    PricingInfo, ResourceSpec, ScaleConfig,
};
use crate::{ResourceRequirements, UniversalJob};

fn make_mock_capabilities() -> CloudCapabilities {
    use crate::cloud::types::{
        ComplianceCertification, ComputeType, NetworkingFeature, Region, SecurityFeature,
        StorageType,
    };
    CloudCapabilities {
        compute_types: vec![ComputeType::VM, ComputeType::Container],
        storage_types: vec![StorageType::BlockStorage, StorageType::ObjectStorage],
        networking_features: vec![NetworkingFeature::VPC, NetworkingFeature::LoadBalancer],
        security_features: vec![SecurityFeature::Encryption, SecurityFeature::Compliance],
        compliance_certifications: vec![
            ComplianceCertification::SOC2,
            ComplianceCertification::ISO27001,
        ],
        regions: vec![Region {
            name: "us-east-1".to_string(),
            location: "Virginia".to_string(),
            availability_zones: vec!["us-east-1a".to_string(), "us-east-1b".to_string()],
        }],
        max_cpu_cores: Some(256),
        max_memory_gb: Some(1024),
        gpu_support: true,
        kubernetes_support: true,
        serverless_support: false,
    }
}

fn make_mock_metadata(name: &str) -> CloudProviderMetadata {
    CloudProviderMetadata {
        name: name.to_string(),
        version: "1.0".to_string(),
        api_version: "v1".to_string(),
        supported_protocols: vec!["rest".to_string(), "grpc".to_string()],
        documentation_url: "https://example.com/docs".to_string(),
        support_contact: "support@example.com".to_string(),
    }
}

/// Configurable mock cloud provider for tests (no production implementations yet).
pub struct MockCloudProvider {
    /// Provider label used in handles and metadata.
    pub name: String,
    /// Resource inventory returned when availability succeeds.
    pub availability: AvailabilityInfo,
    /// When set, returned from [`CloudProviderInterface::get_capabilities`] instead of defaults.
    pub capabilities_override: Option<CloudCapabilities>,
    /// When true, [`CloudProviderInterface::get_availability`] returns an error.
    pub fail_availability: bool,
}

impl CloudProviderInterface for MockCloudProvider {
    fn deploy_job<'a>(
        &'a self,
        job: &'a UniversalJob,
    ) -> impl Future<Output = ToadStoolResult<CloudJobHandle>> + Send + 'a {
        async move {
            Ok(CloudJobHandle {
                job_id: job.job_id,
                provider_job_id: format!("mock-{}", Uuid::new_v4()),
                provider_name: self.name.clone(),
                created_at: SystemTime::now(),
            })
        }
    }

    fn get_job_status<'a>(
        &'a self,
        _handle: &'a CloudJobHandle,
    ) -> impl Future<Output = ToadStoolResult<CloudJobStatus>> + Send + 'a {
        async move { Ok(CloudJobStatus::Running) }
    }

    fn scale_job<'a>(
        &'a self,
        _handle: &'a CloudJobHandle,
        _scale_config: ScaleConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move { Ok(()) }
    }

    fn terminate_job<'a>(
        &'a self,
        _handle: &'a CloudJobHandle,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move { Ok(()) }
    }

    fn get_pricing<'a>(
        &'a self,
        _resource_spec: &'a ResourceSpec,
    ) -> impl Future<Output = ToadStoolResult<PricingInfo>> + Send + 'a {
        async move {
            Ok(PricingInfo {
                cpu_cost_per_hour: 0.1,
                memory_cost_per_gb_hour: 0.05,
                storage_cost_per_gb_month: 0.01,
                network_cost_per_gb: 0.02,
                total_estimated_cost: 10.0,
            })
        }
    }

    fn get_availability<'a>(
        &'a self,
        _region: Option<String>,
    ) -> impl Future<Output = ToadStoolResult<AvailabilityInfo>> + Send + 'a {
        let availability = self.availability.clone();
        let fail = self.fail_availability;
        async move {
            if fail {
                Err(toadstool::error::ToadStoolError::not_found(
                    "availability probe failed",
                ))
            } else {
                Ok(availability)
            }
        }
    }

    fn validate_compliance<'a>(
        &'a self,
        _requirements: &'a ResourceRequirements,
    ) -> impl Future<Output = ToadStoolResult<bool>> + Send + 'a {
        async move { Ok(true) }
    }

    fn get_capabilities(&self) -> CloudCapabilities {
        self.capabilities_override
            .clone()
            .unwrap_or_else(make_mock_capabilities)
    }

    fn get_metadata(&self) -> CloudProviderMetadata {
        make_mock_metadata(&self.name)
    }
}
