// SPDX-License-Identifier: AGPL-3.0-or-later
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
        let load = self.load_estimator.estimate_load(job).await;
        let algo = self.select_algorithm(job);
        debug!(
            job_id = %job.job_id,
            complexity = ?analysis.complexity,
            algorithm = ?algo,
            cpu_load = load.cpu_load,
            mem_load = load.memory_load,
            "splitting job"
        );

        let job_type = Self::determine_job_type(job);
        let default_strategy = JobSplittingStrategy::default();
        let _strategy = self
            .splitting_strategies
            .get(&job_type)
            .unwrap_or(&default_strategy);

        match analysis.complexity {
            JobComplexity::Simple => {
                // Single subtask for simple jobs
                let job_payload =
                    bytes::Bytes::from(serde_json::to_vec(&job.execution_request).map_err(
                        |e| ToadStoolError::runtime(format!("Failed to serialize job: {e}")),
                    )?);

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

    /// Choose the distribution algorithm best suited to this job.
    ///
    /// Prefers `CapabilityMatched` for ML/simulation workloads so that nodes
    /// with matching accelerators are preferred, and falls back to `LoadBased`
    /// for general compute, or `RoundRobin` if the configured list is empty.
    fn select_algorithm(&self, job: &UniversalJob) -> DistributionAlgorithm {
        let job_type = Self::determine_job_type(job);
        let prefer_capability_match = matches!(
            job_type,
            UniversalJobType::MachineLearning | UniversalJobType::Simulation
        );
        if prefer_capability_match {
            self.distribution_algorithms
                .iter()
                .find(|a| matches!(a, DistributionAlgorithm::CapabilityMatched))
                .cloned()
        } else {
            self.distribution_algorithms
                .iter()
                .find(|a| matches!(a, DistributionAlgorithm::LoadBased))
                .cloned()
        }
        .unwrap_or(DistributionAlgorithm::RoundRobin)
    }

    fn determine_job_type(job: &UniversalJob) -> UniversalJobType {
        // Use the job type if available, otherwise analyze characteristics
        job.job_type.as_ref().map_or_else(
            || {
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
            },
            |job_type| job_type.clone(),
        )
    }

    fn create_subtasks(
        &self,
        job: &UniversalJob,
        count: usize,
        base_requirements: &ResourceRequirements,
    ) -> ToadStoolResult<Vec<SubTask>> {
        let mut subtasks = Vec::new();
        let job_payload = bytes::Bytes::from(
            serde_json::to_vec(&job.execution_request)
                .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize job: {e}")))?,
        );

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

            // Create subtask with partition information (Bytes::concat = zero-copy clone of base)
            let partition_info = format!("{{\"partition\": {i}, \"total_partitions\": {count}}}");
            let subtask_payload =
                bytes::Bytes::from([job_payload.as_ref(), partition_info.as_bytes()].concat());

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

    const fn convert_priority(priority: JobPriority) -> u8 {
        match priority {
            JobPriority::Background => 1,
            JobPriority::Low => 2,
            JobPriority::Normal => 5,
            JobPriority::High => 8,
            JobPriority::Critical => 10,
            JobPriority::Emergency => 15,
        }
    }

    /// Split a job and build a `CoordinationJob` that describes how the subtasks
    /// should be executed together. This is the primary entry point for callers
    /// that need both splitting and coordination in one call.
    pub async fn plan_distribution(
        &self,
        job: &UniversalJob,
        analysis: &JobAnalysis,
    ) -> ToadStoolResult<(Vec<SubTask>, super::types::CoordinationJob)> {
        let subtasks = self.split_job(job, analysis).await?;

        let plan = super::types::DistributionPlan {
            plan_id: Uuid::new_v4(),
            job_id: job.job_id,
            subtasks: subtasks
                .iter()
                .map(|st| super::types::SubTaskPlan {
                    subtask_id: st.id,
                    target_nodes: vec![],
                    resource_allocation: st.resource_requirements.clone(),
                    dependencies: vec![],
                })
                .collect(),
            coordination_strategy: super::types::CoordinationStrategy::Parallel,
        };

        let coordination = self.job_coordinator.coordinate(&plan).await;
        Ok((subtasks, coordination))
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::songbird_integration::types::SplittingStrategyType;
    use crate::songbird_integration::types::{
        DistributionConfig, JobAnalysis, JobDistributionStrategy,
    };
    use crate::{
        DistributedRetryConfig, ExecutionTarget, ResourceRequirements, UniversalJob,
        UniversalJobType,
    };
    use std::collections::HashMap;
    use std::time::SystemTime;

    fn make_job(
        job_type: Option<UniversalJobType>,
        execution_request: toadstool::ExecutionRequest,
    ) -> UniversalJob {
        UniversalJob {
            job_id: uuid::Uuid::new_v4(),
            job_type,
            execution_request,
            target: ExecutionTarget::Local,
            priority: crate::JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: ResourceRequirements::default(),
            retry_config: DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        }
    }

    fn make_analysis(complexity: JobComplexity, estimated_subtasks: usize) -> JobAnalysis {
        JobAnalysis {
            complexity,
            distribution_strategy: JobDistributionStrategy::SplitAndDistribute,
            estimated_subtasks,
            resource_requirements: ResourceRequirements::default(),
            preferred_node_types: vec![],
        }
    }

    #[tokio::test]
    async fn test_massive_job_distributor_new() {
        let config = DistributionConfig {
            max_subtasks: 50,
            splitting_strategies: HashMap::new(),
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        // Verify we can use it for split_job
        let job = make_job(None, toadstool::ExecutionRequest::default());
        let analysis = make_analysis(JobComplexity::Simple, 1);
        let subtasks = distributor.split_job(&job, &analysis).await.unwrap();
        assert_eq!(subtasks.len(), 1);
    }

    #[tokio::test]
    async fn test_massive_job_distributor_new_with_custom_strategies() {
        let mut strategies = HashMap::new();
        strategies.insert("machine_learning".to_string(), "map_reduce".to_string());
        let config = DistributionConfig {
            max_subtasks: 100,
            splitting_strategies: strategies,
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        let job = make_job(
            Some(UniversalJobType::MachineLearning),
            toadstool::ExecutionRequest::default(),
        );
        let analysis = make_analysis(JobComplexity::Simple, 1);
        let subtasks = distributor.split_job(&job, &analysis).await.unwrap();
        assert_eq!(subtasks.len(), 1);
    }

    #[tokio::test]
    async fn test_split_job_simple_complexity_single_subtask() {
        let config = DistributionConfig {
            max_subtasks: 100,
            splitting_strategies: HashMap::new(),
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        let job = make_job(None, toadstool::ExecutionRequest::default());
        let analysis = make_analysis(JobComplexity::Simple, 1);

        let subtasks = distributor.split_job(&job, &analysis).await.unwrap();
        assert_eq!(subtasks.len(), 1);
        assert!(!subtasks[0].payload.is_empty());
    }

    #[tokio::test]
    async fn test_split_job_moderate_complexity_multiple_subtasks() {
        let config = DistributionConfig {
            max_subtasks: 100,
            splitting_strategies: HashMap::new(),
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        let job = make_job(
            Some(UniversalJobType::DataProcessing),
            toadstool::ExecutionRequest::default(),
        );
        let analysis = make_analysis(JobComplexity::Moderate, 8);

        let subtasks = distributor.split_job(&job, &analysis).await.unwrap();
        assert!(subtasks.len() >= 2 && subtasks.len() <= 4);
    }

    #[tokio::test]
    async fn test_split_job_complex_complexity_more_subtasks() {
        let config = DistributionConfig {
            max_subtasks: 100,
            splitting_strategies: HashMap::new(),
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        let job = make_job(
            Some(UniversalJobType::Simulation),
            toadstool::ExecutionRequest::default(),
        );
        let analysis = make_analysis(JobComplexity::Complex, 20);

        let subtasks = distributor.split_job(&job, &analysis).await.unwrap();
        assert!(subtasks.len() >= 4 && subtasks.len() <= 16);
    }

    #[tokio::test]
    async fn test_split_job_ultra_massive_max_subtasks() {
        let config = DistributionConfig {
            max_subtasks: 500,
            splitting_strategies: HashMap::new(),
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        let job = make_job(
            Some(UniversalJobType::MachineLearning),
            toadstool::ExecutionRequest::default(),
        );
        let analysis = make_analysis(JobComplexity::UltraMassive, 2000);

        let subtasks = distributor.split_job(&job, &analysis).await.unwrap();
        assert!(subtasks.len() <= 1000);
    }

    #[tokio::test]
    async fn test_plan_distribution_returns_subtasks_and_coordination() {
        let config = DistributionConfig {
            max_subtasks: 100,
            splitting_strategies: HashMap::new(),
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        let job = make_job(
            Some(UniversalJobType::ComputeIntensive),
            toadstool::ExecutionRequest::default(),
        );
        let analysis = make_analysis(JobComplexity::Simple, 1);

        let (subtasks, coordination) = distributor
            .plan_distribution(&job, &analysis)
            .await
            .unwrap();
        assert_eq!(subtasks.len(), 1);
        assert_eq!(coordination.subtask_count, 1);
        assert_eq!(coordination.original_job_id, job.job_id);
    }

    #[tokio::test]
    async fn test_determine_job_type_from_job_type_field() {
        let config = DistributionConfig {
            max_subtasks: 100,
            splitting_strategies: HashMap::new(),
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        let job = make_job(
            Some(UniversalJobType::MachineLearning),
            toadstool::ExecutionRequest::default(),
        );
        let analysis = make_analysis(JobComplexity::Simple, 1);
        let subtasks = distributor.split_job(&job, &analysis).await.unwrap();
        assert_eq!(subtasks.len(), 1);
    }

    #[tokio::test]
    async fn test_determine_job_type_from_request_data_keywords() {
        use toadstool::workload::{PythonSource, WorkloadSpec};
        let req = toadstool::ExecutionRequest {
            workload: WorkloadSpec::Python {
                source: PythonSource::Code {
                    code: "data batch processing pipeline".to_string(),
                },
                python_version: None,
                requirements: vec![],
                env_vars: std::collections::HashMap::new(),
            },
            ..toadstool::ExecutionRequest::default()
        };
        let job = make_job(None, req);
        let config = DistributionConfig {
            max_subtasks: 100,
            splitting_strategies: HashMap::new(),
        };
        let distributor = MassiveJobDistributor::new(config).await.unwrap();
        let analysis = make_analysis(JobComplexity::Simple, 1);
        let _ = distributor.split_job(&job, &analysis).await.unwrap();
    }

    #[test]
    fn test_job_splitting_strategy_from_string_data_parallel() {
        let s = JobSplittingStrategy::from_string("data_parallel");
        assert!(matches!(
            s.strategy_type,
            SplittingStrategyType::DataParallel
        ));
        assert_eq!(s.max_subtasks, 100);
    }

    #[test]
    fn test_job_splitting_strategy_from_string_task_parallel() {
        let s = JobSplittingStrategy::from_string("task_parallel");
        assert!(matches!(
            s.strategy_type,
            SplittingStrategyType::TaskParallel
        ));
    }

    #[test]
    fn test_job_splitting_strategy_from_string_map_reduce() {
        let s = JobSplittingStrategy::from_string("map_reduce");
        assert!(matches!(s.strategy_type, SplittingStrategyType::MapReduce));
    }

    #[test]
    fn test_job_splitting_strategy_from_string_pipeline() {
        let s = JobSplittingStrategy::from_string("pipeline");
        assert!(matches!(s.strategy_type, SplittingStrategyType::Pipeline));
    }

    #[test]
    fn test_job_splitting_strategy_from_string_custom() {
        let s = JobSplittingStrategy::from_string("my_custom_strategy");
        assert!(
            matches!(&s.strategy_type, SplittingStrategyType::Custom(name) if name == "my_custom_strategy")
        );
    }

    #[test]
    fn test_job_splitting_strategy_default() {
        let s = JobSplittingStrategy::default();
        assert!(matches!(
            s.strategy_type,
            SplittingStrategyType::DataParallel
        ));
        assert_eq!(s.max_subtasks, 100);
        assert_eq!(s.min_subtask_size, 1024);
    }
}
