// SPDX-License-Identifier: AGPL-3.0-only
//! Universal Cloud Orchestrator implementation
//!
//! This module contains the main implementation of the UniversalCloudOrchestrator,
//! including job deployment, multi-cloud strategies, and resource management.

use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::{ResourceRequirements, UniversalJob, UniversalJobType};

use crate::cloud::types::{
    AvailabilityInfo, BurstDistribution, CloudDeploymentResult, CloudOrchestratorConfig,
    DeploymentStrategy, DistributionStrategy, FederatedDeployment, MultiCloudAvailability,
    MultiCloudDistribution,
};
use crate::cloud::{
    CloudComplianceEnforcer, CloudCostOptimizer, CloudFederationManager, CloudProviderInterface,
    HybridCloudScheduler, MultiCloudLoadBalancer, UniversalCloudOrchestrator,
};

impl UniversalCloudOrchestrator {
    /// Create new cloud orchestrator
    pub async fn new(config: CloudOrchestratorConfig) -> ToadStoolResult<Self> {
        let providers = RwLock::new(HashMap::new());
        let hybrid_scheduler = HybridCloudScheduler::new(config.scheduling_strategy).await?;
        let cost_optimizer = CloudCostOptimizer::new(config.cost_config).await?;
        let compliance_enforcer = CloudComplianceEnforcer::new(config.compliance_config).await?;
        let load_balancer = MultiCloudLoadBalancer::new(config.load_balancer_config).await?;
        let federation_manager = CloudFederationManager::new(config.federation_config).await?;

        Ok(Self {
            providers,
            hybrid_scheduler,
            cost_optimizer,
            compliance_enforcer,
            _load_balancer: load_balancer,
            _federation_manager: federation_manager,
        })
    }

    /// Register a cloud provider
    pub async fn register_provider(
        &mut self,
        name: String,
        provider: Box<dyn CloudProviderInterface>,
    ) -> ToadStoolResult<()> {
        info!("Registering cloud provider: {}", name);

        let capabilities = provider.get_capabilities();
        let metadata = provider.get_metadata();

        info!("Provider {} capabilities: {:?}", name, capabilities);
        info!("Provider {} metadata: {:?}", name, metadata);

        {
            let mut providers = self.providers.write().await;
            providers.insert(name.clone(), provider);
        }

        self.cost_optimizer
            .add_provider_cost_model(&name, &capabilities)
            .await?;

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

        let deployment_strategy = self.analyze_deployment_requirements(job).await?;
        self.dispatch_deployment_strategy(job, deployment_strategy)
            .await
    }

    /// Route a resolved [`DeploymentStrategy`] to the corresponding deploy path.
    async fn dispatch_deployment_strategy(
        &self,
        job: &UniversalJob,
        deployment_strategy: DeploymentStrategy,
    ) -> ToadStoolResult<CloudDeploymentResult> {
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
        let compliance_constraints = self
            .compliance_enforcer
            .get_constraints_for_job(job)
            .await?;

        let _cost_estimates: HashMap<String, f64> = HashMap::new();

        let _performance_estimates = self.hybrid_scheduler.get_performance_estimates(job).await?;

        let availability = self.get_multi_cloud_availability().await?;

        let available_providers = availability.available_provider_names();

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
                    strategy: DistributionStrategy::Equal,
                },
            })
        } else {
            // `selected_providers` follows availability map iteration order; prefer providers
            // that pass compliance and sort for deterministic choice when several are listed.
            let allowed: HashSet<_> = compliance_constraints.allowed_providers.iter().collect();
            let mut compliant_selected: Vec<String> = selected_providers
                .iter()
                .filter(|p| allowed.contains(p))
                .cloned()
                .collect();
            if compliant_selected.is_empty() {
                compliant_selected.clone_from(&selected_providers);
            }
            compliant_selected.sort();
            Ok(DeploymentStrategy::SingleCloud {
                provider_name: compliant_selected[0].clone(),
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
        drop(providers);

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

        let job_parts = self.split_job_for_multi_cloud(job, distribution).await?;

        for (provider_name, job_part) in job_parts {
            let providers_guard = self.providers.read().await;
            let provider = providers_guard.get(&provider_name).ok_or_else(|| {
                ToadStoolError::not_found(format!("Provider not found: {provider_name}"))
            })?;

            let handle = provider.deploy_job(&job_part).await?;
            drop(providers_guard);
            handles.insert(provider_name, handle);
        }

        Ok(CloudDeploymentResult::Multi { handles })
    }

    /// Deploy with cloud bursting capability
    #[allow(clippy::significant_drop_tightening)] // Must hold lock - primary ref is from providers
    async fn deploy_with_cloud_burst(
        &self,
        job: &UniversalJob,
        primary_provider: &str,
        burst_providers: &[String],
    ) -> ToadStoolResult<CloudDeploymentResult> {
        let providers = self.providers.read().await;
        let primary = providers.get(primary_provider).ok_or_else(|| {
            ToadStoolError::not_found(format!("Primary provider not found: {primary_provider}"))
        })?;

        let availability = primary.get_availability(None).await?;

        if self.can_handle_full_job(&availability, &job.resource_requirements) {
            let handle = primary.deploy_job(job).await?;
            Ok(CloudDeploymentResult::Single {
                provider: primary_provider.to_string(),
                handle,
            })
        } else {
            let burst_distribution = self
                .calculate_burst_distribution(job, primary_provider, burst_providers, &availability)
                .await?;

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
        let federation_deployment = FederatedDeployment {
            federation_id: Uuid::new_v4(),
            nodes: vec![],
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
    #[allow(clippy::significant_drop_tightening)] // Must hold lock during iteration - provider refs are from lock
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
            Some(UniversalJobType::Local) => Err(ToadStoolError::not_supported(
                "Cannot split local jobs across clouds",
            )),
            Some(UniversalJobType::RemoteToadStool { .. }) => {
                self.replicate_job_across_clouds(job, distribution).await
            }
            Some(UniversalJobType::EcosystemTool { .. }) => {
                self.load_balance_job_across_clouds(job, distribution).await
            }
            _ => self.replicate_job_across_clouds(job, distribution).await,
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
            replicated_job.job_id = Uuid::new_v4();
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
        let primary_capacity =
            self.calculate_provider_capacity(primary_availability, &job.resource_requirements);

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

#[cfg(test)]
mod tests;
