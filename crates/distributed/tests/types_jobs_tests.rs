//! Comprehensive tests for distributed job types

use chrono::Utc;
use std::collections::HashMap;
use std::str::FromStr;
use toadstool::{ExecutableSource, ExecutionRequest, PythonSource, WorkloadSpec};
use toadstool_distributed::types::DistributedRetryConfig;
use toadstool_distributed::*;
use uuid::Uuid;

// ============================================================================
// UniversalJobType Tests
// ============================================================================

#[test]
fn test_job_type_local() {
    let job_type = UniversalJobType::Local;
    assert!(matches!(job_type, UniversalJobType::Local));
}

#[test]
fn test_job_type_remote_toadstool() {
    let job_type = UniversalJobType::RemoteToadStool {
        endpoint: "http://remote:8080".to_string(),
    };

    match job_type {
        UniversalJobType::RemoteToadStool { endpoint } => {
            assert!(endpoint.starts_with("http://"));
        }
        _ => panic!("Expected RemoteToadStool variant"),
    }
}

#[test]
fn test_job_type_ecosystem_tool() {
    let job_type = UniversalJobType::EcosystemTool {
        tool_name: "nestgate".to_string(),
        endpoint: "http://nestgate:8082".to_string(),
    };

    match job_type {
        UniversalJobType::EcosystemTool {
            tool_name,
            endpoint,
        } => {
            assert_eq!(tool_name, "nestgate");
            assert!(endpoint.contains("8082"));
        }
        _ => panic!("Expected EcosystemTool variant"),
    }
}

#[test]
fn test_job_type_compute_intensive() {
    let job_type = UniversalJobType::ComputeIntensive;
    assert!(matches!(job_type, UniversalJobType::ComputeIntensive));
}

#[test]
fn test_job_type_memory_intensive() {
    let job_type = UniversalJobType::MemoryIntensive;
    assert!(matches!(job_type, UniversalJobType::MemoryIntensive));
}

#[test]
fn test_job_type_network_intensive() {
    let job_type = UniversalJobType::NetworkIntensive;
    assert!(matches!(job_type, UniversalJobType::NetworkIntensive));
}

#[test]
fn test_job_type_storage_intensive() {
    let job_type = UniversalJobType::StorageIntensive;
    assert!(matches!(job_type, UniversalJobType::StorageIntensive));
}

#[test]
fn test_job_type_hybrid() {
    let job_type = UniversalJobType::Hybrid;
    assert!(matches!(job_type, UniversalJobType::Hybrid));
}

#[test]
fn test_job_type_data_processing() {
    let job_type = UniversalJobType::DataProcessing;
    assert!(matches!(job_type, UniversalJobType::DataProcessing));
}

#[test]
fn test_job_type_machine_learning() {
    let job_type = UniversalJobType::MachineLearning;
    assert!(matches!(job_type, UniversalJobType::MachineLearning));
}

#[test]
fn test_job_type_simulation() {
    let job_type = UniversalJobType::Simulation;
    assert!(matches!(job_type, UniversalJobType::Simulation));
}

#[test]
fn test_job_type_native() {
    let job_type = UniversalJobType::Native;
    assert!(matches!(job_type, UniversalJobType::Native));
}

#[test]
fn test_job_type_container() {
    let job_type = UniversalJobType::Container;
    assert!(matches!(job_type, UniversalJobType::Container));
}

#[test]
fn test_job_type_wasm() {
    let job_type = UniversalJobType::WASM;
    assert!(matches!(job_type, UniversalJobType::WASM));
}

#[test]
fn test_job_type_gpu() {
    let job_type = UniversalJobType::GPU;
    assert!(matches!(job_type, UniversalJobType::GPU));
}

#[test]
fn test_job_type_custom() {
    let job_type = UniversalJobType::Custom("quantum".to_string());

    match job_type {
        UniversalJobType::Custom(name) => assert_eq!(name, "quantum"),
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_job_type_from_str_local() {
    let job_type = UniversalJobType::from_str("local").unwrap();
    assert!(matches!(job_type, UniversalJobType::Local));
}

#[test]
fn test_job_type_from_str_compute() {
    let job_type = UniversalJobType::from_str("compute_intensive").unwrap();
    assert!(matches!(job_type, UniversalJobType::ComputeIntensive));
}

#[test]
fn test_job_type_from_str_memory() {
    let job_type = UniversalJobType::from_str("memory_intensive").unwrap();
    assert!(matches!(job_type, UniversalJobType::MemoryIntensive));
}

#[test]
fn test_job_type_from_str_network() {
    let job_type = UniversalJobType::from_str("network_intensive").unwrap();
    assert!(matches!(job_type, UniversalJobType::NetworkIntensive));
}

#[test]
fn test_job_type_from_str_storage() {
    let job_type = UniversalJobType::from_str("storage_intensive").unwrap();
    assert!(matches!(job_type, UniversalJobType::StorageIntensive));
}

#[test]
fn test_job_type_from_str_hybrid() {
    let job_type = UniversalJobType::from_str("hybrid").unwrap();
    assert!(matches!(job_type, UniversalJobType::Hybrid));
}

#[test]
fn test_job_type_from_str_data_processing() {
    let job_type = UniversalJobType::from_str("data_processing").unwrap();
    assert!(matches!(job_type, UniversalJobType::DataProcessing));
}

#[test]
fn test_job_type_from_str_machine_learning() {
    let job_type = UniversalJobType::from_str("machine_learning").unwrap();
    assert!(matches!(job_type, UniversalJobType::MachineLearning));
}

#[test]
fn test_job_type_from_str_simulation() {
    let job_type = UniversalJobType::from_str("simulation").unwrap();
    assert!(matches!(job_type, UniversalJobType::Simulation));
}

#[test]
fn test_job_type_from_str_native() {
    let job_type = UniversalJobType::from_str("native").unwrap();
    assert!(matches!(job_type, UniversalJobType::Native));
}

#[test]
fn test_job_type_from_str_container() {
    let job_type = UniversalJobType::from_str("container").unwrap();
    assert!(matches!(job_type, UniversalJobType::Container));
}

#[test]
fn test_job_type_from_str_wasm() {
    let job_type = UniversalJobType::from_str("wasm").unwrap();
    assert!(matches!(job_type, UniversalJobType::WASM));
}

#[test]
fn test_job_type_from_str_gpu() {
    let job_type = UniversalJobType::from_str("gpu").unwrap();
    assert!(matches!(job_type, UniversalJobType::GPU));
}

#[test]
fn test_job_type_from_str_custom() {
    let job_type = UniversalJobType::from_str("custom_type").unwrap();

    match job_type {
        UniversalJobType::Custom(name) => assert_eq!(name, "custom_type"),
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_job_type_serialization() {
    let job_type = UniversalJobType::MachineLearning;
    let json = serde_json::to_string(&job_type).unwrap();
    let deserialized: UniversalJobType = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, UniversalJobType::MachineLearning));
}

#[test]
fn test_job_type_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(UniversalJobType::Local);
    set.insert(UniversalJobType::GPU);

    assert_eq!(set.len(), 2);
}

// ============================================================================
// ExecutionTarget Tests
// ============================================================================

#[test]
fn test_execution_target_local() {
    let target = ExecutionTarget::Local;
    assert!(matches!(target, ExecutionTarget::Local));
}

#[test]
fn test_execution_target_toadstool() {
    let target = ExecutionTarget::ToadStool {
        instance_id: "ts-001".to_string(),
        endpoint: "http://toadstool:8080".to_string(),
    };

    match target {
        ExecutionTarget::ToadStool {
            instance_id,
            endpoint,
        } => {
            assert_eq!(instance_id, "ts-001");
            assert!(endpoint.starts_with("http://"));
        }
        _ => panic!("Expected ToadStool variant"),
    }
}

#[test]
fn test_execution_target_ecosystem_service() {
    let target = ExecutionTarget::EcosystemService {
        service_name: "nestgate".to_string(),
        endpoint: "http://nestgate:8082".to_string(),
    };

    match target {
        ExecutionTarget::EcosystemService {
            service_name,
            endpoint,
        } => {
            assert_eq!(service_name, "nestgate");
            assert!(endpoint.contains("nestgate"));
        }
        _ => panic!("Expected EcosystemService variant"),
    }
}

#[test]
fn test_execution_target_best_available() {
    let constraints = ResourceConstraints {
        max_cpu_cores: Some(8.0),
        max_memory_bytes: Some(16 * 1024 * 1024 * 1024),
        required_features: vec![],
        excluded_nodes: vec![],
    };

    let target = ExecutionTarget::BestAvailable { constraints };

    match target {
        ExecutionTarget::BestAvailable { constraints } => {
            assert_eq!(constraints.max_cpu_cores, Some(8.0));
        }
        _ => panic!("Expected BestAvailable variant"),
    }
}

#[test]
fn test_execution_target_load_balanced() {
    let target = ExecutionTarget::LoadBalanced {
        strategy: LoadBalancingStrategy::RoundRobin,
    };

    match target {
        ExecutionTarget::LoadBalanced { strategy } => {
            assert!(matches!(strategy, LoadBalancingStrategy::RoundRobin));
        }
        _ => panic!("Expected LoadBalanced variant"),
    }
}

#[test]
fn test_execution_target_serialization() {
    let target = ExecutionTarget::Local;
    let json = serde_json::to_string(&target).unwrap();
    let deserialized: ExecutionTarget = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, ExecutionTarget::Local));
}

// ============================================================================
// JobPriority Tests
// ============================================================================

#[test]
fn test_job_priority_emergency() {
    let priority = JobPriority::Emergency;
    assert!(matches!(priority, JobPriority::Emergency));
    assert_eq!(priority as u8, 0);
}

#[test]
fn test_job_priority_critical() {
    let priority = JobPriority::Critical;
    assert!(matches!(priority, JobPriority::Critical));
    assert_eq!(priority as u8, 1);
}

#[test]
fn test_job_priority_high() {
    let priority = JobPriority::High;
    assert!(matches!(priority, JobPriority::High));
    assert_eq!(priority as u8, 2);
}

#[test]
fn test_job_priority_normal() {
    let priority = JobPriority::Normal;
    assert!(matches!(priority, JobPriority::Normal));
    assert_eq!(priority as u8, 3);
}

#[test]
fn test_job_priority_low() {
    let priority = JobPriority::Low;
    assert!(matches!(priority, JobPriority::Low));
    assert_eq!(priority as u8, 4);
}

#[test]
fn test_job_priority_background() {
    let priority = JobPriority::Background;
    assert!(matches!(priority, JobPriority::Background));
    assert_eq!(priority as u8, 5);
}

#[test]
fn test_job_priority_ordering() {
    assert!(JobPriority::Emergency < JobPriority::Critical);
    assert!(JobPriority::Critical < JobPriority::High);
    assert!(JobPriority::High < JobPriority::Normal);
    assert!(JobPriority::Normal < JobPriority::Low);
    assert!(JobPriority::Low < JobPriority::Background);
}

#[test]
fn test_job_priority_equality() {
    assert_eq!(JobPriority::Normal, JobPriority::Normal);
    assert_ne!(JobPriority::High, JobPriority::Low);
}

#[test]
fn test_job_priority_serialization() {
    let priority = JobPriority::High;
    let json = serde_json::to_string(&priority).unwrap();
    let deserialized: JobPriority = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, JobPriority::High);
}

// ============================================================================
// LoadBalancingStrategy Tests
// ============================================================================

#[test]
fn test_load_balancing_round_robin() {
    let strategy = LoadBalancingStrategy::RoundRobin;
    assert!(matches!(strategy, LoadBalancingStrategy::RoundRobin));
}

#[test]
fn test_load_balancing_least_connections() {
    let strategy = LoadBalancingStrategy::LeastConnections;
    assert!(matches!(strategy, LoadBalancingStrategy::LeastConnections));
}

#[test]
fn test_load_balancing_weighted_round_robin() {
    let mut weights = HashMap::new();
    weights.insert("node1".to_string(), 2);
    weights.insert("node2".to_string(), 1);

    let strategy = LoadBalancingStrategy::WeightedRoundRobin {
        weights: weights.clone(),
    };

    match strategy {
        LoadBalancingStrategy::WeightedRoundRobin { weights } => {
            assert_eq!(weights.len(), 2);
            assert_eq!(weights.get("node1").unwrap(), &2);
        }
        _ => panic!("Expected WeightedRoundRobin variant"),
    }
}

#[test]
fn test_load_balancing_resource_aware() {
    let strategy = LoadBalancingStrategy::ResourceAware;
    assert!(matches!(strategy, LoadBalancingStrategy::ResourceAware));
}

#[test]
fn test_load_balancing_serialization() {
    let strategy = LoadBalancingStrategy::LatencyBased;
    let json = serde_json::to_string(&strategy).unwrap();
    let deserialized: LoadBalancingStrategy = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, LoadBalancingStrategy::LatencyBased));
}

// ============================================================================
// CompatibilityMode Tests
// ============================================================================

#[test]
fn test_compatibility_mode_native() {
    let mode = CompatibilityMode::Native;
    assert!(matches!(mode, CompatibilityMode::Native));
}

#[test]
fn test_compatibility_mode_container() {
    let mode = CompatibilityMode::Container;
    assert!(matches!(mode, CompatibilityMode::Container));
}

#[test]
fn test_compatibility_mode_emulated() {
    let mode = CompatibilityMode::Emulated;
    assert!(matches!(mode, CompatibilityMode::Emulated));
}

#[test]
fn test_compatibility_mode_hybrid() {
    let mode = CompatibilityMode::Hybrid;
    assert!(matches!(mode, CompatibilityMode::Hybrid));
}

#[test]
fn test_compatibility_mode_linux() {
    let mode = CompatibilityMode::LinuxCompat;
    assert!(matches!(mode, CompatibilityMode::LinuxCompat));
}

#[test]
fn test_compatibility_mode_legacy() {
    let mode = CompatibilityMode::LegacyCompat {
        system_type: "custom-system".to_string(),
    };

    match mode {
        CompatibilityMode::LegacyCompat { system_type } => {
            assert_eq!(system_type, "custom-system")
        }
        _ => panic!("Expected LegacyCompat variant"),
    }
}

#[test]
fn test_compatibility_mode_serialization() {
    let mode = CompatibilityMode::Container;
    let json = serde_json::to_string(&mode).unwrap();
    let deserialized: CompatibilityMode = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, CompatibilityMode::Container));
}

// ============================================================================
// UniversalJob Tests
// ============================================================================

#[test]
fn test_universal_job_creation() {
    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::Local),
        execution_request: ExecutionRequest {
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::Bytes { data: vec![] },
                args: Some(vec!["test".to_string()]),
                working_dir: None,
                env_vars: HashMap::new(),
                user: None,
            },
            ..Default::default()
        },
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: DistributedRetryConfig::default(),
        created_at: Utc::now(),
    };

    assert!(matches!(job.priority, JobPriority::Normal));
    assert!(job.dependencies.is_empty());
}

#[test]
fn test_universal_job_with_dependencies() {
    let dep1 = Uuid::new_v4();
    let dep2 = Uuid::new_v4();

    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::ComputeIntensive),
        execution_request: ExecutionRequest {
            workload: WorkloadSpec::Python {
                source: PythonSource::Code {
                    code: "print('processing data')".to_string(),
                },
                python_version: Some("3.11".to_string()),
                requirements: vec![],
                env_vars: HashMap::new(),
            },
            ..Default::default()
        },
        target: ExecutionTarget::Local,
        priority: JobPriority::High,
        dependencies: vec![dep1, dep2],
        resource_requirements: ResourceRequirements::default(),
        retry_config: DistributedRetryConfig::default(),
        created_at: Utc::now(),
    };

    assert_eq!(job.dependencies.len(), 2);
    assert!(job.dependencies.contains(&dep1));
}

#[test]
fn test_universal_job_serialization() {
    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(UniversalJobType::GPU),
        execution_request: ExecutionRequest {
            workload: WorkloadSpec::Python {
                source: PythonSource::Code {
                    code: "# ML training code".to_string(),
                },
                python_version: None,
                requirements: vec!["torch".to_string()],
                env_vars: HashMap::new(),
            },
            ..Default::default()
        },
        target: ExecutionTarget::Local,
        priority: JobPriority::Critical,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: DistributedRetryConfig::default(),
        created_at: Utc::now(),
    };

    let json = serde_json::to_string(&job).unwrap();
    let deserialized: UniversalJob = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.job_id, job.job_id);
    assert_eq!(deserialized.priority, JobPriority::Critical);
}
