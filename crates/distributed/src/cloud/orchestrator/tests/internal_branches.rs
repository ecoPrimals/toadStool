// SPDX-License-Identifier: AGPL-3.0-or-later
//! Direct coverage for `UniversalCloudOrchestrator` helpers and [`DeploymentStrategy`] dispatch.

use async_trait::async_trait;
use std::time::SystemTime;
use toadstool::ExecutionRequest;
use toadstool::error::ToadStoolError;
use uuid::Uuid;

use super::common::{
    MockCloudProvider, make_availability, make_mock_capabilities, make_mock_metadata,
    make_orchestrator_config, make_requirements,
};
use crate::cloud::types::{
    AvailabilityInfo, CloudDeploymentResult, CloudOrchestratorConfig, ComplianceCertification,
    DeploymentStrategy, DistributionStrategy, MultiCloudDistribution,
};
use crate::cloud::{CloudProviderInterface, UniversalCloudOrchestrator};
use crate::types::DistributedRetryConfig;
use crate::{ExecutionTarget, JobPriority, UniversalJob, UniversalJobType};

fn sample_job(job_type: Option<UniversalJobType>) -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type,
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    }
}

#[tokio::test]
async fn test_split_job_local_not_supported() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::Local));
    let dist = MultiCloudDistribution {
        providers: vec!["aws".to_string()],
        strategy: DistributionStrategy::Equal,
    };
    let err = orch
        .split_job_for_multi_cloud(&job, &dist)
        .await
        .unwrap_err();
    assert!(err.to_string().to_lowercase().contains("local"));
}

#[tokio::test]
async fn test_split_job_remote_toadstool_replicates() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::RemoteToadStool {
        endpoint: "http://r:1".to_string(),
    }));
    let dist = MultiCloudDistribution {
        providers: vec!["aws".to_string(), "gcp".to_string()],
        strategy: DistributionStrategy::Equal,
    };
    let parts = orch.split_job_for_multi_cloud(&job, &dist).await.unwrap();
    assert_eq!(parts.len(), 2);
    assert_ne!(
        parts["aws"].job_id, parts["gcp"].job_id,
        "replicated jobs should get distinct ids"
    );
}

#[tokio::test]
async fn test_split_job_ecosystem_tool_load_balance_path() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::EcosystemTool {
        tool_name: "t".to_string(),
        endpoint: "http://e:1".to_string(),
    }));
    let dist = MultiCloudDistribution {
        providers: vec!["a".to_string(), "b".to_string()],
        strategy: DistributionStrategy::Equal,
    };
    let parts = orch.split_job_for_multi_cloud(&job, &dist).await.unwrap();
    assert_eq!(parts.len(), 2);
}

#[tokio::test]
async fn test_split_job_none_job_type_replicates_default_branch() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let mut job = sample_job(None);
    job.job_type = None;
    let dist = MultiCloudDistribution {
        providers: vec!["aws".to_string()],
        strategy: DistributionStrategy::Equal,
    };
    let parts = orch.split_job_for_multi_cloud(&job, &dist).await.unwrap();
    assert_eq!(parts.len(), 1);
}

#[tokio::test]
async fn test_deploy_to_single_cloud_ok() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let mock = Box::new(MockCloudProvider {
        capabilities_override: None,
        name: "aws".to_string(),
        availability: make_availability(32.0, 64.0, 500.0),
    });
    orch.register_provider("aws".to_string(), mock)
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let res = orch.deploy_to_single_cloud(&job, "aws").await.unwrap();
    assert!(matches!(res, CloudDeploymentResult::Single { .. }));
}

#[tokio::test]
async fn test_deploy_to_single_cloud_missing_provider() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let err = orch
        .deploy_to_single_cloud(&job, "missing")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("missing") || err.to_string().contains("not found"));
}

#[tokio::test]
async fn test_deploy_to_multiple_clouds_two_providers() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    for name in ["aws", "gcp"] {
        orch.register_provider(
            name.to_string(),
            Box::new(MockCloudProvider {
                capabilities_override: None,
                name: name.to_string(),
                availability: make_availability(32.0, 64.0, 500.0),
            }),
        )
        .await
        .unwrap();
    }
    let job = sample_job(Some(UniversalJobType::RemoteToadStool {
        endpoint: "http://x".to_string(),
    }));
    let dist = MultiCloudDistribution {
        providers: vec!["aws".to_string(), "gcp".to_string()],
        strategy: DistributionStrategy::Equal,
    };
    let res = orch
        .deploy_to_multiple_clouds(&job, &["aws".to_string(), "gcp".to_string()], &dist)
        .await
        .unwrap();
    match res {
        CloudDeploymentResult::Multi { handles } => {
            assert_eq!(handles.len(), 2);
        }
        _ => panic!("expected Multi deployment"),
    }
}

#[tokio::test]
async fn test_deploy_to_multiple_clouds_missing_named_provider() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    orch.register_provider(
        "aws".to_string(),
        Box::new(MockCloudProvider {
            capabilities_override: None,
            name: "aws".to_string(),
            availability: make_availability(32.0, 64.0, 500.0),
        }),
    )
    .await
    .unwrap();
    let job = sample_job(Some(UniversalJobType::RemoteToadStool {
        endpoint: "http://x".to_string(),
    }));
    let dist = MultiCloudDistribution {
        providers: vec!["aws".to_string(), "orphan".to_string()],
        strategy: DistributionStrategy::Equal,
    };
    let err = orch
        .deploy_to_multiple_clouds(&job, &["aws".to_string(), "orphan".to_string()], &dist)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("orphan") || err.to_string().contains("not found"));
}

#[tokio::test]
async fn test_deploy_with_cloud_burst_primary_handles_all() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    orch.register_provider(
        "primary".to_string(),
        Box::new(MockCloudProvider {
            capabilities_override: None,
            name: "primary".to_string(),
            availability: make_availability(256.0, 512.0, 2000.0),
        }),
    )
    .await
    .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let res = orch
        .deploy_with_cloud_burst(&job, "primary", &["burst1".to_string()])
        .await
        .unwrap();
    assert!(matches!(res, CloudDeploymentResult::Single { .. }));
}

#[tokio::test]
async fn test_deploy_with_cloud_burst_insufficient_primary_uses_burst_path() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    orch.register_provider(
        "primary".to_string(),
        Box::new(MockCloudProvider {
            capabilities_override: None,
            name: "primary".to_string(),
            availability: make_availability(0.5, 0.5, 0.5),
        }),
    )
    .await
    .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let res = orch
        .deploy_with_cloud_burst(
            &job,
            "primary",
            &["burst1".to_string(), "burst2".to_string()],
        )
        .await
        .unwrap();
    assert!(matches!(res, CloudDeploymentResult::Single { .. }));
}

#[tokio::test]
async fn test_deploy_with_cloud_burst_primary_not_found() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let err = orch
        .deploy_with_cloud_burst(&job, "nope", &[])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("nope") || err.to_string().contains("not found"));
}

#[tokio::test]
async fn test_dispatch_deployment_strategy_multicloud_public_entrypoint() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    for name in ["aws", "gcp"] {
        orch.register_provider(
            name.to_string(),
            Box::new(MockCloudProvider {
                capabilities_override: None,
                name: name.to_string(),
                availability: make_availability(32.0, 64.0, 500.0),
            }),
        )
        .await
        .unwrap();
    }
    let job = sample_job(Some(UniversalJobType::RemoteToadStool {
        endpoint: "http://x".to_string(),
    }));
    let strategy = DeploymentStrategy::MultiCloud {
        providers: vec!["aws".to_string(), "gcp".to_string()],
        distribution: MultiCloudDistribution {
            providers: vec!["aws".to_string(), "gcp".to_string()],
            strategy: DistributionStrategy::Equal,
        },
    };
    let res = orch
        .dispatch_deployment_strategy(&job, strategy)
        .await
        .unwrap();
    assert!(matches!(res, CloudDeploymentResult::Multi { .. }));
}

#[tokio::test]
async fn test_dispatch_deployment_strategy_hybrid_burst_arm() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    orch.register_provider(
        "primary".to_string(),
        Box::new(MockCloudProvider {
            capabilities_override: None,
            name: "primary".to_string(),
            availability: make_availability(256.0, 512.0, 2000.0),
        }),
    )
    .await
    .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let strategy = DeploymentStrategy::HybridCloudBurst {
        primary: "primary".to_string(),
        burst_providers: vec!["burst".to_string()],
    };
    let res = orch
        .dispatch_deployment_strategy(&job, strategy)
        .await
        .unwrap();
    assert!(matches!(res, CloudDeploymentResult::Single { .. }));
}

#[tokio::test]
async fn test_dispatch_deployment_strategy_federated_arm() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let strategy = DeploymentStrategy::FederatedDeployment {
        federation_nodes: vec!["n1".to_string()],
    };
    let res = orch
        .dispatch_deployment_strategy(&job, strategy)
        .await
        .unwrap();
    assert!(matches!(res, CloudDeploymentResult::Federated { .. }));
}

#[tokio::test]
async fn test_deploy_to_federation_builds_coordination_endpoint() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let res = orch
        .deploy_to_federation(&job, &["node-a".to_string()])
        .await
        .unwrap();
    match res {
        CloudDeploymentResult::Federated { deployment } => {
            assert!(deployment.coordination_endpoint.contains("federation"));
        }
        _ => panic!("expected Federated result"),
    }
}

struct FailingAvailabilityProvider {
    name: String,
}

#[async_trait]
impl CloudProviderInterface for FailingAvailabilityProvider {
    async fn deploy_job(
        &self,
        job: &UniversalJob,
    ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobHandle> {
        Ok(crate::cloud::types::CloudJobHandle {
            job_id: job.job_id,
            provider_job_id: "x".to_string(),
            provider_name: self.name.clone(),
            created_at: SystemTime::now(),
        })
    }

    async fn get_job_status(
        &self,
        _handle: &crate::cloud::types::CloudJobHandle,
    ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobStatus> {
        Ok(crate::cloud::types::CloudJobStatus::Running)
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
        Err(ToadStoolError::not_found("availability probe failed"))
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
        make_mock_metadata(&self.name)
    }
}

#[tokio::test]
async fn test_get_multi_cloud_availability_marks_failure() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    orch.register_provider(
        "flake".to_string(),
        Box::new(FailingAvailabilityProvider {
            name: "flake".to_string(),
        }),
    )
    .await
    .unwrap();
    orch.register_provider(
        "good".to_string(),
        Box::new(MockCloudProvider {
            capabilities_override: None,
            name: "good".to_string(),
            availability: make_availability(4.0, 8.0, 100.0),
        }),
    )
    .await
    .unwrap();

    let availability = orch.get_multi_cloud_availability().await.unwrap();
    let dbg = format!("{availability:?}");
    assert!(
        dbg.contains("flake"),
        "unavailable provider should be recorded: {dbg}"
    );
}

#[tokio::test]
async fn test_distribute_work_across_two_burst_providers() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let map = orch
        .distribute_work_across_providers(1.0, &["a".to_string(), "b".to_string()])
        .await
        .unwrap();
    assert!((map["a"] - 0.5).abs() < f64::EPSILON);
    assert!((map["b"] - 0.5).abs() < f64::EPSILON);
}

#[tokio::test]
async fn test_analyze_deployment_requirements_empty_provider_list() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let err = orch
        .analyze_deployment_requirements(&job)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("No compliant providers"));
}

#[tokio::test]
async fn test_analyze_deployment_requirements_multi_selected_single_compliant_falls_back_to_single()
{
    let mut config: CloudOrchestratorConfig = make_orchestrator_config();
    config.compliance_config.required_certifications = vec![ComplianceCertification::HIPAA];
    let mut orch = UniversalCloudOrchestrator::new(config).await.unwrap();

    let mut caps_ok = make_mock_capabilities();
    caps_ok
        .compliance_certifications
        .push(ComplianceCertification::HIPAA);
    let caps_fail = make_mock_capabilities();

    orch.register_provider(
        "aws".to_string(),
        Box::new(MockCloudProvider {
            capabilities_override: Some(caps_ok),
            name: "aws".to_string(),
            availability: make_availability(32.0, 64.0, 500.0),
        }),
    )
    .await
    .unwrap();
    orch.register_provider(
        "gcp".to_string(),
        Box::new(MockCloudProvider {
            capabilities_override: Some(caps_fail),
            name: "gcp".to_string(),
            availability: make_availability(32.0, 64.0, 500.0),
        }),
    )
    .await
    .unwrap();

    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let strategy = orch.analyze_deployment_requirements(&job).await.unwrap();
    // Scheduler may list both providers; only `aws` carries HIPAA in this fixture, so the
    // single-cloud choice must be that compliant provider (not `selected_providers[0]` order).
    assert!(matches!(
        strategy,
        DeploymentStrategy::SingleCloud { ref provider_name } if provider_name == "aws"
    ));
}

#[tokio::test]
async fn test_analyze_deployment_requirements_multicloud_when_multiple_compliant() {
    let mut orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    for name in ["aws", "gcp"] {
        orch.register_provider(
            name.to_string(),
            Box::new(MockCloudProvider {
                capabilities_override: None,
                name: name.to_string(),
                availability: make_availability(32.0, 64.0, 500.0),
            }),
        )
        .await
        .unwrap();
    }
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let strategy = orch.analyze_deployment_requirements(&job).await.unwrap();
    assert!(
        matches!(
            strategy,
            DeploymentStrategy::SingleCloud { .. } | DeploymentStrategy::MultiCloud { .. }
        ),
        "Expected SingleCloud or MultiCloud, got {strategy:?}"
    );
}

#[tokio::test]
async fn test_calculate_burst_distribution_includes_burst_providers() {
    let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
        .await
        .unwrap();
    let job = sample_job(Some(UniversalJobType::ComputeIntensive));
    let primary_avail = make_availability(1.0, 1.0, 1.0);
    let dist = orch
        .calculate_burst_distribution(
            &job,
            "p",
            &["b1".to_string(), "b2".to_string()],
            &primary_avail,
        )
        .await
        .unwrap();
    assert!(dist.providers.contains(&"p".to_string()));
    assert!(dist.providers.iter().any(|s| s == "b1"));
}
