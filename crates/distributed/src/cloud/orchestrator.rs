//! Universal Cloud Orchestrator implementation
//!
//! This module contains the main implementation of the UniversalCloudOrchestrator,
//! including job deployment, multi-cloud strategies, and resource management.

use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::{ResourceRequirements, UniversalJob, UniversalJobType};

use super::core::{CloudProviderInterface, UniversalCloudOrchestrator};
use super::types::{
    AvailabilityInfo, BurstDistribution, CloudDeploymentResult, CloudOrchestratorConfig,
    DeploymentStrategy, FederatedDeployment, MultiCloudAvailability, MultiCloudDistribution,
};

impl UniversalCloudOrchestrator {
    /// Create new cloud orchestrator
    pub async fn new(config: CloudOrchestratorConfig) -> ToadStoolResult<Self> {
        let providers = RwLock::new(HashMap::new());
        let hybrid_scheduler =
            super::scheduling::HybridCloudScheduler::new(config.scheduling_strategy).await?;
        let cost_optimizer = super::cost::CloudCostOptimizer::new(config.cost_config).await?;
        let compliance_enforcer =
            super::compliance::CloudComplianceEnforcer::new(config.compliance_config).await?;
        let load_balancer =
            super::load_balancing::MultiCloudLoadBalancer::new(config.load_balancer_config).await?;
        let federation_manager =
            super::federation::CloudFederationManager::new(config.federation_config).await?;

        Ok(Self {
            providers,
            hybrid_scheduler,
            cost_optimizer,
            compliance_enforcer,
            _load_balancer: load_balancer,
            federation_manager,
        })
    }

    /// Register a cloud provider
    pub async fn register_provider(
        &mut self,
        name: String,
        provider: Box<dyn CloudProviderInterface>,
    ) -> ToadStoolResult<()> {
        info!("Registering cloud provider: {}", name);

        // Validate provider capabilities
        let capabilities = provider.get_capabilities();
        let metadata = provider.get_metadata();

        info!("Provider {} capabilities: {:?}", name, capabilities);
        info!("Provider {} metadata: {:?}", name, metadata);

        // Add to registry
        let mut providers = self.providers.write().await;
        providers.insert(name.clone(), provider);

        // Update cost models
        self.cost_optimizer
            .add_provider_cost_model(&name, &capabilities)
            .await?;

        // Update compliance checker
        self.compliance_enforcer
            .add_provider_compliance(&name, &capabilities)
            .await?;

        info!("Successfully registered cloud provider: {}", name);
        Ok(())
    }

    /// Deploy job across optimal cloud(s)
    pub async fn deploy_universal_job(
        &self,
        job: &UniversalJob,
    ) -> ToadStoolResult<CloudDeploymentResult> {
        info!(
            "Deploying universal job {} across optimal cloud(s)",
            job.job_id
        );

        // Analyze job requirements
        let deployment_strategy = self.analyze_deployment_requirements(job).await?;

        match deployment_strategy {
            DeploymentStrategy::SingleCloud { provider_name } => {
                self.deploy_to_single_cloud(job, &provider_name).await
            }
            DeploymentStrategy::MultiCloud {
                providers,
                distribution,
            } => {
                self.deploy_to_multiple_clouds(job, &providers, &distribution)
                    .await
            }
            DeploymentStrategy::HybridCloudBurst {
                primary,
                burst_providers,
            } => {
                self.deploy_with_cloud_burst(job, &primary, &burst_providers)
                    .await
            }
            DeploymentStrategy::FederatedDeployment { federation_nodes } => {
                self.deploy_to_federation(job, &federation_nodes).await
            }
        }
    }

    /// Analyze job to determine optimal deployment strategy
    async fn analyze_deployment_requirements(
        &self,
        job: &UniversalJob,
    ) -> ToadStoolResult<DeploymentStrategy> {
        // Check compliance requirements
        let compliance_constraints = self
            .compliance_enforcer
            .get_constraints_for_job(job)
            .await?;

        // Get cost estimates from all providers
        let _cost_estimates: HashMap<String, f64> = HashMap::new(); // Placeholder

        // Get performance estimates
        let _performance_estimates = self.hybrid_scheduler.get_performance_estimates(job).await?;

        // Get current availability
        let _availability = self.get_multi_cloud_availability().await?;

        // Apply scheduling strategy - simplified for now
        let providers = self.providers.read().await;
        let available_providers: Vec<String> = providers.keys().cloned().collect();

        let selected_providers = self
            .hybrid_scheduler
            .select_providers(job, &available_providers)
            .await?;

        if selected_providers.is_empty() {
            return Err(ToadStoolError::not_found(
                "No compliant providers available",
            ));
        }

        if selected_providers.len() == 1 {
            Ok(DeploymentStrategy::SingleCloud {
                provider_name: selected_providers[0].clone(),
            })
        } else if compliance_constraints.allowed_providers.len() > 1 {
            Ok(DeploymentStrategy::MultiCloud {
                providers: selected_providers.clone(),
                distribution: MultiCloudDistribution {
                    providers: selected_providers,
                    strategy: super::types::DistributionStrategy::Equal,
                },
            })
        } else {
            Ok(DeploymentStrategy::SingleCloud {
                provider_name: selected_providers[0].clone(),
            })
        }
    }

    /// Deploy to a single cloud provider
    async fn deploy_to_single_cloud(
        &self,
        job: &UniversalJob,
        provider_name: &str,
    ) -> ToadStoolResult<CloudDeploymentResult> {
        let providers = self.providers.read().await;
        let provider = providers.get(provider_name).ok_or_else(|| {
            ToadStoolError::not_found(format!("Cloud provider not found: {provider_name}"))
        })?;

        let handle = provider.deploy_job(job).await?;

        Ok(CloudDeploymentResult::Single {
            provider: provider_name.to_string(),
            handle,
        })
    }

    /// Deploy to multiple clouds simultaneously
    async fn deploy_to_multiple_clouds(
        &self,
        job: &UniversalJob,
        _providers: &[String],
        distribution: &MultiCloudDistribution,
    ) -> ToadStoolResult<CloudDeploymentResult> {
        let mut handles = HashMap::new();

        // Split job according to distribution strategy
        let job_parts = self.split_job_for_multi_cloud(job, distribution).await?;

        // Deploy each part to its assigned cloud
        for (provider_name, job_part) in job_parts {
            let providers_guard = self.providers.read().await;
            let provider = providers_guard.get(&provider_name).ok_or_else(|| {
                ToadStoolError::not_found(format!("Provider not found: {provider_name}"))
            })?;

            let handle = provider.deploy_job(&job_part).await?;
            handles.insert(provider_name, handle);
        }

        Ok(CloudDeploymentResult::Multi { handles })
    }

    /// Deploy with cloud bursting capability
    async fn deploy_with_cloud_burst(
        &self,
        job: &UniversalJob,
        primary_provider: &str,
        burst_providers: &[String],
    ) -> ToadStoolResult<CloudDeploymentResult> {
        // Try primary provider first
        let providers = self.providers.read().await;
        let primary = providers.get(primary_provider).ok_or_else(|| {
            ToadStoolError::not_found(format!("Primary provider not found: {primary_provider}"))
        })?;

        // Check if primary can handle the full load
        let availability = primary.get_availability(None).await?;

        if self.can_handle_full_job(&availability, &job.resource_requirements) {
            // Primary can handle it
            let handle = primary.deploy_job(job).await?;
            Ok(CloudDeploymentResult::Single {
                provider: primary_provider.to_string(),
                handle,
            })
        } else {
            // Need to burst to additional clouds
            let burst_distribution = self
                .calculate_burst_distribution(job, primary_provider, burst_providers, &availability)
                .await?;

            // Simplified cloud burst - deploy to first provider for now
            self.deploy_to_single_cloud(job, &burst_distribution.primary_provider)
                .await
        }
    }

    /// Deploy to federated cloud network
    async fn deploy_to_federation(
        &self,
        _job: &UniversalJob,
        _federation_nodes: &[String],
    ) -> ToadStoolResult<CloudDeploymentResult> {
        // Create federated deployment across multiple clouds
        // Each cloud provider handles its portion of the workload
        let federation_deployment = FederatedDeployment {
            federation_id: Uuid::new_v4(),
            nodes: vec![],
            // Use environment variable or default federation endpoint
            coordination_endpoint: std::env::var("TOADSTOOL_FEDERATION_ENDPOINT").unwrap_or_else(
                |_| {
                    format!(
                        "https://federation.{}:{}",
                        std::env::var("TOADSTOOL_DOMAIN")
                            .unwrap_or_else(|_| "toadstool.local".to_string()),
                        toadstool_config::defaults::network::FEDERATION_PORT
                    )
                },
            ),
        };

        Ok(CloudDeploymentResult::Federated {
            deployment: federation_deployment,
        })
    }

    /// Get availability across all clouds
    async fn get_multi_cloud_availability(&self) -> ToadStoolResult<MultiCloudAvailability> {
        let mut availability = MultiCloudAvailability::new();
        let providers = self.providers.read().await;

        for (name, provider) in providers.iter() {
            match provider.get_availability(None).await {
                Ok(provider_availability) => {
                    availability.add_provider(name.clone(), provider_availability);
                }
                Err(e) => {
                    warn!("Failed to get availability from provider {}: {}", name, e);
                    availability.mark_provider_unavailable(name.clone());
                }
            }
        }

        Ok(availability)
    }

    /// Check if provider can handle job requirements
    pub(crate) fn can_handle_full_job(
        &self,
        availability: &AvailabilityInfo,
        requirements: &ResourceRequirements,
    ) -> bool {
        availability.cpu_cores >= requirements.cpu.min_cores
            && availability.memory_gb
                >= (requirements.memory.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            && availability.storage_gb
                >= (requirements.storage.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }

    /// Split job for multi-cloud deployment
    async fn split_job_for_multi_cloud(
        &self,
        job: &UniversalJob,
        distribution: &MultiCloudDistribution,
    ) -> ToadStoolResult<HashMap<String, UniversalJob>> {
        match &job.job_type {
            Some(UniversalJobType::Local) => {
                // Can't split local jobs
                Err(ToadStoolError::not_supported(
                    "Cannot split local jobs across clouds",
                ))
            }
            Some(UniversalJobType::RemoteToadStool { .. }) => {
                // Remote ToadStool jobs can be replicated
                self.replicate_job_across_clouds(job, distribution).await
            }
            Some(UniversalJobType::EcosystemTool { .. }) => {
                // Ecosystem tool jobs can be load balanced
                self.load_balance_job_across_clouds(job, distribution).await
            }
            _ => {
                // Default: replicate job (handles None and other job types)
                self.replicate_job_across_clouds(job, distribution).await
            }
        }
    }

    /// Replicate job across multiple clouds
    async fn replicate_job_across_clouds(
        &self,
        job: &UniversalJob,
        distribution: &MultiCloudDistribution,
    ) -> ToadStoolResult<HashMap<String, UniversalJob>> {
        let mut jobs = HashMap::new();

        for provider_name in &distribution.providers {
            let mut replicated_job = job.clone();
            replicated_job.job_id = Uuid::new_v4(); // New job ID for replica
            jobs.insert(provider_name.clone(), replicated_job);
        }

        Ok(jobs)
    }

    /// Load balance job across clouds
    async fn load_balance_job_across_clouds(
        &self,
        job: &UniversalJob,
        distribution: &MultiCloudDistribution,
    ) -> ToadStoolResult<HashMap<String, UniversalJob>> {
        // For now, just replicate - more sophisticated splitting can be added later
        self.replicate_job_across_clouds(job, distribution).await
    }

    /// Calculate optimal burst distribution
    async fn calculate_burst_distribution(
        &self,
        job: &UniversalJob,
        primary_provider: &str,
        burst_providers: &[String],
        primary_availability: &AvailabilityInfo,
    ) -> ToadStoolResult<BurstDistribution> {
        // Calculate how much work primary can handle
        let primary_capacity =
            self.calculate_provider_capacity(primary_availability, &job.resource_requirements);

        // Distribute remaining work across burst providers
        let remaining_work = 1.0 - primary_capacity;
        let _burst_distribution = self
            .distribute_work_across_providers(remaining_work, burst_providers)
            .await?;

        let mut providers = vec![primary_provider.to_string()];
        providers.extend_from_slice(burst_providers);

        Ok(BurstDistribution {
            providers,
            primary_provider: primary_provider.to_string(),
        })
    }

    /// Calculate provider capacity for job
    pub(crate) fn calculate_provider_capacity(
        &self,
        availability: &AvailabilityInfo,
        requirements: &ResourceRequirements,
    ) -> f64 {
        let cpu_ratio = availability.cpu_cores / requirements.cpu.min_cores;
        let memory_ratio = availability.memory_gb
            / (requirements.memory.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
        let storage_ratio = availability.storage_gb
            / (requirements.storage.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0));

        // Take the minimum ratio as the limiting factor
        cpu_ratio.min(memory_ratio).min(storage_ratio).min(1.0)
    }

    /// Distribute work across burst providers
    async fn distribute_work_across_providers(
        &self,
        work_amount: f64,
        providers: &[String],
    ) -> ToadStoolResult<HashMap<String, f64>> {
        let mut distribution = HashMap::new();
        let work_per_provider = work_amount / providers.len() as f64;

        for provider in providers {
            distribution.insert(provider.clone(), work_per_provider);
        }

        Ok(distribution)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::{
        ComplianceConfig, CostConfig, FederationConfig, HybridSchedulingStrategy,
        LoadBalancerConfig, LoadBalancingAlgorithm,
    };
    use crate::types::resources::{
        CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
    };
    use std::time::Duration;

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
        assert!((cap - 0.5).abs() < 0.01); // 8/16 = 0.5
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
}
