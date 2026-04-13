// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::coordination::types::SplittingStrategyType;
use crate::coordination::types::{DistributionConfig, JobAnalysis, JobDistributionStrategy};
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
