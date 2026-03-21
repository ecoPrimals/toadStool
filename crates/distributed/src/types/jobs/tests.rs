// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for distributed job types and serde roundtrips.

use std::collections::HashMap;
use std::str::FromStr;
use std::time::SystemTime;

use uuid::Uuid;

use super::*;
use crate::types::resources::{DistributedRetryConfig, ResourceConstraints, ResourceRequirements};

fn make_universal_job(
    job_type: Option<UniversalJobType>,
    target: ExecutionTarget,
    priority: JobPriority,
) -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type,
        execution_request: toadstool::ExecutionRequest::default(),
        target,
        priority,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    }
}

#[test]
fn universal_job_type_from_str_standard_variants() {
    assert!(matches!(
        UniversalJobType::from_str("local").unwrap(),
        UniversalJobType::Local
    ));
    assert!(matches!(
        UniversalJobType::from_str("compute_intensive").unwrap(),
        UniversalJobType::ComputeIntensive
    ));
    assert!(matches!(
        UniversalJobType::from_str("gpu").unwrap(),
        UniversalJobType::GPU
    ));
    assert!(matches!(
        UniversalJobType::from_str("WASM").unwrap(),
        UniversalJobType::WASM
    ));
}

#[test]
fn universal_job_type_from_str_custom() {
    let custom = UniversalJobType::from_str("my_custom_type").unwrap();
    match &custom {
        UniversalJobType::Custom(s) => assert_eq!(s, "my_custom_type"),
        _ => panic!("expected Custom variant"),
    }
}

#[test]
fn resource_requirements_default_values() {
    let req = ResourceRequirements::default();
    assert_eq!(req.cpu.min_cores, 1.0);
    assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
    assert!(req.gpu.is_none());
}

#[test]
fn job_priority_ordering() {
    use std::cmp::Ordering;
    assert!(JobPriority::Emergency < JobPriority::Normal);
    assert!(JobPriority::High < JobPriority::Low);
    assert_eq!(
        JobPriority::Normal.cmp(&JobPriority::Normal),
        Ordering::Equal
    );
}

#[test]
fn universal_job_creation_with_variants() {
    let _job_local = make_universal_job(
        Some(UniversalJobType::Local),
        ExecutionTarget::Local,
        JobPriority::Normal,
    );
    let _job_best_available = make_universal_job(
        Some(UniversalJobType::GPU),
        ExecutionTarget::BestAvailable {
            constraints: ResourceConstraints {
                max_cpu_cores: Some(8.0),
                max_memory_bytes: None,
                required_features: vec![],
                excluded_nodes: vec![],
            },
        },
        JobPriority::High,
    );
}

#[test]
fn execution_target_variants() {
    let _local = ExecutionTarget::Local;
    let _toadstool = ExecutionTarget::ToadStool {
        instance_id: "inst-1".to_string(),
        endpoint: toadstool_common::constants::network::default_http_url(),
    };
    let _best = ExecutionTarget::BestAvailable {
        constraints: ResourceConstraints {
            max_cpu_cores: Some(8.0),
            max_memory_bytes: Some(16 * 1024 * 1024 * 1024),
            required_features: vec!["gpu".to_string()],
            excluded_nodes: vec![],
        },
    };
}

#[test]
fn compatibility_mode_as_str() {
    assert_eq!(CompatibilityMode::Native.as_str(), "native");
    assert_eq!(CompatibilityMode::Container.as_str(), "container");
    assert_eq!(
        CompatibilityMode::LegacyCompat {
            system_type: "old".to_string()
        }
        .as_str(),
        "legacy_compat"
    );
}

#[test]
fn load_balancing_strategy_construction() {
    let _rr = LoadBalancingStrategy::RoundRobin;
    let _lc = LoadBalancingStrategy::LeastConnections;
    let mut weights = HashMap::new();
    weights.insert("a".to_string(), 1);
    let _wrr = LoadBalancingStrategy::WeightedRoundRobin { weights };
}

#[test]
fn universal_job_queue_new_and_default() {
    let queue = UniversalJobQueue::new();
    assert_eq!(queue.total_jobs(), 0);
    let default_queue = UniversalJobQueue::default();
    assert_eq!(default_queue.total_jobs(), 0);
}

#[test]
fn dependency_graph_add_job() {
    let mut graph = DependencyGraph::new();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    graph.add_job(id1, vec![]).expect("add root");
    graph.add_job(id2, vec![id1]).expect("add dependent");
}

#[test]
fn job_metadata_from_job() {
    let job = make_universal_job(
        Some(UniversalJobType::Local),
        ExecutionTarget::Local,
        JobPriority::Normal,
    );
    let meta = JobMetadata::from_job(&job);
    assert_eq!(meta.job_id, job.job_id);
    assert_eq!(meta.priority, job.priority);
}

#[test]
fn resource_requirement_index_add_job() {
    let mut index = ResourceRequirementIndex::new();
    let job = make_universal_job(
        Some(UniversalJobType::Local),
        ExecutionTarget::Local,
        JobPriority::Normal,
    );
    index
        .add_job(job.job_id, job.resource_requirements)
        .expect("add job");
}

#[test]
fn toadstool_hosting_config_construction() {
    let config = ToadStoolHostingConfig {
        enabled: true,
        mode: "standalone".to_string(),
        resource_limits: HashMap::new(),
        security_settings: HashMap::new(),
        resource_allocation: None,
    };
    assert!(config.enabled);
    assert_eq!(config.mode, "standalone");
}

#[test]
fn universal_job_serde_roundtrip() {
    let job = make_universal_job(
        Some(UniversalJobType::Local),
        ExecutionTarget::Local,
        JobPriority::Normal,
    );
    let json = serde_json::to_string(&job).unwrap();
    let parsed: UniversalJob = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.job_id, job.job_id);
}

#[test]
fn execution_target_serde_roundtrip() {
    let targets = [
        ExecutionTarget::Local,
        ExecutionTarget::ToadStool {
            instance_id: "i1".to_string(),
            endpoint: "http://localhost:8080".to_string(),
        },
        ExecutionTarget::EcosystemService {
            service_name: "svc".to_string(),
            endpoint: "http://svc:8080".to_string(),
        },
    ];
    for t in targets {
        let json = serde_json::to_string(&t).unwrap();
        let _: ExecutionTarget = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn load_balancing_strategy_serde_roundtrip() {
    let mut weights = HashMap::new();
    weights.insert("a".to_string(), 2);
    let strategies = [
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::WeightedRoundRobin { weights },
        LoadBalancingStrategy::ResourceAware,
        LoadBalancingStrategy::LatencyBased,
    ];
    for s in strategies {
        let json = serde_json::to_string(&s).unwrap();
        let _: LoadBalancingStrategy = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn compatibility_mode_serde_roundtrip() {
    for mode in [
        CompatibilityMode::Native,
        CompatibilityMode::Container,
        CompatibilityMode::Emulated,
        CompatibilityMode::Hybrid,
        CompatibilityMode::LinuxCompat,
        CompatibilityMode::WindowsCompat,
        CompatibilityMode::MacOSCompat,
        CompatibilityMode::ContainerCompat,
        CompatibilityMode::LegacyCompat {
            system_type: "old".to_string(),
        },
    ] {
        let json = serde_json::to_string(&mode).unwrap();
        let _: CompatibilityMode = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn toadstool_hosting_config_serde_roundtrip() {
    let mut limits = HashMap::new();
    limits.insert("cpu".to_string(), 8);
    let config = ToadStoolHostingConfig {
        enabled: true,
        mode: "standalone".to_string(),
        resource_limits: limits,
        security_settings: HashMap::new(),
        resource_allocation: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ToadStoolHostingConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.mode, config.mode);
}

#[test]
fn universal_job_type_serde_roundtrip() {
    let types = [
        UniversalJobType::Local,
        UniversalJobType::RemoteToadStool {
            endpoint: "http://x:8080".to_string(),
        },
        UniversalJobType::EcosystemTool {
            tool_name: "t".to_string(),
            endpoint: "http://t:8080".to_string(),
        },
        UniversalJobType::ComputeIntensive,
        UniversalJobType::GPU,
        UniversalJobType::Custom("custom".to_string()),
    ];
    for t in types {
        let json = serde_json::to_string(&t).unwrap();
        let _: UniversalJobType = serde_json::from_str(&json).unwrap();
    }
}

#[tokio::test]
async fn universal_job_queue_add_job() {
    let mut queue = UniversalJobQueue::new();
    let job = make_universal_job(
        Some(UniversalJobType::Local),
        ExecutionTarget::Local,
        JobPriority::Normal,
    );
    queue.add_job(job).await.expect("add job");
}

#[test]
fn execution_target_best_available_serde() {
    let target = ExecutionTarget::BestAvailable {
        constraints: ResourceConstraints {
            max_cpu_cores: Some(16.0),
            max_memory_bytes: Some(32 * 1024 * 1024 * 1024),
            required_features: vec!["gpu".to_string()],
            excluded_nodes: vec![],
        },
    };
    let json = serde_json::to_string(&target).unwrap();
    let _: ExecutionTarget = serde_json::from_str(&json).unwrap();
}

#[test]
fn execution_target_load_balanced_serde() {
    let target = ExecutionTarget::LoadBalanced {
        strategy: LoadBalancingStrategy::RoundRobin,
    };
    let json = serde_json::to_string(&target).unwrap();
    let _: ExecutionTarget = serde_json::from_str(&json).unwrap();
}
