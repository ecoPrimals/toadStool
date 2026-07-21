// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::*;
use crate::coordination::types::{
    CapacityConfig, ConnectionHealth, CoordinationConnection, CoordinationTransport,
    HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
};
use crate::{
    DistributedRetryConfig, ExecutionTarget, ResourceRequirements, UniversalJob, UniversalJobType,
    UniversalScheduler, UniversalSchedulerConfig,
};
use toadstool_common::constants::network::LOCALHOST_IPV4;
use uuid::Uuid;

fn test_connection() -> CoordinationConnection {
    let endpoint = format!("http://{}:{}", LOCALHOST_IPV4, 50051_u16);
    CoordinationConnection {
        endpoints: vec![endpoint.clone()],
        active_endpoint: endpoint,
        auth_token: None,
        health_status: ConnectionHealth::Healthy,
        protocol_config: ProtocolConfig {
            protocol: CoordinationTransport::HTTP,
            http: HttpProtocolConfig {
                timeout_ms: 5000,
                max_retries: 3,
                headers: std::collections::HashMap::new(),
            },
            message_queue: MessageQueueProtocolConfig {
                queue_name: "jobs".to_string(),
                exchange: "toadstool".to_string(),
                routing_key: "compute".to_string(),
            },
        },
    }
}

fn capacity_config() -> CapacityConfig {
    CapacityConfig {
        monitoring_interval: Duration::from_mins(1),
        resource_buffer: 0.1,
    }
}

fn base_job() -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::Local),
        execution_request: toadstool::ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: toadstool::JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    }
}

/// Job with explicit CPU and memory floors; other resource fields stay at defaults.
fn job_with_cpu_memory(cpu_min_cores: f64, memory_min_bytes: u64) -> UniversalJob {
    let mut job = base_job();
    job.resource_requirements.cpu.min_cores = cpu_min_cores;
    job.resource_requirements.memory.min_bytes = memory_min_bytes;
    job
}

async fn test_integration() -> ToadStoolCoordinationIntegration {
    let config = UniversalSchedulerConfig::default();
    let scheduler = Arc::new(UniversalScheduler::new(config).await.unwrap());
    ToadStoolCoordinationIntegration::new(
        "messaging-test".to_string(),
        test_connection(),
        capacity_config(),
        scheduler,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn analyze_job_complexity_simple_below_moderate_cpu_threshold() {
    let integration = test_integration().await;
    let job = job_with_cpu_memory(3.9, 1024 * 1024 * 1024);
    let c = integration.analyze_job_complexity(&job).await.unwrap();
    assert_eq!(c, JobComplexity::Simple);
}

#[tokio::test]
async fn analyze_job_complexity_moderate_at_cpu_threshold() {
    let integration = test_integration().await;
    let job = job_with_cpu_memory(4.0, 1024 * 1024 * 1024);
    let c = integration.analyze_job_complexity(&job).await.unwrap();
    assert_eq!(c, JobComplexity::Moderate);
}

#[tokio::test]
async fn analyze_job_complexity_complex_at_cpu_threshold() {
    let integration = test_integration().await;
    let job = job_with_cpu_memory(8.0, 1024 * 1024 * 1024);
    let c = integration.analyze_job_complexity(&job).await.unwrap();
    assert_eq!(c, JobComplexity::Complex);
}

#[tokio::test]
async fn analyze_job_complexity_ultra_massive_at_cpu_threshold() {
    let integration = test_integration().await;
    let job = job_with_cpu_memory(16.0, 1024 * 1024 * 1024);
    let c = integration.analyze_job_complexity(&job).await.unwrap();
    assert_eq!(c, JobComplexity::UltraMassive);
}

#[tokio::test]
async fn analyze_job_complexity_moderate_via_memory_floor() {
    let integration = test_integration().await;
    let sixteen_gib = 16_u64 * 1024 * 1024 * 1024;
    let job = job_with_cpu_memory(1.0, sixteen_gib);
    let c = integration.analyze_job_complexity(&job).await.unwrap();
    assert_eq!(c, JobComplexity::Moderate);
}

#[tokio::test]
async fn analyze_job_complexity_complex_via_memory_floor() {
    let integration = test_integration().await;
    let thirty_two_gib = 32_u64 * 1024 * 1024 * 1024;
    let job = job_with_cpu_memory(1.0, thirty_two_gib);
    let c = integration.analyze_job_complexity(&job).await.unwrap();
    assert_eq!(c, JobComplexity::Complex);
}

#[tokio::test]
async fn analyze_job_complexity_ultra_massive_via_memory_floor() {
    let integration = test_integration().await;
    let sixty_four_gib = 64_u64 * 1024 * 1024 * 1024;
    let job = job_with_cpu_memory(1.0, sixty_four_gib);
    let c = integration.analyze_job_complexity(&job).await.unwrap();
    assert_eq!(c, JobComplexity::UltraMassive);
}

#[tokio::test]
async fn estimate_subtask_count_simple() {
    let integration = test_integration().await;
    let job = base_job();
    let n = integration
        .estimate_subtask_count(&job, &JobComplexity::Simple)
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn estimate_subtask_count_moderate() {
    let integration = test_integration().await;
    let job = base_job();
    let n = integration
        .estimate_subtask_count(&job, &JobComplexity::Moderate)
        .await
        .unwrap();
    assert_eq!(n, 5);
}

#[tokio::test]
async fn estimate_subtask_count_complex() {
    let integration = test_integration().await;
    let job = base_job();
    let n = integration
        .estimate_subtask_count(&job, &JobComplexity::Complex)
        .await
        .unwrap();
    assert_eq!(n, 25);
}

#[tokio::test]
async fn estimate_subtask_count_ultra_massive() {
    let integration = test_integration().await;
    let job = base_job();
    let n = integration
        .estimate_subtask_count(&job, &JobComplexity::UltraMassive)
        .await
        .unwrap();
    assert_eq!(n, 1000);
}
