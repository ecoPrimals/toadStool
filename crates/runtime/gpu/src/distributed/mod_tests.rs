// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::scheduler::SchedulingPolicy;
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn test_distributed_scheduler_creation() {
    let local = Arc::new(UniversalComputeScheduler::new(
        SchedulingPolicy::CapabilityMatch,
    ));
    let scheduler = DistributedGpuScheduler::new(local);

    let towers = scheduler.available_towers().await;
    assert_eq!(towers.len(), 1); // Only local initially
}

#[tokio::test]
async fn test_register_remote_tower() {
    let local = Arc::new(UniversalComputeScheduler::new(
        SchedulingPolicy::CapabilityMatch,
    ));
    let scheduler = DistributedGpuScheduler::new(local);

    // Test fixture: placeholder address for unit test (production uses coordination discovery)
    let endpoint = RemoteTowerEndpoint {
        tower_id: "remote-1".to_string(),
        address: "10.0.0.2:8080".to_string(),
        gpu_capabilities: None,
        last_seen: Instant::now(),
        latency_ms: 5,
    };

    scheduler.register_remote_tower(endpoint).await;

    let towers = scheduler.available_towers().await;
    assert_eq!(towers.len(), 2); // Local + 1 remote
}

#[tokio::test]
async fn test_statistics() {
    let local = Arc::new(UniversalComputeScheduler::new(
        SchedulingPolicy::CapabilityMatch,
    ));
    let scheduler = DistributedGpuScheduler::new(local);

    let stats = scheduler.statistics().await;
    assert_eq!(stats.total_towers, 1);
    assert_eq!(stats.total_jobs, 0);
}

#[tokio::test]
async fn test_execute_distributed_single() {
    use crate::cpu_resource::{CpuComputeResource, UniversalComputeResourceDispatch};
    use crate::universal::{
        ComputeRequirements, OptimizationHints, UniversalKernel, UniversalWorkload,
    };

    let local = Arc::new(UniversalComputeScheduler::new(
        SchedulingPolicy::CapabilityMatch,
    ));
    let cpu = CpuComputeResource::new().expect("CPU resource");
    local
        .register_resource(Arc::new(UniversalComputeResourceDispatch::Cpu(cpu)))
        .await;

    let scheduler = DistributedGpuScheduler::new(local);

    let workload = UniversalWorkload {
        id: "test-workload".to_string(),
        requirements: ComputeRequirements::default(),
        kernel: UniversalKernel::Operation {
            operation: crate::universal::Operation::GeneralCompute,
            parameters: std::collections::HashMap::default(),
        },
        inputs: vec![],
        output_size: 0,
        hints: OptimizationHints::default(),
    };

    let result = scheduler
        .execute_distributed(workload, PartitionStrategy::Single)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_execute_distributed_data_parallel() {
    use crate::cpu_resource::{CpuComputeResource, UniversalComputeResourceDispatch};
    use crate::universal::{
        ComputeRequirements, OptimizationHints, UniversalKernel, UniversalWorkload,
    };

    let local = Arc::new(UniversalComputeScheduler::new(
        SchedulingPolicy::CapabilityMatch,
    ));
    let cpu = CpuComputeResource::new().expect("CPU resource");
    local
        .register_resource(Arc::new(UniversalComputeResourceDispatch::Cpu(cpu)))
        .await;

    let scheduler = DistributedGpuScheduler::new(local);

    let workload = UniversalWorkload {
        id: "test-dp".to_string(),
        requirements: ComputeRequirements::default(),
        kernel: UniversalKernel::Operation {
            operation: crate::universal::Operation::GeneralCompute,
            parameters: std::collections::HashMap::default(),
        },
        inputs: vec![],
        output_size: 0,
        hints: OptimizationHints::default(),
    };

    let result = scheduler
        .execute_distributed(workload, PartitionStrategy::DataParallel { chunk_size: 64 })
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_execute_distributed_pipeline() {
    use crate::cpu_resource::{CpuComputeResource, UniversalComputeResourceDispatch};
    use crate::universal::{
        ComputeRequirements, OptimizationHints, UniversalKernel, UniversalWorkload,
    };

    let local = Arc::new(UniversalComputeScheduler::new(
        SchedulingPolicy::CapabilityMatch,
    ));
    let cpu = CpuComputeResource::new().expect("CPU resource");
    local
        .register_resource(Arc::new(UniversalComputeResourceDispatch::Cpu(cpu)))
        .await;

    let scheduler = DistributedGpuScheduler::new(local);

    let workload = UniversalWorkload {
        id: "test-pipeline".to_string(),
        requirements: ComputeRequirements::default(),
        kernel: UniversalKernel::Operation {
            operation: crate::universal::Operation::GeneralCompute,
            parameters: std::collections::HashMap::default(),
        },
        inputs: vec![],
        output_size: 0,
        hints: OptimizationHints::default(),
    };

    let result = scheduler
        .execute_distributed(
            workload,
            PartitionStrategy::Pipeline {
                stages: vec!["stage1".to_string()],
            },
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_statistics_with_remote_tower() {
    let local = Arc::new(UniversalComputeScheduler::new(
        SchedulingPolicy::CapabilityMatch,
    ));
    let scheduler = DistributedGpuScheduler::new(local);

    let endpoint = RemoteTowerEndpoint {
        tower_id: "remote-1".to_string(),
        address: "10.0.0.2:8080".to_string(),
        gpu_capabilities: None,
        last_seen: Instant::now(),
        latency_ms: 5,
    };
    scheduler.register_remote_tower(endpoint).await;

    let stats = scheduler.statistics().await;
    assert_eq!(stats.total_towers, 2);
    assert_eq!(stats.active_towers, 2);
}
