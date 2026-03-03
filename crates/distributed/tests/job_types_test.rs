// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for job types and queues
//!
//! Day 3 of Month 1 test expansion - focusing on job management

use toadstool_distributed::types::{
    CompatibilityMode, ExecutionTarget, JobPriority, LoadBalancingStrategy, UniversalJobQueue,
    UniversalJobType,
};

// ============================================================================
// UniversalJobType Tests (10 tests)
// ============================================================================

#[test]
fn test_job_type_local() {
    // Test local job type
    let job_type = UniversalJobType::Local;

    match job_type {
        UniversalJobType::Local => {
            // Success
        }
        _ => panic!("Expected Local job type"),
    }
}

#[test]
fn test_job_type_native() {
    // Test native execution type
    let job_type = UniversalJobType::Native;

    match job_type {
        UniversalJobType::Native => {
            // Success
        }
        _ => panic!("Expected Native job type"),
    }
}

#[test]
fn test_job_type_container() {
    // Test container execution type
    let job_type = UniversalJobType::Container;

    match job_type {
        UniversalJobType::Container => {
            // Success
        }
        _ => panic!("Expected Container job type"),
    }
}

#[test]
fn test_job_type_wasm() {
    // Test WASM execution type
    let job_type = UniversalJobType::WASM;

    match job_type {
        UniversalJobType::WASM => {
            // Success
        }
        _ => panic!("Expected WASM job type"),
    }
}

#[test]
fn test_job_type_simulation() {
    // Test simulation execution type
    let job_type = UniversalJobType::Simulation;

    match job_type {
        UniversalJobType::Simulation => {
            // Success
        }
        _ => panic!("Expected Simulation job type"),
    }
}

#[test]
fn test_job_type_gpu() {
    // Test GPU execution type
    let job_type = UniversalJobType::GPU;

    match job_type {
        UniversalJobType::GPU => {
            // Success
        }
        _ => panic!("Expected GPU job type"),
    }
}

#[test]
fn test_job_type_machine_learning() {
    // Test machine learning job type
    let job_type = UniversalJobType::MachineLearning;

    match job_type {
        UniversalJobType::MachineLearning => {
            // Success
        }
        _ => panic!("Expected MachineLearning job type"),
    }
}

#[test]
fn test_job_type_data_processing() {
    // Test data processing job type
    let job_type = UniversalJobType::DataProcessing;

    match job_type {
        UniversalJobType::DataProcessing => {
            // Success
        }
        _ => panic!("Expected DataProcessing job type"),
    }
}

#[test]
fn test_job_type_custom() {
    // Test custom job type
    let job_type = UniversalJobType::Custom("special-workload".to_string());

    match job_type {
        UniversalJobType::Custom(name) => {
            assert_eq!(name, "special-workload");
        }
        _ => panic!("Expected Custom job type"),
    }
}

#[test]
fn test_job_type_clone() {
    // Test cloning job types
    let job_type = UniversalJobType::ComputeIntensive;
    let cloned = job_type.clone();

    match (job_type, cloned) {
        (UniversalJobType::ComputeIntensive, UniversalJobType::ComputeIntensive) => {
            // Success
        }
        _ => panic!("Cloned job type should match original"),
    }
}

// ============================================================================
// JobPriority Tests (6 tests)
// ============================================================================

#[test]
fn test_job_priority_emergency() {
    // Test emergency priority
    let priority = JobPriority::Emergency;

    match priority {
        JobPriority::Emergency => {
            // Success
        }
        _ => panic!("Expected Emergency priority"),
    }
}

#[test]
fn test_job_priority_critical() {
    // Test critical priority
    let priority = JobPriority::Critical;

    match priority {
        JobPriority::Critical => {
            // Success
        }
        _ => panic!("Expected Critical priority"),
    }
}

#[test]
fn test_job_priority_high() {
    // Test high priority
    let priority = JobPriority::High;

    match priority {
        JobPriority::High => {
            // Success
        }
        _ => panic!("Expected High priority"),
    }
}

#[test]
fn test_job_priority_normal() {
    // Test normal priority
    let priority = JobPriority::Normal;

    match priority {
        JobPriority::Normal => {
            // Success
        }
        _ => panic!("Expected Normal priority"),
    }
}

#[test]
fn test_job_priority_low() {
    // Test low priority
    let priority = JobPriority::Low;

    match priority {
        JobPriority::Low => {
            // Success
        }
        _ => panic!("Expected Low priority"),
    }
}

#[test]
fn test_job_priority_clone() {
    // Test cloning priority
    let priority = JobPriority::High;
    let cloned = priority;

    match (priority, cloned) {
        (JobPriority::High, JobPriority::High) => {
            // Success
        }
        _ => panic!("Cloned priority should match original"),
    }
}

// ============================================================================
// UniversalJobQueue Tests (5 tests)
// ============================================================================

#[test]
fn test_job_queue_creation() {
    // Test creating a new job queue
    let queue = UniversalJobQueue::new();

    assert_eq!(queue.total_jobs(), 0, "New queue should be empty");
}

#[test]
fn test_job_queue_default() {
    // Test default job queue creation
    let queue = UniversalJobQueue::default();

    assert_eq!(queue.total_jobs(), 0, "Default queue should be empty");
}

#[test]
fn test_job_queue_empty_count() {
    // Test empty queue job count
    let queue = UniversalJobQueue::new();

    assert_eq!(queue.total_jobs(), 0, "New queue should have 0 jobs");
}

#[test]
fn test_job_queue_multiple_instances() {
    // Test creating multiple queue instances
    let queue1 = UniversalJobQueue::new();
    let queue2 = UniversalJobQueue::new();

    assert_eq!(queue1.total_jobs(), 0);
    assert_eq!(queue2.total_jobs(), 0);
}

#[test]
fn test_job_queue_debug_format() {
    // Test debug formatting
    let queue = UniversalJobQueue::new();
    let debug_str = format!("{:?}", queue);

    assert!(
        debug_str.contains("UniversalJobQueue"),
        "Debug output should contain type name"
    );
}

// ============================================================================
// LoadBalancingStrategy Tests (4 tests)
// ============================================================================

#[test]
fn test_load_balancing_round_robin() {
    // Test round-robin strategy
    let strategy = LoadBalancingStrategy::RoundRobin;

    match strategy {
        LoadBalancingStrategy::RoundRobin => {
            // Success
        }
        _ => panic!("Expected RoundRobin strategy"),
    }
}

#[test]
fn test_load_balancing_least_connections() {
    // Test least-connections strategy
    let strategy = LoadBalancingStrategy::LeastConnections;

    match strategy {
        LoadBalancingStrategy::LeastConnections => {
            // Success
        }
        _ => panic!("Expected LeastConnections strategy"),
    }
}

#[test]
fn test_load_balancing_resource_aware() {
    // Test resource-aware strategy
    let strategy = LoadBalancingStrategy::ResourceAware;

    match strategy {
        LoadBalancingStrategy::ResourceAware => {
            // Success
        }
        _ => panic!("Expected ResourceAware strategy"),
    }
}

#[test]
fn test_load_balancing_clone() {
    // Test cloning strategy
    let strategy = LoadBalancingStrategy::RoundRobin;
    let cloned = strategy.clone();

    match (strategy, cloned) {
        (LoadBalancingStrategy::RoundRobin, LoadBalancingStrategy::RoundRobin) => {
            // Success
        }
        _ => panic!("Cloned strategy should match original"),
    }
}

// ============================================================================
// ExecutionTarget Tests (4 tests)
// ============================================================================

#[test]
fn test_execution_target_local() {
    // Test local execution target
    let target = ExecutionTarget::Local;

    match target {
        ExecutionTarget::Local => {
            // Success
        }
        _ => panic!("Expected Local execution target"),
    }
}

#[test]
fn test_execution_target_toadstool() {
    // Test ToadStool execution target
    let target = ExecutionTarget::ToadStool {
        instance_id: "toadstool-001".to_string(),
        endpoint: "http://localhost:8080".to_string(),
    };

    match target {
        ExecutionTarget::ToadStool {
            instance_id,
            endpoint,
        } => {
            assert_eq!(instance_id, "toadstool-001");
            assert_eq!(endpoint, "http://localhost:8080");
        }
        _ => panic!("Expected ToadStool execution target"),
    }
}

#[test]
fn test_execution_target_ecosystem_service() {
    // Test ecosystem service execution target
    let target = ExecutionTarget::EcosystemService {
        service_name: "beardog".to_string(),
        endpoint: "http://localhost:9090".to_string(),
    };

    match target {
        ExecutionTarget::EcosystemService {
            service_name,
            endpoint,
        } => {
            assert_eq!(service_name, "beardog");
            assert_eq!(endpoint, "http://localhost:9090");
        }
        _ => panic!("Expected EcosystemService execution target"),
    }
}

#[test]
fn test_execution_target_clone() {
    // Test cloning execution target
    let target = ExecutionTarget::Local;
    let cloned = target.clone();

    match (target, cloned) {
        (ExecutionTarget::Local, ExecutionTarget::Local) => {
            // Success
        }
        _ => panic!("Cloned target should match original"),
    }
}

// ============================================================================
// CompatibilityMode Tests (5 tests)
// ============================================================================

#[test]
fn test_compatibility_mode_native() {
    // Test native compatibility mode
    let mode = CompatibilityMode::Native;

    match mode {
        CompatibilityMode::Native => {
            // Success
        }
        _ => panic!("Expected Native compatibility mode"),
    }
}

#[test]
fn test_compatibility_mode_container() {
    // Test container compatibility mode
    let mode = CompatibilityMode::Container;

    match mode {
        CompatibilityMode::Container => {
            // Success
        }
        _ => panic!("Expected Container compatibility mode"),
    }
}

#[test]
fn test_compatibility_mode_emulated() {
    // Test emulated compatibility mode
    let mode = CompatibilityMode::Emulated;

    match mode {
        CompatibilityMode::Emulated => {
            // Success
        }
        _ => panic!("Expected Emulated compatibility mode"),
    }
}

#[test]
fn test_compatibility_mode_to_string() {
    // Test converting compatibility mode to string (zero-copy)
    let mode = CompatibilityMode::Native;
    let mode_str = mode.as_str(); // Zero-copy optimization

    assert_eq!(mode_str, "native");

    // Test owned string if needed
    let owned = mode.as_str().to_string();
    assert_eq!(owned, "native");
}

#[test]
fn test_compatibility_mode_clone() {
    // Test cloning compatibility mode
    let mode = CompatibilityMode::Container;
    let cloned = mode.clone();

    match (mode, cloned) {
        (CompatibilityMode::Container, CompatibilityMode::Container) => {
            // Success
        }
        _ => panic!("Cloned mode should match original"),
    }
}
