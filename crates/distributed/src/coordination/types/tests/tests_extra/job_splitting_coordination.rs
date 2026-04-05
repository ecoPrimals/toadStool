// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::make_test_job;
use crate::ResourceRequirements;
use crate::coordination::types::*;
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::test]
async fn test_job_splitting_strategy_split_job_max_subtasks_one() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::DataParallel,
        max_subtasks: 1,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_job_splitting_strategy_split_job_data_parallel() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::DataParallel,
        max_subtasks: 4,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert!(!result.is_empty());
    assert!(result.len() <= 4);
}

#[tokio::test]
async fn test_job_splitting_strategy_split_job_task_parallel() {
    let mut req = ResourceRequirements::default();
    req.cpu.min_cores = 4.0;
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::TaskParallel,
        max_subtasks: 3,
        min_subtask_size: 1,
    };
    let job = make_test_job(req);
    let result = strategy.split_job(&job).await;
    assert_eq!(result.len(), 3);
}

#[tokio::test]
async fn test_job_splitting_strategy_split_job_map_reduce() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::MapReduce,
        max_subtasks: 2,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert!(!result.is_empty());
}

#[tokio::test]
async fn test_load_estimator_estimate_load() {
    let estimator = LoadEstimator::default();
    let job = make_test_job(ResourceRequirements::default());
    let load = estimator.estimate_load(&job).await;
    assert!(load.cpu_load >= 0.0 && load.cpu_load <= 1.0);
    assert!(load.memory_load >= 0.0 && load.memory_load <= 1.0);
    assert!(load.network_load >= 0.0 && load.network_load <= 1.0);
}

#[tokio::test]
async fn test_job_coordinator_coordinate() {
    let coord = JobCoordinator::default();
    let plan = DistributionPlan {
        plan_id: Uuid::new_v4(),
        job_id: Uuid::new_v4(),
        subtasks: vec![
            SubTaskPlan {
                subtask_id: Uuid::new_v4(),
                target_nodes: vec!["n1".to_string()],
                resource_allocation: ResourceRequirements::default(),
                dependencies: vec![],
            },
            SubTaskPlan {
                subtask_id: Uuid::new_v4(),
                target_nodes: vec!["n2".to_string()],
                resource_allocation: ResourceRequirements::default(),
                dependencies: vec![],
            },
        ],
        coordination_strategy: CoordinationStrategy::Parallel,
    };
    let job = coord.coordinate(&plan).await;
    assert_eq!(job.subtask_count, 2);
}

#[test]
fn test_capacity_info_from_system() {
    let info = CapacityInfo::from_system();
    assert!(info.cpu_cores > 0.0);
    assert!(info.memory_bytes > 0);
    let _ = info.storage_bytes;
}

#[tokio::test]
async fn test_job_receiver_receive_none_when_empty() {
    let (tx, rx) = mpsc::channel::<CoordinationJobMessage>(1);
    drop(tx);
    let mut receiver = JobReceiver { receiver: rx };
    let result = receiver.receive().await;
    assert!(result.is_none());
}

#[test]
fn test_splitting_strategy_type_variants() {
    let _ = SplittingStrategyType::DataParallel;
    let _ = SplittingStrategyType::TaskParallel;
    let _ = SplittingStrategyType::Pipeline;
    let _ = SplittingStrategyType::MapReduce;
    let _ = SplittingStrategyType::Custom("custom".to_string());
}

#[tokio::test]
async fn test_job_splitting_strategy_custom_falls_back_to_task_parallel() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::Custom("custom".to_string()),
        max_subtasks: 2,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_job_splitting_strategy_pipeline_falls_back_to_task_parallel() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::Pipeline,
        max_subtasks: 2,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert_eq!(result.len(), 2);
}
