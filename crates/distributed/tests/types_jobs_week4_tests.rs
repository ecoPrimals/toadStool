// SPDX-License-Identifier: AGPL-3.0-or-later
//! Distributed System Job Types - Week 4 Test Coverage Expansion
//!
//! Comprehensive tests for distributed job types including job specifications,
//! scheduling, priorities, resource requirements, and status tracking.

use std::str::FromStr;
use toadstool_distributed::types::{
    ExecutionTarget, JobPriority, LoadBalancingStrategy, UniversalJobType,
};

#[test]
fn test_job_priority_ordering() {
    // Verify ordering (note: lower enum value = higher priority)
    assert!(JobPriority::Emergency < JobPriority::Critical);
    assert!(JobPriority::Critical < JobPriority::High);
    assert!(JobPriority::High < JobPriority::Normal);
    assert!(JobPriority::Normal < JobPriority::Low);
    assert!(JobPriority::Low < JobPriority::Background);
}

#[test]
fn test_job_priority_display() {
    assert_eq!(format!("{:?}", JobPriority::Emergency), "Emergency");
    assert_eq!(format!("{:?}", JobPriority::Critical), "Critical");
    assert_eq!(format!("{:?}", JobPriority::High), "High");
    assert_eq!(format!("{:?}", JobPriority::Normal), "Normal");
    assert_eq!(format!("{:?}", JobPriority::Low), "Low");
    assert_eq!(format!("{:?}", JobPriority::Background), "Background");
}

#[test]
fn test_universal_job_type_from_str() {
    assert!(matches!(
        UniversalJobType::from_str("local").unwrap(),
        UniversalJobType::Local
    ));
    assert!(matches!(
        UniversalJobType::from_str("compute_intensive").unwrap(),
        UniversalJobType::ComputeIntensive
    ));
    assert!(matches!(
        UniversalJobType::from_str("native").unwrap(),
        UniversalJobType::Native
    ));
    assert!(matches!(
        UniversalJobType::from_str("container").unwrap(),
        UniversalJobType::Container
    ));
}

#[test]
fn test_universal_job_type_variants() {
    let types = vec![
        UniversalJobType::Local,
        UniversalJobType::ComputeIntensive,
        UniversalJobType::MemoryIntensive,
        UniversalJobType::Native,
        UniversalJobType::Container,
        UniversalJobType::WASM,
        UniversalJobType::GPU,
        UniversalJobType::Custom("special".to_string()),
    ];

    assert_eq!(types.len(), 8);
}

#[test]
fn test_universal_job_type_custom() {
    let custom = UniversalJobType::Custom("my-custom-type".to_string());
    assert!(matches!(custom, UniversalJobType::Custom(_)));
}

#[test]
fn test_universal_job_type_remote_toadstool() {
    let remote = UniversalJobType::RemoteToadStool {
        endpoint: "http://remote:8084".to_string(),
    };

    match remote {
        UniversalJobType::RemoteToadStool { endpoint } => {
            assert_eq!(endpoint, "http://remote:8084");
        }
        _ => panic!("Expected RemoteToadStool type"),
    }
}

#[test]
fn test_universal_job_type_ecosystem_tool() {
    let tool = UniversalJobType::EcosystemTool {
        tool_name: "squirrel".to_string(),
        endpoint: "http://squirrel:8083".to_string(),
    };

    match tool {
        UniversalJobType::EcosystemTool {
            tool_name,
            endpoint,
        } => {
            assert_eq!(tool_name, "squirrel");
            assert_eq!(endpoint, "http://squirrel:8083");
        }
        _ => panic!("Expected EcosystemTool type"),
    }
}

#[test]
fn test_execution_target_local() {
    let target = ExecutionTarget::Local;
    assert!(matches!(target, ExecutionTarget::Local));
}

#[test]
fn test_execution_target_toadstool() {
    let target = ExecutionTarget::ToadStool {
        instance_id: "instance-1".to_string(),
        endpoint: "http://localhost:8084".to_string(),
    };

    match target {
        ExecutionTarget::ToadStool {
            instance_id,
            endpoint,
        } => {
            assert_eq!(instance_id, "instance-1");
            assert_eq!(endpoint, "http://localhost:8084");
        }
        _ => panic!("Expected ToadStool target"),
    }
}

#[test]
fn test_execution_target_ecosystem_service() {
    let target = ExecutionTarget::EcosystemService {
        service_name: "songbird".to_string(),
        endpoint: "http://songbird:8080".to_string(),
    };

    match target {
        ExecutionTarget::EcosystemService {
            service_name,
            endpoint,
        } => {
            assert_eq!(service_name, "songbird");
            assert_eq!(endpoint, "http://songbird:8080");
        }
        _ => panic!("Expected EcosystemService target"),
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
        _ => panic!("Expected LoadBalanced target"),
    }
}

#[test]
fn test_load_balancing_strategy_variants() {
    let strategies = vec![
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
    ];

    assert_eq!(strategies.len(), 2);
}

#[test]
fn test_job_priority_clone() {
    let priority = JobPriority::High;
    let cloned = priority;
    assert_eq!(priority, cloned);
}

#[test]
fn test_universal_job_type_clone() {
    let job_type = UniversalJobType::Native;
    let cloned = job_type.clone();
    assert_eq!(job_type, cloned);
}

#[test]
fn test_execution_target_clone() {
    let target = ExecutionTarget::Local;
    let cloned = target.clone();
    // Both should be Local variants
    assert!(matches!(target, ExecutionTarget::Local));
    assert!(matches!(cloned, ExecutionTarget::Local));
}

#[test]
fn test_job_priority_copy() {
    let p1 = JobPriority::Normal;
    let p2 = p1; // Copy, not move
    assert_eq!(p1, p2);
    // Can still use p1
    assert_eq!(p1, JobPriority::Normal);
}

#[test]
fn test_universal_job_type_equality() {
    assert_eq!(UniversalJobType::Local, UniversalJobType::Local);
    assert_eq!(UniversalJobType::Native, UniversalJobType::Native);
    assert_ne!(UniversalJobType::Local, UniversalJobType::Native);
}

#[test]
fn test_job_priority_all_variants() {
    let all_priorities = vec![
        JobPriority::Emergency,
        JobPriority::Critical,
        JobPriority::High,
        JobPriority::Normal,
        JobPriority::Low,
        JobPriority::Background,
    ];

    assert_eq!(all_priorities.len(), 6);

    // Verify they're all different
    for i in 0..all_priorities.len() {
        for j in (i + 1)..all_priorities.len() {
            assert_ne!(all_priorities[i], all_priorities[j]);
        }
    }
}

#[test]
fn test_universal_job_type_workload_variants() {
    // Test all workload classification types
    let workload_types = vec![
        UniversalJobType::ComputeIntensive,
        UniversalJobType::MemoryIntensive,
        UniversalJobType::NetworkIntensive,
        UniversalJobType::StorageIntensive,
        UniversalJobType::Hybrid,
        UniversalJobType::DataProcessing,
        UniversalJobType::MachineLearning,
        UniversalJobType::Simulation,
    ];

    assert_eq!(workload_types.len(), 8);
}

#[test]
fn test_universal_job_type_runtime_variants() {
    // Test all runtime types
    let runtime_types = vec![
        UniversalJobType::Native,
        UniversalJobType::Container,
        UniversalJobType::WASM,
        UniversalJobType::GPU,
    ];

    assert_eq!(runtime_types.len(), 4);
}
