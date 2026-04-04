// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for distributed execution types

#![expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]

use std::collections::HashMap;
use std::time::SystemTime;
use toadstool_distributed::types::execution::*;
use toadstool_distributed::*;
use uuid::Uuid;

// ============================================================================
// DistributedExecutionStatus Tests
// ============================================================================

#[test]
fn test_execution_status_pending() {
    let status = DistributedExecutionStatus::Pending;
    assert!(matches!(status, DistributedExecutionStatus::Pending));
}

#[test]
fn test_execution_status_running() {
    let status = DistributedExecutionStatus::Running;
    assert!(matches!(status, DistributedExecutionStatus::Running));
}

#[test]
fn test_execution_status_completed() {
    let status = DistributedExecutionStatus::Completed;
    assert!(matches!(status, DistributedExecutionStatus::Completed));
}

#[test]
fn test_execution_status_failed() {
    let status = DistributedExecutionStatus::Failed("timeout".to_string());

    match status {
        DistributedExecutionStatus::Failed(msg) => assert_eq!(msg, "timeout"),
        _ => panic!("Expected Failed variant"),
    }
}

#[test]
fn test_execution_status_cancelled() {
    let status = DistributedExecutionStatus::Cancelled;
    assert!(matches!(status, DistributedExecutionStatus::Cancelled));
}

#[test]
fn test_execution_status_serialization() {
    let status = DistributedExecutionStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: DistributedExecutionStatus = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, DistributedExecutionStatus::Running));
}

// ============================================================================
// NodeAssignment Tests
// ============================================================================

#[test]
fn test_node_assignment_creation() {
    let assignment = NodeAssignment {
        node_id: "node-001".to_string(),
        resources: ResourceAllocation::default(),
        tasks: vec!["task1".to_string(), "task2".to_string()],
    };

    assert_eq!(assignment.node_id, "node-001");
    assert_eq!(assignment.tasks.len(), 2);
}

#[test]
fn test_node_assignment_empty_tasks() {
    let assignment = NodeAssignment {
        node_id: "node-002".to_string(),
        resources: ResourceAllocation::default(),
        tasks: vec![],
    };

    assert!(assignment.tasks.is_empty());
}

#[test]
fn test_node_assignment_serialization() {
    let assignment = NodeAssignment {
        node_id: "test-node".to_string(),
        resources: ResourceAllocation::default(),
        tasks: vec!["test-task".to_string()],
    };

    let json = serde_json::to_string(&assignment).unwrap();
    let deserialized: NodeAssignment = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.node_id, "test-node");
}

// ============================================================================
// DistributedExecution Tests
// ============================================================================

#[test]
fn test_distributed_execution_creation() {
    let execution = DistributedExecution {
        execution_id: Uuid::new_v4(),
        distribution_time: SystemTime::now(),
        node_assignments: vec![],
        resource_allocations: vec![],
        status: DistributedExecutionStatus::Pending,
    };

    assert!(matches!(
        execution.status,
        DistributedExecutionStatus::Pending
    ));
    assert!(execution.node_assignments.is_empty());
}

#[test]
fn test_distributed_execution_with_assignments() {
    let assignment = NodeAssignment {
        node_id: "node-1".to_string(),
        resources: ResourceAllocation::default(),
        tasks: vec!["task1".to_string()],
    };

    let execution = DistributedExecution {
        execution_id: Uuid::new_v4(),
        distribution_time: SystemTime::now(),
        node_assignments: vec![assignment],
        resource_allocations: vec![],
        status: DistributedExecutionStatus::Running,
    };

    assert_eq!(execution.node_assignments.len(), 1);
    assert_eq!(execution.node_assignments[0].node_id, "node-1");
}

#[test]
fn test_distributed_execution_serialization() {
    let execution = DistributedExecution {
        execution_id: Uuid::new_v4(),
        distribution_time: SystemTime::now(),
        node_assignments: vec![],
        resource_allocations: vec![],
        status: DistributedExecutionStatus::Completed,
    };

    let json = serde_json::to_string(&execution).unwrap();
    let deserialized: DistributedExecution = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.execution_id, execution.execution_id);
}

// ============================================================================
// JobDistributionResult Tests
// ============================================================================

#[test]
fn test_job_distribution_result_creation() {
    let result = JobDistributionResult {
        job_id: Uuid::new_v4(),
        target_node: "node-001".to_string(),
        distribution_time: SystemTime::now(),
    };

    assert_eq!(result.target_node, "node-001");
}

#[test]
fn test_job_distribution_result_serialization() {
    let result = JobDistributionResult {
        job_id: Uuid::new_v4(),
        target_node: "test-node".to_string(),
        distribution_time: SystemTime::now(),
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: JobDistributionResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.job_id, result.job_id);
    assert_eq!(deserialized.target_node, "test-node");
}

// ============================================================================
// UniversalExecutionResult Tests
// ============================================================================

#[test]
fn test_universal_execution_result_creation() {
    let result = UniversalExecutionResult {
        substrate_used: "x86_64-linux".to_string(),
        execution_time_ms: 125.5,
        energy_consumed_joules: 10.2,
        result_data: vec![1, 2, 3],
        performance_metrics: HashMap::new(),
        substrate_health_post_execution: None,
    };

    assert_eq!(result.substrate_used, "x86_64-linux");
    assert_eq!(result.execution_time_ms, 125.5);
}

#[test]
fn test_universal_execution_result_with_metrics() {
    let mut metrics = HashMap::new();
    metrics.insert("cpu_utilization".to_string(), 75.5);
    metrics.insert("memory_peak_mb".to_string(), 512.0);

    let result = UniversalExecutionResult {
        substrate_used: "aarch64-linux".to_string(),
        execution_time_ms: 200.0,
        energy_consumed_joules: 15.5,
        result_data: vec![],
        performance_metrics: metrics,
        substrate_health_post_execution: Some("healthy".to_string()),
    };

    assert_eq!(result.performance_metrics.len(), 2);
    assert_eq!(
        result.performance_metrics.get("cpu_utilization").unwrap(),
        &75.5
    );
}

#[test]
fn test_universal_execution_result_serialization() {
    let result = UniversalExecutionResult {
        substrate_used: "test".to_string(),
        execution_time_ms: 100.0,
        energy_consumed_joules: 5.0,
        result_data: vec![1, 2, 3],
        performance_metrics: HashMap::new(),
        substrate_health_post_execution: None,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: UniversalExecutionResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.substrate_used, "test");
    assert_eq!(deserialized.result_data, vec![1, 2, 3]);
}

// ============================================================================
// PlatformResourceRequirements Tests
// ============================================================================

#[test]
fn test_platform_resource_requirements_basic() {
    let requirements = PlatformResourceRequirements {
        compute_units: 4,
        memory_bytes: 8 * 1024 * 1024 * 1024,    // 8 GB
        storage_bytes: 100 * 1024 * 1024 * 1024, // 100 GB
        network_bandwidth_bps: 1_000_000_000,    // 1 Gbps
        specialized_hardware: vec![],
    };

    assert_eq!(requirements.compute_units, 4);
    assert_eq!(requirements.memory_bytes, 8 * 1024 * 1024 * 1024);
}

#[test]
fn test_platform_resource_requirements_with_hardware() {
    let requirements = PlatformResourceRequirements {
        compute_units: 8,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 1024 * 1024 * 1024 * 1024, // 1 TB
        network_bandwidth_bps: 10_000_000_000,    // 10 Gbps
        specialized_hardware: vec!["GPU".to_string(), "TPU".to_string()],
    };

    assert_eq!(requirements.specialized_hardware.len(), 2);
    assert!(
        requirements
            .specialized_hardware
            .contains(&"GPU".to_string())
    );
}

#[test]
fn test_platform_resource_requirements_serialization() {
    let requirements = PlatformResourceRequirements {
        compute_units: 2,
        memory_bytes: 4 * 1024 * 1024 * 1024,
        storage_bytes: 50 * 1024 * 1024 * 1024,
        network_bandwidth_bps: 100_000_000,
        specialized_hardware: vec![],
    };

    let json = serde_json::to_string(&requirements).unwrap();
    let deserialized: PlatformResourceRequirements = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.compute_units, 2);
}

// ============================================================================
// PerformancePredictions Tests
// ============================================================================

#[test]
fn test_performance_predictions_creation() {
    let predictions = PerformancePredictions {
        estimated_runtime_ms: 1000.0,
        memory_usage_peak_bytes: 512 * 1024 * 1024,
        energy_consumption_joules: 50.0,
        reliability_score: 0.95,
    };

    assert_eq!(predictions.estimated_runtime_ms, 1000.0);
    assert_eq!(predictions.reliability_score, 0.95);
}

#[test]
fn test_performance_predictions_high_reliability() {
    let predictions = PerformancePredictions {
        estimated_runtime_ms: 500.0,
        memory_usage_peak_bytes: 256 * 1024 * 1024,
        energy_consumption_joules: 25.0,
        reliability_score: 0.99,
    };

    assert!(predictions.reliability_score > 0.9);
}

#[test]
fn test_performance_predictions_serialization() {
    let predictions = PerformancePredictions {
        estimated_runtime_ms: 750.0,
        memory_usage_peak_bytes: 1024 * 1024 * 1024,
        energy_consumption_joules: 75.0,
        reliability_score: 0.85,
    };

    let json = serde_json::to_string(&predictions).unwrap();
    let deserialized: PerformancePredictions = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.estimated_runtime_ms, 750.0);
}

// ============================================================================
// PlatformSpecificExecution Tests
// ============================================================================

#[test]
fn test_platform_specific_execution_creation() {
    let execution = PlatformSpecificExecution {
        target_platform: "linux-x86_64".to_string(),
        execution_context: "container".to_string(),
        resource_requirements: PlatformResourceRequirements {
            compute_units: 2,
            memory_bytes: 2 * 1024 * 1024 * 1024,
            storage_bytes: 10 * 1024 * 1024 * 1024,
            network_bandwidth_bps: 100_000_000,
            specialized_hardware: vec![],
        },
        execution_commands: vec!["./app".to_string(), "--flag".to_string()],
        environment_setup: HashMap::new(),
    };

    assert_eq!(execution.target_platform, "linux-x86_64");
    assert_eq!(execution.execution_commands.len(), 2);
}

#[test]
fn test_platform_specific_execution_with_env() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin".to_string());
    env.insert("HOME".to_string(), "/home/user".to_string());

    let execution = PlatformSpecificExecution {
        target_platform: "macos-arm64".to_string(),
        execution_context: "native".to_string(),
        resource_requirements: PlatformResourceRequirements {
            compute_units: 4,
            memory_bytes: 8 * 1024 * 1024 * 1024,
            storage_bytes: 50 * 1024 * 1024 * 1024,
            network_bandwidth_bps: 1_000_000_000,
            specialized_hardware: vec![],
        },
        execution_commands: vec!["run.sh".to_string()],
        environment_setup: env,
    };

    assert_eq!(execution.environment_setup.len(), 2);
}

#[test]
fn test_platform_specific_execution_serialization() {
    let execution = PlatformSpecificExecution {
        target_platform: "test".to_string(),
        execution_context: "test-context".to_string(),
        resource_requirements: PlatformResourceRequirements {
            compute_units: 1,
            memory_bytes: 1024 * 1024 * 1024,
            storage_bytes: 1024 * 1024 * 1024,
            network_bandwidth_bps: 10_000_000,
            specialized_hardware: vec![],
        },
        execution_commands: vec!["test".to_string()],
        environment_setup: HashMap::new(),
    };

    let json = serde_json::to_string(&execution).unwrap();
    let deserialized: PlatformSpecificExecution = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.target_platform, "test");
}

// ============================================================================
// OptimizedExecution Tests
// ============================================================================

#[test]
fn test_optimized_execution_creation() {
    let execution = OptimizedExecution {
        platform_execution: PlatformSpecificExecution {
            target_platform: "linux".to_string(),
            execution_context: "optimized".to_string(),
            resource_requirements: PlatformResourceRequirements {
                compute_units: 4,
                memory_bytes: 4 * 1024 * 1024 * 1024,
                storage_bytes: 20 * 1024 * 1024 * 1024,
                network_bandwidth_bps: 500_000_000,
                specialized_hardware: vec![],
            },
            execution_commands: vec!["run".to_string()],
            environment_setup: HashMap::new(),
        },
        optimizations_applied: vec!["vectorization".to_string(), "caching".to_string()],
        performance_predictions: PerformancePredictions {
            estimated_runtime_ms: 100.0,
            memory_usage_peak_bytes: 512 * 1024 * 1024,
            energy_consumption_joules: 10.0,
            reliability_score: 0.98,
        },
    };

    assert_eq!(execution.optimizations_applied.len(), 2);
    assert!(
        execution
            .optimizations_applied
            .contains(&"vectorization".to_string())
    );
}

#[test]
fn test_optimized_execution_serialization() {
    let execution = OptimizedExecution {
        platform_execution: PlatformSpecificExecution {
            target_platform: "test".to_string(),
            execution_context: "test".to_string(),
            resource_requirements: PlatformResourceRequirements {
                compute_units: 1,
                memory_bytes: 1024 * 1024 * 1024,
                storage_bytes: 1024 * 1024 * 1024,
                network_bandwidth_bps: 10_000_000,
                specialized_hardware: vec![],
            },
            execution_commands: vec![],
            environment_setup: HashMap::new(),
        },
        optimizations_applied: vec![],
        performance_predictions: PerformancePredictions {
            estimated_runtime_ms: 50.0,
            memory_usage_peak_bytes: 128 * 1024 * 1024,
            energy_consumption_joules: 5.0,
            reliability_score: 0.9,
        },
    };

    let json = serde_json::to_string(&execution).unwrap();
    let deserialized: OptimizedExecution = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.platform_execution.target_platform, "test");
}

// ============================================================================
// BiologicalComputation Tests
// ============================================================================

#[test]
fn test_biological_computation_creation() {
    let mut conditions = HashMap::new();
    conditions.insert("temperature".to_string(), "37C".to_string());
    conditions.insert("pH".to_string(), "7.4".to_string());

    let computation = BiologicalComputation {
        computation_type: "protein_folding".to_string(),
        input_molecules: vec!["protein_A".to_string()],
        expected_outputs: vec!["folded_protein_A".to_string()],
        reaction_conditions: conditions,
        timeout_hours: 24.0,
    };

    assert_eq!(computation.computation_type, "protein_folding");
    assert_eq!(computation.reaction_conditions.len(), 2);
}

#[test]
fn test_biological_computation_serialization() {
    let computation = BiologicalComputation {
        computation_type: "dna_synthesis".to_string(),
        input_molecules: vec!["nucleotides".to_string()],
        expected_outputs: vec!["dna_sequence".to_string()],
        reaction_conditions: HashMap::new(),
        timeout_hours: 12.0,
    };

    let json = serde_json::to_string(&computation).unwrap();
    let deserialized: BiologicalComputation = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.computation_type, "dna_synthesis");
}

// ============================================================================
// NeuromorphicConfig Tests
// ============================================================================

#[test]
fn test_neuromorphic_config_creation() {
    let config = NeuromorphicConfig {
        platform: "Loihi".to_string(),
        neuron_model: "LIF".to_string(),
        synapse_model: "STDP".to_string(),
        learning_rule: "Hebbian".to_string(),
        connectivity_pattern: "random".to_string(),
    };

    assert_eq!(config.platform, "Loihi");
    assert_eq!(config.neuron_model, "LIF");
}

#[test]
fn test_neuromorphic_config_serialization() {
    let config = NeuromorphicConfig {
        platform: "TrueNorth".to_string(),
        neuron_model: "integrate-and-fire".to_string(),
        synapse_model: "static".to_string(),
        learning_rule: "none".to_string(),
        connectivity_pattern: "all-to-all".to_string(),
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: NeuromorphicConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.platform, "TrueNorth");
}

// ============================================================================
// SpikingNeuralNetwork Tests
// ============================================================================

#[test]
fn test_spiking_neural_network_creation() {
    let mut neuron_params = HashMap::new();
    neuron_params.insert("threshold".to_string(), -55.0);
    neuron_params.insert("reset_potential".to_string(), -70.0);

    let network = SpikingNeuralNetwork {
        network_topology: "feedforward".to_string(),
        neuron_parameters: neuron_params,
        synapse_parameters: HashMap::new(),
        input_encoding: "rate".to_string(),
        output_decoding: "spike_count".to_string(),
    };

    assert_eq!(network.network_topology, "feedforward");
    assert_eq!(network.neuron_parameters.len(), 2);
}

#[test]
fn test_spiking_neural_network_serialization() {
    let network = SpikingNeuralNetwork {
        network_topology: "recurrent".to_string(),
        neuron_parameters: HashMap::new(),
        synapse_parameters: HashMap::new(),
        input_encoding: "temporal".to_string(),
        output_decoding: "population".to_string(),
    };

    let json = serde_json::to_string(&network).unwrap();
    let deserialized: SpikingNeuralNetwork = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.network_topology, "recurrent");
}

// ============================================================================
// SpikeTrains Tests
// ============================================================================

#[test]
fn test_spike_trains_creation() {
    let trains = SpikeTrains {
        spike_times: vec![vec![10.5, 25.3, 40.1], vec![15.2, 30.7]],
        neuron_ids: vec![0, 1],
        total_simulation_time_ms: 100.0,
    };

    assert_eq!(trains.spike_times.len(), 2);
    assert_eq!(trains.neuron_ids.len(), 2);
    assert_eq!(trains.total_simulation_time_ms, 100.0);
}

#[test]
fn test_spike_trains_empty() {
    let trains = SpikeTrains {
        spike_times: vec![],
        neuron_ids: vec![],
        total_simulation_time_ms: 0.0,
    };

    assert!(trains.spike_times.is_empty());
}

#[test]
fn test_spike_trains_serialization() {
    let trains = SpikeTrains {
        spike_times: vec![vec![1.0, 2.0], vec![1.5, 2.5]],
        neuron_ids: vec![0, 1],
        total_simulation_time_ms: 10.0,
    };

    let json = serde_json::to_string(&trains).unwrap();
    let deserialized: SpikeTrains = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.spike_times.len(), 2);
}
