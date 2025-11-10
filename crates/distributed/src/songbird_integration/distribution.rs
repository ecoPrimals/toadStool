//! Job distribution and splitting logic

use std::collections::HashMap;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use tracing::debug;
use uuid::Uuid;

use crate::{
    CpuRequirements, JobPriority, MemoryRequirements, ResourceRequirements, UniversalJob,
    UniversalJobType,
};

use super::types::{
    DistributionAlgorithm, DistributionConfig, JobAnalysis, JobComplexity, JobCoordinator,
    JobSplittingStrategy, LoadEstimator, MassiveJobDistributor, SubTask,
};

impl MassiveJobDistributor {
    pub async fn new(config: DistributionConfig) -> ToadStoolResult<Self> {
        // Initialize splitting strategies based on configuration
        let mut splitting_strategies = HashMap::new();

        // Default strategies for different job types
        splitting_strategies.insert(
            UniversalJobType::ComputeIntensive,
            JobSplittingStrategy::default(),
        );
        splitting_strategies.insert(
            UniversalJobType::DataProcessing,
            JobSplittingStrategy::default(),
        );
        splitting_strategies.insert(
            UniversalJobType::MachineLearning,
            JobSplittingStrategy::default(),
        );
        splitting_strategies.insert(
            UniversalJobType::Simulation,
            JobSplittingStrategy::default(),
        );

        // Add custom strategies from config
        for (job_type_str, strategy_str) in &config.splitting_strategies {
            if let Ok(job_type) = job_type_str.parse::<UniversalJobType>() {
                // Parse strategy configuration (simplified for now)
                splitting_strategies
                    .insert(job_type, JobSplittingStrategy::from_string(strategy_str));
            }
        }

        let distribution_algorithms = vec![
            DistributionAlgorithm::RoundRobin,
            DistributionAlgorithm::LoadBased,
            DistributionAlgorithm::CapabilityMatched,
            DistributionAlgorithm::GeographicOptimized,
        ];

        Ok(Self {
            splitting_strategies,
            distribution_algorithms,
            load_estimator: LoadEstimator::default(),
            job_coordinator: JobCoordinator::default(),
        })
    }

    pub async fn split_job(
        &self,
        job: &UniversalJob,
        analysis: &JobAnalysis,
    ) -> ToadStoolResult<Vec<SubTask>> {
        debug!(
            "Splitting job {} with complexity {:?}",
            job.job_id, analysis.complexity
        );

        let job_type = Self::determine_job_type(job);
        let _strategy = self
            .splitting_strategies
            .get(&job_type)
            .unwrap_or(&JobSplittingStrategy::default());

        match analysis.complexity {
            JobComplexity::Simple => {
                // Single subtask for simple jobs
                let job_payload = serde_json::to_vec(&job.execution_request).map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to serialize job: {e}"))
                })?;

                Ok(vec![SubTask {
                    id: Uuid::new_v4(),
                    payload: job_payload,
                    resource_requirements: analysis.resource_requirements.clone(),
                    priority: Self::convert_priority(job.priority),
                    constraints: analysis.preferred_node_types.clone(),
                }])
            }
            JobComplexity::Moderate => {
                // Split into 2-4 subtasks based on estimated parallelism
                let subtask_count = std::cmp::min(4, analysis.estimated_subtasks);
                self.create_subtasks(job, subtask_count, &analysis.resource_requirements)
            }
            JobComplexity::Complex => {
                // Split into 4-16 subtasks for better distribution
                let subtask_count = std::cmp::min(16, analysis.estimated_subtasks);
                self.create_subtasks(job, subtask_count, &analysis.resource_requirements)
            }
            JobComplexity::UltraMassive => {
                // Maximum parallelization for ultra-massive jobs
                let subtask_count = std::cmp::min(1000, analysis.estimated_subtasks);
                self.create_subtasks(job, subtask_count, &analysis.resource_requirements)
            }
        }
    }

    fn determine_job_type(job: &UniversalJob) -> UniversalJobType {
        // Use the job type if available, otherwise analyze characteristics
        if let Some(job_type) = &job.job_type {
            job_type.clone()
        } else {
            // Analyze execution request to determine type
            let request_str = format!("{:?}", job.execution_request);
            if request_str.contains("ml")
                || request_str.contains("ai")
                || request_str.contains("neural")
            {
                UniversalJobType::MachineLearning
            } else if request_str.contains("data")
                || request_str.contains("process")
                || request_str.contains("batch")
            {
                UniversalJobType::DataProcessing
            } else if request_str.contains("simulation")
                || request_str.contains("model")
                || request_str.contains("physics")
            {
                UniversalJobType::Simulation
            } else {
                UniversalJobType::ComputeIntensive
            }
        }
    }

    fn create_subtasks(
        &self,
        job: &UniversalJob,
        count: usize,
        base_requirements: &ResourceRequirements,
    ) -> ToadStoolResult<Vec<SubTask>> {
        let mut subtasks = Vec::new();
        let job_payload = serde_json::to_vec(&job.execution_request)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize job: {e}")))?;

        // Calculate resource allocation per subtask
        let cpu_per_task = base_requirements.cpu.min_cores / count as f64;
        let memory_per_task = base_requirements.memory.min_bytes / count as u64;

        for i in 0..count {
            let subtask_requirements = ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: cpu_per_task,
                    max_cores: base_requirements.cpu.max_cores.map(|c| c / count as f64),
                },
                memory: MemoryRequirements {
                    min_bytes: memory_per_task,
                    max_bytes: base_requirements.memory.max_bytes.map(|m| m / count as u64),
                },
                storage: base_requirements.storage.clone(),
                network: base_requirements.network.clone(),
                gpu: base_requirements.gpu.clone(),
            };

            // Create subtask with partition information
            let mut subtask_payload = job_payload.clone();
            // Add partition metadata (simplified)
            let partition_info = format!("{{\"partition\": {i}, \"total_partitions\": {count}}}");
            subtask_payload.extend(partition_info.as_bytes());

            subtasks.push(SubTask {
                id: Uuid::new_v4(),
                payload: subtask_payload,
                resource_requirements: subtask_requirements,
                priority: Self::convert_priority(job.priority),
                constraints: vec![format!("subtask_{}_of_{}", i + 1, count)],
            });
        }

        debug!("Created {} subtasks for job {}", subtasks.len(), job.job_id);
        Ok(subtasks)
    }

    fn convert_priority(priority: JobPriority) -> u8 {
        match priority {
            JobPriority::Background => 1,
            JobPriority::Low => 2,
            JobPriority::Normal => 5,
            JobPriority::High => 8,
            JobPriority::Critical => 10,
            JobPriority::Emergency => 15,
        }
    }
}

// Default implementation for JobSplittingStrategy
impl Default for JobSplittingStrategy {
    fn default() -> Self {
        use super::types::SplittingStrategyType;
        Self {
            strategy_type: SplittingStrategyType::DataParallel,
            max_subtasks: 100,
            min_subtask_size: 1024, // 1KB minimum
        }
    }
}

impl JobSplittingStrategy {
    pub fn from_string(strategy_str: &str) -> Self {
        use super::types::SplittingStrategyType;

        let strategy_type = match strategy_str {
            "data_parallel" => SplittingStrategyType::DataParallel,
            "task_parallel" => SplittingStrategyType::TaskParallel,
            "pipeline" => SplittingStrategyType::Pipeline,
            "map_reduce" => SplittingStrategyType::MapReduce,
            custom => SplittingStrategyType::Custom(custom.to_string()),
        };

        Self {
            strategy_type,
            max_subtasks: 100,
            min_subtask_size: 1024,
        }
    }
}
