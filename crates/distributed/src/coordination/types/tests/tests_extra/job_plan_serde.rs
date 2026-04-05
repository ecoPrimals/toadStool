// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::make_test_job;
use crate::ResourceRequirements;
use crate::coordination::types::*;
use std::time::SystemTime;
use toadstool_common::auth::AuthType;
use uuid::Uuid;

#[test]
fn test_job_result_empty_output_serde() {
    let result = JobResult {
        job_id: Uuid::new_v4(),
        status: "ok".to_string(),
        output: vec![],
        metrics: ExecutionMetrics {
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            cpu_usage: 0.0,
            memory_usage: 0,
            network_io: 0,
            disk_io: 0,
        },
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: JobResult = serde_json::from_str(&json).unwrap();
    assert!(parsed.output.is_empty());
}

#[test]
fn test_complexity_level_all_variants_serde() {
    for level in [
        ComplexityLevel::Low,
        ComplexityLevel::Medium,
        ComplexityLevel::High,
        ComplexityLevel::Extreme,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let _: ComplexityLevel = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_authentication_config_all_auth_types() {
    for auth_type in [
        AuthType::None,
        AuthType::ApiKey,
        AuthType::Bearer,
        AuthType::Basic,
        AuthType::OAuth2,
    ] {
        let config = AuthenticationConfig {
            auth_type: auth_type.clone(),
            api_key: None,
            token: None,
            username: None,
            password: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AuthenticationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            format!("{:?}", parsed.auth_type),
            format!("{:?}", auth_type)
        );
    }
}

#[test]
fn test_sub_task_plan_empty_dependencies() {
    let plan = SubTaskPlan {
        subtask_id: Uuid::new_v4(),
        target_nodes: vec![],
        resource_allocation: ResourceRequirements::default(),
        dependencies: vec![],
    };
    let json = serde_json::to_string(&plan).unwrap();
    let parsed: SubTaskPlan = serde_json::from_str(&json).unwrap();
    assert!(parsed.dependencies.is_empty());
}

#[test]
fn test_coordination_job_request_full_roundtrip() {
    let mut req = ResourceRequirements::default();
    req.cpu.min_cores = 2.0;
    let job_req = CoordinationJobRequest {
        job_id: Uuid::new_v4(),
        job_payload: bytes::Bytes::from(vec![1, 2, 3, 4, 5]),
        target_nodes: vec!["n1".to_string(), "n2".to_string()],
        resource_requirements: req,
        priority: 10,
        constraints: vec!["gpu".to_string(), "fast".to_string()],
    };
    let json = serde_json::to_string(&job_req).unwrap();
    let parsed: CoordinationJobRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.target_nodes.len(), 2);
    assert_eq!(parsed.constraints.len(), 2);
}

#[test]
fn test_distribution_plan_with_subtasks_serde() {
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
                dependencies: vec![Uuid::new_v4()],
            },
        ],
        coordination_strategy: CoordinationStrategy::MapReduce,
    };
    let json = serde_json::to_string(&plan).unwrap();
    let parsed: DistributionPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.subtasks.len(), 2);
}

#[test]
fn test_coordination_job_request_empty_payload_serde() {
    let req = CoordinationJobRequest {
        job_id: Uuid::new_v4(),
        job_payload: bytes::Bytes::new(),
        target_nodes: vec![],
        resource_requirements: ResourceRequirements::default(),
        priority: 0,
        constraints: vec![],
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: CoordinationJobRequest = serde_json::from_str(&json).unwrap();
    assert!(parsed.job_payload.is_empty());
}

#[test]
fn test_job_analysis_empty_preferred_node_types_serde() {
    let analysis = JobAnalysis {
        complexity: JobComplexity::Simple,
        distribution_strategy: JobDistributionStrategy::LocalOnly,
        estimated_subtasks: 1,
        resource_requirements: ResourceRequirements::default(),
        preferred_node_types: vec![],
    };
    let json = serde_json::to_string(&analysis).unwrap();
    let parsed: JobAnalysis = serde_json::from_str(&json).unwrap();
    assert!(parsed.preferred_node_types.is_empty());
}

#[test]
fn test_sub_task_handle_empty_target_nodes_serde() {
    let handle = SubTaskHandle {
        subtask_id: Uuid::new_v4(),
        coordination_job_id: Uuid::new_v4(),
        target_nodes: vec![],
        submitted_at: SystemTime::now(),
        status: SubTaskStatus::Failed,
    };
    let json = serde_json::to_string(&handle).unwrap();
    let parsed: SubTaskHandle = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed.status, SubTaskStatus::Failed));
}

#[test]
fn test_coordination_job_wait_for_any_serde() {
    let job = CoordinationJob {
        job_id: Uuid::new_v4(),
        original_job_id: Uuid::new_v4(),
        subtask_count: 3,
        completion_strategy: CompletionStrategy::WaitForAny,
    };
    let json = serde_json::to_string(&job).unwrap();
    let parsed: CoordinationJob = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        parsed.completion_strategy,
        CompletionStrategy::WaitForAny
    ));
}

#[test]
fn test_distribution_plan_empty_subtasks_serde() {
    let plan = DistributionPlan {
        plan_id: Uuid::new_v4(),
        job_id: Uuid::new_v4(),
        subtasks: vec![],
        coordination_strategy: CoordinationStrategy::Sequential,
    };
    let json = serde_json::to_string(&plan).unwrap();
    let parsed: DistributionPlan = serde_json::from_str(&json).unwrap();
    assert!(parsed.subtasks.is_empty());
}

#[test]
fn test_authentication_config_basic_serde() {
    let config = AuthenticationConfig {
        auth_type: AuthType::Basic,
        api_key: None,
        token: None,
        username: Some("user".to_string()),
        password: Some("pass".to_string()),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: AuthenticationConfig = serde_json::from_str(&json).unwrap();
    assert!(parsed.username.is_some());
}

#[test]
fn test_capacity_info_insufficient_memory() {
    let mut req = ResourceRequirements::default();
    req.memory.min_bytes = 32 * 1024 * 1024 * 1024;
    let info = CapacityInfo {
        cpu_cores: 8.0,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024 * 1024,
    };
    let job = make_test_job(req);
    assert!(!info.can_handle_job(&job));
}

#[test]
fn test_capacity_info_insufficient_storage() {
    let mut req = ResourceRequirements::default();
    req.storage.min_bytes = 200 * 1024 * 1024 * 1024;
    let info = CapacityInfo {
        cpu_cores: 8.0,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024 * 1024,
    };
    let job = make_test_job(req);
    assert!(!info.can_handle_job(&job));
}
