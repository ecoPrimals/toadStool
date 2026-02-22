use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::types::resources::ResourceAllocation;

/// Node assignment for distributed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAssignment {
    /// Node ID
    pub node_id: String,
    /// Assigned resources
    pub resources: ResourceAllocation,
    /// Task assignments
    pub tasks: Vec<String>,
}

/// Status of distributed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributedExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedExecution {
    pub execution_id: Uuid,
    pub distribution_time: DateTime<Utc>,
    pub node_assignments: Vec<NodeAssignment>,
    pub resource_allocations: Vec<ResourceAllocation>,
    pub status: DistributedExecutionStatus,
}

/// Job distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDistributionResult {
    pub job_id: Uuid,
    pub target_node: String,
    pub distribution_time: DateTime<Utc>,
}

/// Universal execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalExecutionResult {
    pub substrate_used: String,
    pub execution_time_ms: f64,
    pub energy_consumed_joules: f64,
    pub result_data: Vec<u8>,
    pub performance_metrics: HashMap<String, f64>,
    pub substrate_health_post_execution: Option<String>,
}

/// Platform-specific execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSpecificExecution {
    pub target_platform: String,
    pub execution_context: String,
    pub resource_requirements: PlatformResourceRequirements,
    pub execution_commands: Vec<String>,
    pub environment_setup: HashMap<String, String>,
}

/// Optimized execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedExecution {
    pub platform_execution: PlatformSpecificExecution,
    pub optimizations_applied: Vec<String>,
    pub performance_predictions: PerformancePredictions,
}

/// Platform-specific resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformResourceRequirements {
    pub compute_units: u32,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bandwidth_bps: u64,
    pub specialized_hardware: Vec<String>,
}

/// Performance predictions for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePredictions {
    pub estimated_runtime_ms: f64,
    pub memory_usage_peak_bytes: u64,
    pub energy_consumption_joules: f64,
    pub reliability_score: f64,
}

/// Biological computation specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologicalComputation {
    pub computation_type: String,
    pub input_molecules: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub reaction_conditions: HashMap<String, String>,
    pub timeout_hours: f64,
}

/// Biological computation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologicalResult {
    pub output_molecules: Vec<String>,
    pub reaction_efficiency: f64,
    pub computation_time_hours: f64,
    pub side_reactions: Vec<String>,
}

/// Biological system health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologicalHealthStatus {
    pub system_viability: f64,
    pub contamination_level: f64,
    pub resource_consumption: HashMap<String, f64>,
    pub waste_accumulation: HashMap<String, f64>,
}

/// Neuromorphic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuromorphicConfig {
    pub platform: String,
    pub neuron_model: String,
    pub synapse_model: String,
    pub learning_rule: String,
    pub connectivity_pattern: String,
}

/// Spiking neural network specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikingNeuralNetwork {
    pub network_topology: String,
    pub neuron_parameters: HashMap<String, f64>,
    pub synapse_parameters: HashMap<String, f64>,
    pub input_encoding: String,
    pub output_decoding: String,
}

/// Spike train data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeTrains {
    pub spike_times: Vec<Vec<f64>>,
    pub neuron_ids: Vec<usize>,
    pub total_simulation_time_ms: f64,
}

/// Echo state training data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoStateTrainingData {
    pub input_sequences: Vec<Vec<f64>>,
    pub target_sequences: Vec<Vec<f64>>,
    pub reservoir_size: usize,
    pub leak_rate: f64,
    pub input_scaling: f64,
}

/// Trained echo state network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainedESN {
    pub reservoir_weights: Vec<Vec<f64>>,
    pub input_weights: Vec<Vec<f64>>,
    pub output_weights: Vec<Vec<f64>>,
    pub performance_metrics: HashMap<String, f64>,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_assignment_construction() {
        let assignment = NodeAssignment {
            node_id: "node-1".to_string(),
            resources: ResourceAllocation::default(),
            tasks: vec!["task-a".to_string(), "task-b".to_string()],
        };
        assert_eq!(assignment.node_id, "node-1");
        assert_eq!(assignment.tasks.len(), 2);
    }

    #[test]
    fn test_distributed_execution_status_variants() {
        let _pending = DistributedExecutionStatus::Pending;
        let _running = DistributedExecutionStatus::Running;
        let _completed = DistributedExecutionStatus::Completed;
        let _failed = DistributedExecutionStatus::Failed("error".to_string());
        let _cancelled = DistributedExecutionStatus::Cancelled;
    }

    #[test]
    fn test_job_distribution_result_serialization_roundtrip() {
        let result = JobDistributionResult {
            job_id: Uuid::new_v4(),
            target_node: "node-1".to_string(),
            distribution_time: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: JobDistributionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.target_node, result.target_node);
    }

    #[test]
    fn test_universal_execution_result_construction() {
        let result = UniversalExecutionResult {
            substrate_used: "native".to_string(),
            execution_time_ms: 123.5,
            energy_consumed_joules: 0.5,
            result_data: vec![1, 2, 3],
            performance_metrics: HashMap::new(),
            substrate_health_post_execution: Some("healthy".to_string()),
        };
        assert_eq!(result.substrate_used, "native");
        assert!((result.execution_time_ms - 123.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_platform_resource_requirements_construction() {
        let req = PlatformResourceRequirements {
            compute_units: 8,
            memory_bytes: 16 * 1024 * 1024 * 1024,
            storage_bytes: 100 * 1024 * 1024 * 1024,
            network_bandwidth_bps: 1_000_000_000,
            specialized_hardware: vec!["gpu".to_string()],
        };
        assert_eq!(req.compute_units, 8);
    }

    #[test]
    fn test_neuromorphic_config_serialization_roundtrip() {
        let config = NeuromorphicConfig {
            platform: "akida".to_string(),
            neuron_model: "lif".to_string(),
            synapse_model: "stdp".to_string(),
            learning_rule: "hebbian".to_string(),
            connectivity_pattern: "sparse".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: NeuromorphicConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.platform, config.platform);
    }
}
