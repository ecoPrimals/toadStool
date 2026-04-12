// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::types::{
    DistributedRetryConfig, ExecutionTarget, JobPriority, ResourceRequirements,
};
use std::time::SystemTime;
use toadstool::ExecutionRequest;
use uuid::Uuid;

fn make_test_job(job_type: Option<UniversalJobType>) -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type,
        execution_request: ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    }
}

#[tokio::test]
async fn test_hybrid_cloud_scheduler_new_cost_optimized() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
        .await
        .unwrap();
    let _ = scheduler;
}

#[tokio::test]
async fn test_hybrid_cloud_scheduler_new_performance_optimized() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::PerformanceOptimized)
        .await
        .unwrap();
    let _ = scheduler;
}

#[tokio::test]
async fn test_hybrid_cloud_scheduler_new_compliance_first() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::ComplianceFirst)
        .await
        .unwrap();
    let _ = scheduler;
}

#[tokio::test]
async fn test_hybrid_cloud_scheduler_new_balanced() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::Balanced {
        cost_weight: 0.5,
        performance_weight: 0.3,
        compliance_weight: 0.2,
    })
    .await
    .unwrap();
    let _ = scheduler;
}

#[tokio::test]
async fn test_hybrid_cloud_scheduler_new_geographic_affinity() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::GeographicAffinity {
        preferred_regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
    })
    .await
    .unwrap();
    let _ = scheduler;
}

#[tokio::test]
async fn test_hybrid_cloud_scheduler_new_latency_sensitive() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::LatencySensitive {
        max_latency_ms: 50,
        target_regions: vec!["us-east-1".to_string()],
    })
    .await
    .unwrap();
    let _ = scheduler;
}

#[tokio::test]
async fn test_hybrid_cloud_scheduler_new_sustainability_focused() {
    let scheduler =
        HybridCloudScheduler::new(HybridSchedulingStrategy::SustainabilityFocused {
            renewable_energy_preference: 0.8,
        })
        .await
        .unwrap();
    let _ = scheduler;
}

#[tokio::test]
async fn test_get_performance_estimates_compute_intensive() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
        .await
        .unwrap();
    let job = make_test_job(Some(UniversalJobType::ComputeIntensive));
    let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
    assert!(estimates.contains_key("aws"));
    assert!(estimates.contains_key("azure"));
    assert!(estimates.contains_key("gcp"));
    assert_eq!(estimates.get("aws"), Some(&100.0));
}

#[tokio::test]
async fn test_get_performance_estimates_memory_intensive() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
        .await
        .unwrap();
    let job = make_test_job(Some(UniversalJobType::MemoryIntensive));
    let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
    assert_eq!(estimates.get("aws"), Some(&80.0));
}

#[tokio::test]
async fn test_get_performance_estimates_network_intensive() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
        .await
        .unwrap();
    let job = make_test_job(Some(UniversalJobType::NetworkIntensive));
    let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
    assert_eq!(estimates.get("aws"), Some(&60.0));
}

#[tokio::test]
async fn test_get_performance_estimates_storage_intensive() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
        .await
        .unwrap();
    let job = make_test_job(Some(UniversalJobType::StorageIntensive));
    let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
    assert_eq!(estimates.get("aws"), Some(&70.0));
}

#[tokio::test]
async fn test_get_performance_estimates_default_job_type() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
        .await
        .unwrap();
    let job = make_test_job(None);
    let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
    assert_eq!(estimates.get("aws"), Some(&50.0));
}

#[tokio::test]
async fn test_select_providers_returns_all_when_multiple_registered() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
        .await
        .unwrap();
    let job = make_test_job(Some(UniversalJobType::ComputeIntensive));
    let providers = scheduler
        .select_providers(&job, &["aws".to_string(), "gcp".to_string()])
        .await
        .unwrap();
    assert_eq!(providers, vec!["aws".to_string(), "gcp".to_string()]);
}

#[tokio::test]
async fn test_select_providers_empty_when_none_registered() {
    let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
        .await
        .unwrap();
    let job = make_test_job(Some(UniversalJobType::ComputeIntensive));
    let providers = scheduler.select_providers(&job, &[]).await.unwrap();
    assert!(providers.is_empty());
}

#[tokio::test]
async fn test_cloud_cost_tracker_new() {
    let tracker = CloudCostTracker::new();
    let _ = tracker;
}

#[tokio::test]
async fn test_cloud_performance_tracker_new() {
    let tracker = CloudPerformanceTracker::new();
    let _ = tracker;
}
