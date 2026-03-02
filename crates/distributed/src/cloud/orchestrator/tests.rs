//! Cloud orchestrator tests

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::cloud::types::AvailabilityInfo;
    use crate::cloud::{
        CloudDeploymentResult, CloudOrchestratorConfig, CloudProviderInterface, ComplianceConfig,
        CostConfig, FederationConfig, HybridSchedulingStrategy, LoadBalancerConfig,
        LoadBalancingAlgorithm, UniversalCloudOrchestrator,
    };
    use crate::types::resources::{
        CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
    };
    use crate::{ResourceRequirements, UniversalJob, UniversalJobType};
    use std::time::{Duration, SystemTime};

    fn make_orchestrator_config() -> CloudOrchestratorConfig {
        CloudOrchestratorConfig {
            scheduling_strategy: HybridSchedulingStrategy::Balanced {
                cost_weight: 0.33,
                performance_weight: 0.33,
                compliance_weight: 0.34,
            },
            cost_config: CostConfig {
                budget_limit: None,
                cost_tracking_enabled: true,
                spot_instance_preference: 0.5,
            },
            compliance_config: ComplianceConfig {
                required_certifications: vec![],
                allowed_regions: vec!["us-east-1".to_string()],
                data_sovereignty_requirements: vec![],
            },
            load_balancer_config: LoadBalancerConfig {
                algorithm: LoadBalancingAlgorithm::RoundRobin,
                health_check_interval: Duration::from_secs(10),
                failover_timeout: Duration::from_secs(30),
            },
            federation_config: FederationConfig {
                federation_id: "test-fed".to_string(),
                discovery_endpoints: vec![],
                trust_anchors: vec![],
            },
        }
    }

    fn make_availability(cpu: f64, memory_gb: f64, storage_gb: f64) -> AvailabilityInfo {
        AvailabilityInfo {
            cpu_cores: cpu,
            memory_gb,
            storage_gb,
            gpu_count: 0,
            regions: vec![],
            availability_zones: vec![],
        }
    }

    fn make_requirements(cpu: f64, memory_bytes: u64, storage_bytes: u64) -> ResourceRequirements {
        ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: cpu,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: memory_bytes,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: storage_bytes,
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        }
    }

    fn make_mock_capabilities() -> crate::cloud::types::CloudCapabilities {
        use crate::cloud::types::{
            ComplianceCertification, ComputeType, NetworkingFeature, Region, SecurityFeature,
            StorageType,
        };
        crate::cloud::types::CloudCapabilities {
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

    fn make_mock_metadata(name: &str) -> crate::cloud::types::CloudProviderMetadata {
        crate::cloud::types::CloudProviderMetadata {
            name: name.to_string(),
            version: "1.0".to_string(),
            api_version: "v1".to_string(),
            supported_protocols: vec!["rest".to_string(), "grpc".to_string()],
            documentation_url: "https://example.com/docs".to_string(),
            support_contact: "support@example.com".to_string(),
        }
    }

    #[tokio::test]
    async fn test_can_handle_full_job_sufficient_resources() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(8.0, 16.0, 100.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
        assert!(orch.can_handle_full_job(&availability, &requirements));
    }

    #[tokio::test]
    async fn test_can_handle_full_job_insufficient_cpu() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(2.0, 64.0, 500.0);
        let requirements = make_requirements(8.0, 1024 * 1024 * 1024, 1024 * 1024 * 1024);
        assert!(!orch.can_handle_full_job(&availability, &requirements));
    }

    #[tokio::test]
    async fn test_can_handle_full_job_insufficient_memory() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(16.0, 2.0, 500.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 1024 * 1024 * 1024);
        assert!(!orch.can_handle_full_job(&availability, &requirements));
    }

    #[tokio::test]
    async fn test_calculate_provider_capacity_exact_fit() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(4.0, 8.0, 100.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!((cap - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_calculate_provider_capacity_cpu_limited() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(8.0, 32.0, 500.0);
        let requirements = make_requirements(16.0, 1024 * 1024 * 1024, 1024 * 1024 * 1024);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!((cap - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_calculate_provider_capacity_capped_at_one() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(32.0, 128.0, 1000.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!(cap <= 1.0);
    }

    #[tokio::test]
    async fn test_orchestrator_construction() {
        let config = make_orchestrator_config();
        let orch = UniversalCloudOrchestrator::new(config).await;
        assert!(orch.is_ok());
    }

    #[tokio::test]
    async fn test_can_handle_full_job_exact_resources() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(4.0, 8.0, 100.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
        assert!(orch.can_handle_full_job(&availability, &requirements));
    }

    #[tokio::test]
    async fn test_can_handle_full_job_insufficient_storage() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(16.0, 32.0, 10.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
        assert!(!orch.can_handle_full_job(&availability, &requirements));
    }

    #[tokio::test]
    async fn test_calculate_provider_capacity_memory_limited() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(16.0, 4.0, 500.0);
        let requirements = make_requirements(4.0, 16 * 1024 * 1024 * 1024, 1024 * 1024 * 1024);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!((cap - 0.25).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_calculate_provider_capacity_storage_limited() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(32.0, 128.0, 25.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!(cap < 1.0);
    }

    #[tokio::test]
    async fn test_calculate_provider_capacity_zero_requirements() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(1.0, 1.0, 1.0);
        let requirements = make_requirements(0.0, 0, 0);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!(cap <= 1.0);
    }

    #[tokio::test]
    async fn test_deploy_universal_job_no_providers() {
        use std::time::SystemTime;
        use toadstool::ExecutionRequest;
        use uuid::Uuid;

        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let job = UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: Some(UniversalJobType::ComputeIntensive),
            execution_request: ExecutionRequest::default(),
            target: crate::ExecutionTarget::Local,
            priority: crate::JobPriority::Normal,
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
            err.to_string().contains("No compliant providers")
                || err.to_string().contains("not found")
        );
    }

    #[tokio::test]
    async fn test_orchestrator_config_scheduling_strategies() {
        use crate::cloud::HybridSchedulingStrategy;

        let config_cost = CloudOrchestratorConfig {
            scheduling_strategy: HybridSchedulingStrategy::CostOptimized,
            ..make_orchestrator_config()
        };
        let orch = UniversalCloudOrchestrator::new(config_cost).await;
        assert!(orch.is_ok());

        let config_perf = CloudOrchestratorConfig {
            scheduling_strategy: HybridSchedulingStrategy::PerformanceOptimized,
            ..make_orchestrator_config()
        };
        let orch2 = UniversalCloudOrchestrator::new(config_perf).await;
        assert!(orch2.is_ok());
    }

    #[tokio::test]
    async fn test_availability_info_make_helper() {
        let avail = make_availability(8.0, 16.0, 100.0);
        assert_eq!(avail.cpu_cores, 8.0);
        assert_eq!(avail.memory_gb, 16.0);
        assert_eq!(avail.storage_gb, 100.0);
        assert_eq!(avail.gpu_count, 0);
    }

    #[tokio::test]
    async fn test_requirements_make_helper() {
        let req = make_requirements(2.0, 4096, 1024);
        assert_eq!(req.cpu.min_cores, 2.0);
        assert_eq!(req.memory.min_bytes, 4096);
        assert_eq!(req.storage.min_bytes, 1024);
    }

    #[tokio::test]
    async fn test_calculate_provider_capacity_all_above_requirements() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(100.0, 200.0, 1000.0);
        let requirements = make_requirements(2.0, 1024, 100);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!((cap - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_job_scheduling_across_providers_with_mock() {
        use async_trait::async_trait;
        use std::time::SystemTime;
        use toadstool::ExecutionRequest;
        use uuid::Uuid;

        struct MockCloudProvider {
            name: String,
            availability: AvailabilityInfo,
        }

        // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
        #[async_trait]
        impl CloudProviderInterface for MockCloudProvider {
            async fn deploy_job(
                &self,
                job: &UniversalJob,
            ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobHandle>
            {
                Ok(crate::cloud::types::CloudJobHandle {
                    job_id: job.job_id,
                    provider_job_id: format!("mock-{}", Uuid::new_v4()),
                    provider_name: self.name.clone(),
                    created_at: SystemTime::now(),
                })
            }

            async fn get_job_status(
                &self,
                _handle: &crate::cloud::types::CloudJobHandle,
            ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobStatus>
            {
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
                    cpu_cost_per_hour: 0.1,
                    memory_cost_per_gb_hour: 0.05,
                    storage_cost_per_gb_month: 0.01,
                    network_cost_per_gb: 0.02,
                    total_estimated_cost: 10.0,
                })
            }

            async fn get_availability(
                &self,
                _region: Option<String>,
            ) -> toadstool::error::ToadStoolResult<AvailabilityInfo> {
                Ok(self.availability.clone())
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
            target: crate::ExecutionTarget::Local,
            priority: crate::JobPriority::Normal,
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
    async fn test_resource_capacity_management_distribute_work() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(8.0, 16.0, 100.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!((cap - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_provider_selection_cpu_bottleneck() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(2.0, 128.0, 1000.0);
        let requirements = make_requirements(8.0, 4 * 1024 * 1024 * 1024, 50 * 1024 * 1024 * 1024);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!((cap - 0.25).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_provider_selection_storage_bottleneck() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(64.0, 256.0, 25.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
        assert!(!orch.can_handle_full_job(&availability, &requirements));
    }

    #[tokio::test]
    async fn test_error_handling_provider_not_found() {
        use std::time::SystemTime;
        use toadstool::ExecutionRequest;

        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let job = UniversalJob {
            job_id: uuid::Uuid::new_v4(),
            job_type: Some(UniversalJobType::StorageIntensive),
            execution_request: ExecutionRequest::default(),
            target: crate::ExecutionTarget::Local,
            priority: crate::JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: make_requirements(
                2.0,
                1024 * 1024 * 1024,
                10 * 1024 * 1024 * 1024,
            ),
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
        use async_trait::async_trait;

        struct MinimalMock;

        // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
        #[async_trait]
        impl CloudProviderInterface for MinimalMock {
            async fn deploy_job(
                &self,
                job: &crate::UniversalJob,
            ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobHandle>
            {
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
            ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobStatus>
            {
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
    async fn test_can_handle_full_job_boundary_exactly() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(4.0, 8.0, 100.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024);
        assert!(orch.can_handle_full_job(&availability, &requirements));
    }

    #[tokio::test]
    async fn test_calculate_provider_capacity_storage_ratio() {
        let orch = UniversalCloudOrchestrator::new(make_orchestrator_config())
            .await
            .unwrap();
        let availability = make_availability(8.0, 32.0, 50.0);
        let requirements = make_requirements(4.0, 8 * 1024 * 1024 * 1024, 200 * 1024 * 1024 * 1024);
        let cap = orch.calculate_provider_capacity(&availability, &requirements);
        assert!(cap < 0.3);
    }

    #[tokio::test]
    async fn test_availability_info_fields() {
        let avail = make_availability(4.0, 8.0, 50.0);
        assert_eq!(avail.gpu_count, 0);
        assert!(avail.regions.is_empty());
        assert!(avail.availability_zones.is_empty());
    }

    #[tokio::test]
    async fn test_requirements_memory_conversion() {
        let req = make_requirements(2.0, 16 * 1024 * 1024 * 1024, 100);
        let memory_gb = req.memory.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        assert!((memory_gb - 16.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_deployment_strategy_variants() {
        use crate::cloud::types::{
            DeploymentStrategy, DistributionStrategy, FederatedDeployment, MultiCloudDistribution,
        };

        let single = DeploymentStrategy::SingleCloud {
            provider_name: "aws".to_string(),
        };
        assert!(matches!(single, DeploymentStrategy::SingleCloud { .. }));

        let multi = DeploymentStrategy::MultiCloud {
            providers: vec!["aws".to_string(), "gcp".to_string()],
            distribution: MultiCloudDistribution {
                providers: vec!["aws".to_string(), "gcp".to_string()],
                strategy: DistributionStrategy::Equal,
            },
        };
        assert!(matches!(multi, DeploymentStrategy::MultiCloud { .. }));

        let fed = DeploymentStrategy::FederatedDeployment {
            federation_nodes: vec!["node-1".to_string()],
        };
        assert!(matches!(
            fed,
            DeploymentStrategy::FederatedDeployment { .. }
        ));

        let fed_deploy = FederatedDeployment {
            federation_id: uuid::Uuid::new_v4(),
            nodes: vec![],
            coordination_endpoint: "https://fed.example.com".to_string(),
        };
        assert!(fed_deploy.nodes.is_empty());
    }

    #[tokio::test]
    async fn test_cloud_deployment_result_variants() {
        use crate::cloud::types::{CloudDeploymentResult, CloudJobHandle, FederatedDeployment};
        use std::time::SystemTime;

        let single = CloudDeploymentResult::Single {
            provider: "aws".to_string(),
            handle: CloudJobHandle {
                job_id: uuid::Uuid::new_v4(),
                provider_job_id: "pj-1".to_string(),
                provider_name: "aws".to_string(),
                created_at: SystemTime::now(),
            },
        };
        assert!(matches!(single, CloudDeploymentResult::Single { .. }));

        let fed = CloudDeploymentResult::Federated {
            deployment: FederatedDeployment {
                federation_id: uuid::Uuid::new_v4(),
                nodes: vec![],
                coordination_endpoint: "https://x".to_string(),
            },
        };
        assert!(matches!(fed, CloudDeploymentResult::Federated { .. }));
    }
}
