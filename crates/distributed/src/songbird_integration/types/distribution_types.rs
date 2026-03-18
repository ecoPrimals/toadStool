// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metrics, job splitting, load estimation, and coordination types

use uuid::Uuid;

use crate::{ResourceRequirements, UniversalJob};

use super::job_types::{
    CompletionStrategy, CoordinationJob, CoordinationStrategy, DistributionPlan, SubTask,
};

// ============================================================================
// Metrics and Distribution Support
// ============================================================================

pub struct LoadMetric {
    pub cpu_load: f64,
    pub memory_load: f64,
    pub network_load: f64,
}

pub struct JobSplittingStrategy {
    pub strategy_type: SplittingStrategyType,
    pub max_subtasks: usize,
    pub min_subtask_size: usize,
}

impl JobSplittingStrategy {
    pub async fn split_job(&self, job: &UniversalJob) -> Vec<SubTask> {
        if self.max_subtasks <= 1 {
            return vec![];
        }
        let cpu_cores = job.resource_requirements.cpu.min_cores as usize;
        let num_subtasks = std::cmp::min(self.max_subtasks, cpu_cores.max(2));
        match &self.strategy_type {
            SplittingStrategyType::DataParallel => {
                self.split_data_parallel(job, num_subtasks).await
            }
            SplittingStrategyType::TaskParallel => {
                self.split_task_parallel(job, num_subtasks).await
            }
            SplittingStrategyType::MapReduce => self.split_map_reduce(job, num_subtasks).await,
            _ => self.split_task_parallel(job, num_subtasks).await,
        }
    }

    async fn split_data_parallel(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        let mut subtasks = Vec::with_capacity(num_subtasks);
        let per_task_cpu = (job.resource_requirements.cpu.min_cores / num_subtasks as f64).max(0.5);
        let per_task_memory = job.resource_requirements.memory.min_bytes / num_subtasks as u64;
        for i in 0..num_subtasks {
            subtasks.push(SubTask {
                id: Uuid::new_v4(),
                payload: bytes::Bytes::new(),
                resource_requirements: ResourceRequirements {
                    cpu: crate::types::resources::CpuRequirements {
                        min_cores: per_task_cpu,
                        max_cores: job
                            .resource_requirements
                            .cpu
                            .max_cores
                            .map(|c| c / num_subtasks as f64),
                    },
                    memory: crate::types::resources::MemoryRequirements {
                        min_bytes: per_task_memory,
                        max_bytes: job
                            .resource_requirements
                            .memory
                            .max_bytes
                            .map(|m| m / num_subtasks as u64),
                    },
                    storage: job.resource_requirements.storage.clone(),
                    network: job.resource_requirements.network.clone(),
                    gpu: job.resource_requirements.gpu.clone(),
                },
                priority: job.priority as u8,
                constraints: vec![format!("chunk_{}_of_{}", i, num_subtasks)],
            });
        }
        subtasks
    }

    async fn split_task_parallel(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        (0..num_subtasks)
            .map(|i| SubTask {
                id: Uuid::new_v4(),
                payload: bytes::Bytes::new(),
                resource_requirements: job.resource_requirements.clone(),
                priority: job.priority as u8,
                constraints: vec![format!("task_{}_of_{}", i, num_subtasks)],
            })
            .collect()
    }

    async fn split_map_reduce(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        self.split_data_parallel(job, num_subtasks).await
    }
}

pub enum SplittingStrategyType {
    DataParallel,
    TaskParallel,
    Pipeline,
    MapReduce,
    Custom(String),
}

pub type DistributionAlgorithm = crate::common::distribution::DistributionAlgorithm;

pub struct LoadEstimator {
    pub estimation_model: String,
}

impl Default for LoadEstimator {
    fn default() -> Self {
        Self {
            estimation_model: "linear".to_string(),
        }
    }
}

impl LoadEstimator {
    pub async fn estimate_load(&self, job: &UniversalJob) -> LoadMetric {
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(4.0);
        #[allow(clippy::cast_precision_loss)]
        let total_memory = toadstool_sysmon::memory_info()
            .map(|m| m.total as f64)
            .unwrap_or(1.0);
        let cpu_load = (job.resource_requirements.cpu.min_cores / cpu_cores).min(1.0);
        let memory_load =
            (job.resource_requirements.memory.min_bytes as f64 / total_memory).min(1.0);
        let default_network = match &job.job_type {
            Some(crate::types::jobs::UniversalJobType::Local) => 0.1,
            Some(crate::types::jobs::UniversalJobType::Native) => 0.1,
            Some(crate::types::jobs::UniversalJobType::RemoteToadStool { .. }) => 0.3,
            Some(crate::types::jobs::UniversalJobType::EcosystemTool { .. }) => 0.2,
            Some(crate::types::jobs::UniversalJobType::RecursiveHosting { .. }) => 0.4,
            Some(crate::types::jobs::UniversalJobType::NetworkIntensive) => 0.8,
            Some(crate::types::jobs::UniversalJobType::DataProcessing) => 0.4,
            Some(crate::types::jobs::UniversalJobType::MachineLearning) => 0.3,
            Some(_) => 0.2,
            None => 0.2,
        };
        let network_load = job
            .resource_requirements
            .network
            .bandwidth_mbps
            .map_or(default_network, |bandwidth| {
                (bandwidth as f64 / 1000.0).min(1.0)
            });
        LoadMetric {
            cpu_load,
            memory_load,
            network_load,
        }
    }
}

pub struct JobCoordinator {
    pub coordination_strategy: String,
}

impl Default for JobCoordinator {
    fn default() -> Self {
        Self {
            coordination_strategy: "parallel".to_string(),
        }
    }
}

impl JobCoordinator {
    pub async fn coordinate(&self, plan: &DistributionPlan) -> CoordinationJob {
        let completion_strategy = match plan.coordination_strategy {
            CoordinationStrategy::Sequential => CompletionStrategy::WaitForAll,
            CoordinationStrategy::Parallel => CompletionStrategy::WaitForAll,
            CoordinationStrategy::Pipeline => CompletionStrategy::WaitForAll,
            CoordinationStrategy::MapReduce => CompletionStrategy::WaitForAll,
        };
        CoordinationJob {
            job_id: Uuid::new_v4(),
            original_job_id: plan.job_id,
            subtask_count: plan.subtasks.len(),
            completion_strategy,
        }
    }

    #[must_use]
    pub fn with_strategy(strategy: &str) -> Self {
        Self {
            coordination_strategy: strategy.to_string(),
        }
    }
}
