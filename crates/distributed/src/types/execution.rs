// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
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

/// Status of distributed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributedExecutionStatus {
    /// Execution queued, not yet started.
    Pending,
    /// Execution in progress.
    Running,
    /// Execution completed successfully.
    Completed,
    /// Execution failed with error message.
    Failed(String),
    /// Execution was cancelled.
    Cancelled,
}

/// Distributed execution tracking with node assignments and status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedExecution {
    /// Unique execution identifier.
    pub execution_id: Uuid,
    /// When the execution was distributed to nodes.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub distribution_time: SystemTime,
    /// Per-node task assignments.
    pub node_assignments: Vec<NodeAssignment>,
    /// Resource allocations across nodes.
    pub resource_allocations: Vec<ResourceAllocation>,
    /// Current execution status.
    pub status: DistributedExecutionStatus,
}

/// Result of distributing a job to a target node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDistributionResult {
    /// Job identifier.
    pub job_id: Uuid,
    /// Node that received the job.
    pub target_node: String,
    /// When the job was distributed.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub distribution_time: SystemTime,
}

/// Universal execution result across substrates (native, GPU, neuromorphic, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalExecutionResult {
    /// Substrate used (e.g. native, gpu, neuromorphic).
    pub substrate_used: String,
    /// Execution time in milliseconds.
    pub execution_time_ms: f64,
    /// Energy consumed in joules.
    pub energy_consumed_joules: f64,
    /// Raw result bytes.
    pub result_data: Vec<u8>,
    /// Performance metrics (throughput, latency, etc.).
    pub performance_metrics: HashMap<String, f64>,
    /// Post-execution substrate health (optional).
    pub substrate_health_post_execution: Option<String>,
}

/// Platform-specific execution configuration for heterogeneous substrates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSpecificExecution {
    /// Target platform identifier.
    pub target_platform: String,
    /// Execution context (e.g. batch, interactive).
    pub execution_context: String,
    /// Resource requirements for this platform.
    pub resource_requirements: PlatformResourceRequirements,
    /// Commands to execute.
    pub execution_commands: Vec<String>,
    /// Environment variables for execution.
    pub environment_setup: HashMap<String, String>,
}

/// Optimized execution configuration with applied optimizations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedExecution {
    /// Platform-specific execution config.
    pub platform_execution: PlatformSpecificExecution,
    /// List of optimizations applied.
    pub optimizations_applied: Vec<String>,
    /// Predicted performance metrics.
    pub performance_predictions: PerformancePredictions,
}

/// Platform-specific resource requirements for specialized hardware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformResourceRequirements {
    /// Compute units (cores, tiles, etc.).
    pub compute_units: u32,
    /// Memory in bytes.
    pub memory_bytes: u64,
    /// Storage in bytes.
    pub storage_bytes: u64,
    /// Network bandwidth in bits per second.
    pub network_bandwidth_bps: u64,
    /// Specialized hardware (gpu, npu, neuromorphic, etc.).
    pub specialized_hardware: Vec<String>,
}

/// Performance predictions for execution planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePredictions {
    /// Estimated runtime in ms.
    pub estimated_runtime_ms: f64,
    /// Peak memory usage in bytes.
    pub memory_usage_peak_bytes: u64,
    /// Predicted energy consumption in joules.
    pub energy_consumption_joules: f64,
    /// Reliability score (0.0–1.0).
    pub reliability_score: f64,
}

/// Biological computation specification for DNA/compute substrates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologicalComputation {
    /// Type of biological computation.
    pub computation_type: String,
    /// Input molecule identifiers.
    pub input_molecules: Vec<String>,
    /// Expected output molecule identifiers.
    pub expected_outputs: Vec<String>,
    /// Reaction conditions (temperature, pH, etc.).
    pub reaction_conditions: HashMap<String, String>,
    /// Timeout in hours.
    pub timeout_hours: f64,
}

/// Result of biological computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologicalResult {
    /// Output molecule identifiers.
    pub output_molecules: Vec<String>,
    /// Reaction efficiency (0.0–1.0).
    pub reaction_efficiency: f64,
    /// Actual computation time in hours.
    pub computation_time_hours: f64,
    /// Side reactions that occurred.
    pub side_reactions: Vec<String>,
}

/// Biological system health for substrate monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologicalHealthStatus {
    /// System viability (0.0–1.0).
    pub system_viability: f64,
    /// Contamination level.
    pub contamination_level: f64,
    /// Resource consumption per resource type.
    pub resource_consumption: HashMap<String, f64>,
    /// Waste accumulation per type.
    pub waste_accumulation: HashMap<String, f64>,
}

/// Neuromorphic platform configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuromorphicConfig {
    /// Platform identifier (e.g. akida, loihi).
    pub platform: String,
    /// Neuron model (e.g. LIF, AdEx).
    pub neuron_model: String,
    /// Synapse model (e.g. STDP).
    pub synapse_model: String,
    /// Learning rule (e.g. hebbian).
    pub learning_rule: String,
    /// Connectivity pattern (sparse, dense, etc.).
    pub connectivity_pattern: String,
}

/// Spiking neural network specification for neuromorphic execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikingNeuralNetwork {
    /// Network topology description.
    pub network_topology: String,
    /// Neuron parameters per layer/type.
    pub neuron_parameters: HashMap<String, f64>,
    /// Synapse parameters.
    pub synapse_parameters: HashMap<String, f64>,
    /// Input encoding (rate, temporal, etc.).
    pub input_encoding: String,
    /// Output decoding method.
    pub output_decoding: String,
}

/// Spike train data for neuromorphic simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeTrains {
    /// Spike times per neuron (nested by neuron).
    pub spike_times: Vec<Vec<f64>>,
    /// Neuron IDs corresponding to spike_times.
    pub neuron_ids: Vec<usize>,
    /// Total simulation time in ms.
    pub total_simulation_time_ms: f64,
}

/// Echo state network training data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoStateTrainingData {
    /// Input sequences for training.
    pub input_sequences: Vec<Vec<f64>>,
    /// Target sequences for supervised learning.
    pub target_sequences: Vec<Vec<f64>>,
    /// Reservoir size (number of reservoir neurons).
    pub reservoir_size: usize,
    /// Leak rate for reservoir dynamics.
    pub leak_rate: f64,
    /// Input scaling factor.
    pub input_scaling: f64,
}

/// Trained echo state network weights and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainedESN {
    /// Reservoir weight matrix.
    pub reservoir_weights: Vec<Vec<f64>>,
    /// Input-to-reservoir weights.
    pub input_weights: Vec<Vec<f64>>,
    /// Reservoir-to-output weights.
    pub output_weights: Vec<Vec<f64>>,
    /// Training performance metrics.
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
            distribution_time: SystemTime::now(),
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
