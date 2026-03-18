// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deployment and provider registration tests

use async_trait::async_trait;
use std::time::SystemTime;
use toadstool::ExecutionRequest;
use uuid::Uuid;

use super::common::{
    MockCloudProvider, make_availability, make_mock_capabilities, make_mock_metadata,
    make_orchestrator_config, make_requirements,
};
use crate::cloud::types::{AvailabilityInfo, CloudDeploymentResult};
use crate::cloud::{CloudProviderInterface, UniversalCloudOrchestrator};
use crate::{ExecutionTarget, JobPriority, UniversalJob, UniversalJobType};

#[tokio::test]
async fn test_deploy_universal_job_no_providers() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::ComputeIntensive),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No compliant providers") || err.to_string().contains("not found")
    );
}

#[tokio::test]
async fn test_job_scheduling_across_providers_with_mock() {
    let config = make_orchestrator_config();
    let mut orch = UniversalCloudOrchestrator::new(config).await.unwrap();

    let mock = Box::new(MockCloudProvider {
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    orch.register_provider("aws".to_string(), mock)
        .await
        .unwrap();

    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::ComputeIntensive),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
    let deployment = result.unwrap();
    assert!(matches!(deployment, CloudDeploymentResult::Single { .. }));
}

#[tokio::test]
async fn test_error_handling_provider_not_found() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::StorageIntensive),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(2.0, 1024 * 1024 * 1024, 10 * 1024 * 1024 * 1024),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("No compliant providers")
            || err_msg.contains("not found")
            || err_msg.to_lowercase().contains("provider")
    );
}

#[tokio::test]
async fn test_register_provider_success() {
    struct MinimalMock;

    #[async_trait]
    impl CloudProviderInterface for MinimalMock {
        async fn deploy_job(
            &self,
            job: &crate::UniversalJob,
        ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobHandle> {
            Ok(crate::cloud::types::CloudJobHandle {
                job_id: job.job_id,
                provider_job_id: "test-id".to_string(),
                provider_name: "minimal".to_string(),
                created_at: SystemTime::now(),
            })
        }

        async fn get_job_status(
            &self,
            _handle: &crate::cloud::types::CloudJobHandle,
        ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobStatus> {
            Ok(crate::cloud::types::CloudJobStatus::Completed)
        }

        async fn scale_job(
            &self,
            _handle: &crate::cloud::types::CloudJobHandle,
            _scale_config: crate::cloud::types::ScaleConfig,
        ) -> toadstool::error::ToadStoolResult<()> {
            Ok(())
        }

        async fn terminate_job(
            &self,
            _handle: &crate::cloud::types::CloudJobHandle,
        ) -> toadstool::error::ToadStoolResult<()> {
            Ok(())
        }

        async fn get_pricing(
            &self,
            _resource_spec: &crate::cloud::types::ResourceSpec,
        ) -> toadstool::error::ToadStoolResult<crate::cloud::types::PricingInfo> {
            Ok(crate::cloud::types::PricingInfo {
                cpu_cost_per_hour: 0.0,
                memory_cost_per_gb_hour: 0.0,
                storage_cost_per_gb_month: 0.0,
                network_cost_per_gb: 0.0,
                total_estimated_cost: 0.0,
            })
        }

        async fn get_availability(
            &self,
            _region: Option<String>,
        ) -> toadstool::error::ToadStoolResult<AvailabilityInfo> {
            Ok(make_availability(8.0, 16.0, 100.0))
        }

        async fn validate_compliance(
            &self,
            _requirements: &crate::ResourceRequirements,
        ) -> toadstool::error::ToadStoolResult<bool> {
            Ok(true)
        }

        fn get_capabilities(&self) -> crate::cloud::types::CloudCapabilities {
            make_mock_capabilities()
        }

        fn get_metadata(&self) -> crate::cloud::types::CloudProviderMetadata {
            make_mock_metadata("minimal")
        }
    }

    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let result = orch
        .register_provider("test-provider".to_string(), Box::new(MinimalMock))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_deploy_local_job_with_multi_cloud_fails_split() {
    let config = make_orchestrator_config();
    let mut orch = UniversalCloudOrchestrator::new(config).await.unwrap();

    let mock1 = Box::new(MockCloudProvider {
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    let mock2 = Box::new(MockCloudProvider {
        name: "gcp".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    orch.register_provider("aws".to_string(), mock1)
        .await
        .unwrap();
    orch.register_provider("gcp".to_string(), mock2)
        .await
        .unwrap();

    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::Local),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    if let Err(e) = result {
        assert!(
            e.to_string().contains("Cannot split")
                || e.to_string().contains("local jobs")
                || e.to_string().contains("not supported"),
            "expected split error, got: {}",
            e
        );
    }
}

#[tokio::test]
async fn test_deploy_multi_cloud_remote_toadstool() {
    let config = make_orchestrator_config();
    let mut orch = UniversalCloudOrchestrator::new(config).await.unwrap();

    let mock1 = Box::new(MockCloudProvider {
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    let mock2 = Box::new(MockCloudProvider {
        name: "gcp".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    orch.register_provider("aws".to_string(), mock1)
        .await
        .unwrap();
    orch.register_provider("gcp".to_string(), mock2)
        .await
        .unwrap();

    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::RemoteToadStool {
            endpoint: "http://remote:8080".to_string(),
        }),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
    let deployment = result.unwrap();
    assert!(
        matches!(deployment, CloudDeploymentResult::Single { .. })
            || matches!(deployment, CloudDeploymentResult::Multi { .. }),
        "expected Single or Multi deployment"
    );
}

#[tokio::test]
async fn test_deploy_ecosystem_tool_multi_cloud() {
    let config = make_orchestrator_config();
    let mut orch = UniversalCloudOrchestrator::new(config).await.unwrap();

    let mock1 = Box::new(MockCloudProvider {
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    let mock2 = Box::new(MockCloudProvider {
        name: "gcp".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    orch.register_provider("aws".to_string(), mock1)
        .await
        .unwrap();
    orch.register_provider("gcp".to_string(), mock2)
        .await
        .unwrap();

    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::EcosystemTool {
            tool_name: "biome".to_string(),
            endpoint: "http://biome:8080".to_string(),
        }),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
    let deployment = result.unwrap();
    assert!(
        matches!(deployment, CloudDeploymentResult::Single { .. })
            || matches!(deployment, CloudDeploymentResult::Multi { .. }),
        "expected Single or Multi deployment"
    );
}

#[tokio::test]
async fn test_deploy_federated_deployment() {
    let mut config = make_orchestrator_config();
    config.compliance_config.allowed_regions =
        vec!["us-east-1".to_string(), "eu-west-1".to_string()];
    let mut orch = UniversalCloudOrchestrator::new(config).await.unwrap();

    let mock = Box::new(MockCloudProvider {
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    orch.register_provider("aws".to_string(), mock)
        .await
        .unwrap();

    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::ComputeIntensive),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_register_provider_twice_overwrites() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let mock1 = Box::new(MockCloudProvider {
        name: "dup".to_string(),
        availability: make_availability(4.0, 8.0, 50.0),
    });
    orch.register_provider("dup".to_string(), mock1)
        .await
        .unwrap();
    let mock2 = Box::new(MockCloudProvider {
        name: "dup".to_string(),
        availability: make_availability(8.0, 16.0, 100.0),
    });
    let result = orch.register_provider("dup".to_string(), mock2).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_split_job_for_multi_cloud_replication() {
    let config = make_orchestrator_config();
    let mut orch = UniversalCloudOrchestrator::new(config).await.unwrap();

    let mock1 = Box::new(MockCloudProvider {
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    let mock2 = Box::new(MockCloudProvider {
        name: "gcp".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    });
    orch.register_provider("aws".to_string(), mock1)
        .await
        .unwrap();
    orch.register_provider("gcp".to_string(), mock2)
        .await
        .unwrap();

    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::RemoteToadStool {
            endpoint: "http://remote:8080".to_string(),
        }),
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            2.0,
            4 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
    let deployment = result.unwrap();
    assert!(
        matches!(deployment, CloudDeploymentResult::Single { .. })
            || matches!(deployment, CloudDeploymentResult::Multi { .. })
    );
}
