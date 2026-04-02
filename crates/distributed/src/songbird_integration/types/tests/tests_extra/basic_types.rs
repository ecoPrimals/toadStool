// SPDX-License-Identifier: AGPL-3.0-only

use super::helpers::sample_songbird_integration_config;
use crate::songbird_integration::types::*;
use uuid::Uuid;

#[test]
fn test_connection_health_all_variants_debug() {
    for h in [
        ConnectionHealth::Healthy,
        ConnectionHealth::Degraded,
        ConnectionHealth::Unhealthy,
        ConnectionHealth::Unknown,
    ] {
        let s = format!("{h:?}");
        assert!(!s.is_empty());
    }
}

#[test]
fn test_complexity_level_all_variants() {
    let _ = ComplexityLevel::Low;
    let _ = ComplexityLevel::Medium;
    let _ = ComplexityLevel::High;
    let _ = ComplexityLevel::Extreme;
}

#[test]
fn test_execution_metrics_constructor() {
    let m = ExecutionMetrics {
        start_time: std::time::SystemTime::now(),
        end_time: std::time::SystemTime::now(),
        cpu_usage: 0.5,
        memory_usage: 1024,
        network_io: 100,
        disk_io: 200,
    };
    assert_eq!(m.memory_usage, 1024);
}

#[test]
fn test_songbird_job_response_success_estimated_completion_none() {
    let resp = SongbirdJobResponse::Success {
        job_id: Uuid::new_v4(),
        status: "done".to_string(),
        message: "OK".to_string(),
        estimated_completion: None,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SongbirdJobResponse = serde_json::from_str(&json).unwrap();
    if let SongbirdJobResponse::Success {
        estimated_completion,
        ..
    } = parsed
    {
        assert!(estimated_completion.is_none());
    } else {
        panic!("expected Success");
    }
}

#[test]
fn test_job_distribution_strategy_hybrid_execution_serde() {
    let s = JobDistributionStrategy::HybridExecution;
    let json = serde_json::to_string(&s).unwrap();
    let _: JobDistributionStrategy = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_songbird_integration_config_constructor() {
    let config = sample_songbird_integration_config();
    assert_eq!(config.receiver_config.max_concurrent_jobs, 4);
}

#[test]
fn test_universal_job_processor_constructor() {
    let p = UniversalJobProcessor::new("proc-1".to_string());
    assert_eq!(p.processor_id, "proc-1");
    assert!(!p.display_name.is_empty());
    assert!(p.max_concurrent_jobs > 0);
}
