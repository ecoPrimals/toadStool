// SPDX-License-Identifier: AGPL-3.0-or-later
//! Graph node types for workflow representation
//!
//! This module contains types for representing individual nodes in a workflow graph,
//! including resource requirements and a fluent builder pattern.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use toadstool::resources::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
};
use toadstool_common::constants::primal_identity;

/// A node in the execution graph representing a workload unit
///
/// ## Self-Knowledge Principle
///
/// Each node encapsulates knowledge of:
/// - Which primal it belongs to
/// - What operation it performs
/// - What resources it needs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique node identifier
    pub id: String,

    /// Primal name (e.g., "toadstool", "intelligence", "storage")
    /// This is self-knowledge - the node knows which primal it needs
    /// Defaults to "toadstool" if not specified
    #[serde(default = "default_primal")]
    pub primal: String,

    /// Operation type (e.g., "gpu_compute", "cpu_compute", "storage")
    pub operation: String,

    /// Resource requirements for this node
    #[serde(default)]
    pub requirements: NodeResourceRequirements,

    /// Estimated execution duration (type-safe)
    /// Replaces duration_secs in metadata for better ergonomics
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub duration: Option<Duration>,

    /// Optional metadata (workload hints, model size, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_primal() -> String {
    primal_identity::PRIMAL_NAME.to_string()
}

#[expect(
    clippy::ref_option,
    reason = "serde helper signature uses &Option for serialize_with"
)]
fn serialize_duration<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match duration {
        Some(d) => serializer.serialize_u64(d.as_secs()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let secs: Option<u64> = Option::deserialize(deserializer)?;
    Ok(secs.map(Duration::from_secs))
}

/// Resource requirements for a graph node
///
/// Uses Option for all fields to allow partial specification.
/// Estimation logic will provide sensible defaults for missing requirements.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeResourceRequirements {
    /// CPU requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuRequirements>,

    /// Memory requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryRequirements>,

    /// Storage requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageRequirements>,

    /// GPU requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuRequirements>,

    /// Network requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkRequirements>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Builder Pattern - Modern Idiomatic Rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl GraphNode {
    /// Create a builder for ergonomic node construction
    ///
    /// # Example
    /// ```rust,ignore
    /// let node = GraphNode::builder("my_node", "gpu_compute")
    ///     .cpu(4.0)
    ///     .memory_gb(8)
    ///     .gpu_memory_gb(16)
    ///     .duration_secs(60)
    ///     .build();
    /// ```
    pub fn builder(id: impl Into<String>, operation: impl Into<String>) -> GraphNodeBuilder {
        GraphNodeBuilder::new(id, operation)
    }

    /// Create a simple node with defaults
    pub fn simple(id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            primal: primal_identity::PRIMAL_NAME.to_string(),
            operation: operation.into(),
            requirements: NodeResourceRequirements::default(),
            duration: None,
            metadata: HashMap::new(),
        }
    }
}

/// Builder for GraphNode with fluent API
///
/// Provides ergonomic construction of graph nodes with sensible defaults.
pub struct GraphNodeBuilder {
    id: String,
    primal: String,
    operation: String,
    cpu_cores: Option<f64>,
    memory_bytes: Option<u64>,
    gpu_memory_bytes: Option<u64>,
    storage_bytes: Option<u64>,
    network_bandwidth_mbps: Option<u64>,
    duration: Option<Duration>,
    metadata: HashMap<String, String>,
}

impl GraphNodeBuilder {
    /// Create a new builder with required fields
    #[must_use]
    pub fn new(id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            primal: primal_identity::PRIMAL_NAME.to_string(),
            operation: operation.into(),
            cpu_cores: None,
            memory_bytes: None,
            gpu_memory_bytes: None,
            storage_bytes: None,
            network_bandwidth_mbps: None,
            duration: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the primal for this node
    #[must_use]
    pub fn primal(mut self, primal: impl Into<String>) -> Self {
        self.primal = primal.into();
        self
    }

    /// Set CPU cores required
    #[must_use]
    pub fn cpu(mut self, cores: f64) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    /// Set memory in bytes
    #[must_use]
    pub fn memory_bytes(mut self, bytes: u64) -> Self {
        self.memory_bytes = Some(bytes);
        self
    }

    /// Set memory in gigabytes (convenience method)
    #[must_use]
    pub fn memory_gb(mut self, gb: u64) -> Self {
        self.memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Set GPU memory in bytes
    #[must_use]
    pub fn gpu_memory_bytes(mut self, bytes: u64) -> Self {
        self.gpu_memory_bytes = Some(bytes);
        self
    }

    /// Set GPU memory in gigabytes (convenience method)
    #[must_use]
    pub fn gpu_memory_gb(mut self, gb: u64) -> Self {
        self.gpu_memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Set storage requirement in bytes
    #[must_use]
    pub fn storage_bytes(mut self, bytes: u64) -> Self {
        self.storage_bytes = Some(bytes);
        self
    }

    /// Set storage requirement in gigabytes
    #[must_use]
    pub fn storage_gb(mut self, gb: u64) -> Self {
        self.storage_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Set network bandwidth requirement in Mbps
    #[must_use]
    pub fn network_mbps(mut self, mbps: u64) -> Self {
        self.network_bandwidth_mbps = Some(mbps);
        self
    }

    /// Set execution duration
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set execution duration in seconds (convenience method)
    #[must_use]
    pub fn duration_secs(mut self, secs: u64) -> Self {
        self.duration = Some(Duration::from_secs(secs));
        self
    }

    /// Add metadata key-value pair
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build the GraphNode
    pub fn build(self) -> GraphNode {
        let requirements = NodeResourceRequirements {
            cpu: self.cpu_cores.map(|cores| CpuRequirements {
                min_cores: cores,
                max_cores: None,
                architecture: None,
            }),
            memory: self.memory_bytes.map(|bytes| MemoryRequirements {
                min_bytes: bytes,
                max_bytes: None,
            }),
            gpu: self.gpu_memory_bytes.map(|memory| GpuRequirements {
                min_units: 1,
                max_units: None,
                gpu_type: None,
                min_memory_bytes: Some(memory),
            }),
            storage: self.storage_bytes.map(|bytes| StorageRequirements {
                min_bytes: bytes,
                max_bytes: None,
                storage_type: None,
            }),
            network: self.network_bandwidth_mbps.map(|mbps| NetworkRequirements {
                min_bandwidth: Some(mbps * 1024 * 1024 / 8), // Mbps to bytes/sec
                max_bandwidth: None,
                max_latency_ms: None,
            }),
        };

        GraphNode {
            id: self.id,
            primal: self.primal,
            operation: self.operation,
            requirements,
            duration: self.duration,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_node_builder() {
        let node = GraphNode::builder("test_node", "gpu_compute")
            .cpu(4.0)
            .memory_gb(8)
            .gpu_memory_gb(16)
            .duration_secs(60)
            .metadata("model", "llama2-7b")
            .build();

        assert_eq!(node.id, "test_node");
        assert_eq!(node.operation, "gpu_compute");
        assert_eq!(
            node.requirements.cpu.as_ref().map(|c| c.min_cores),
            Some(4.0)
        );
        assert_eq!(
            node.requirements.memory.as_ref().map(|m| m.min_bytes),
            Some(8 * 1024 * 1024 * 1024)
        );
        assert_eq!(node.duration, Some(Duration::from_secs(60)));
        assert_eq!(node.metadata.get("model"), Some(&"llama2-7b".to_string()));
    }

    #[test]
    fn test_simple_node() {
        let node = GraphNode::simple("simple_node", "cpu_compute");
        assert_eq!(node.id, "simple_node");
        assert_eq!(node.primal, "toadstool");
        assert_eq!(node.operation, "cpu_compute");
        assert!(node.requirements.cpu.is_none());
    }

    #[test]
    fn test_node_resource_requirements_default() {
        let req = NodeResourceRequirements::default();
        assert!(req.cpu.is_none());
        assert!(req.memory.is_none());
        assert!(req.storage.is_none());
        assert!(req.gpu.is_none());
        assert!(req.network.is_none());
    }

    #[test]
    fn test_graph_node_serialization_roundtrip() {
        let node = GraphNode::builder("serial_node", "gpu_compute")
            .duration_secs(120)
            .metadata("k", "v")
            .build();
        let json = serde_json::to_string(&node).unwrap();
        let restored: GraphNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node.id, restored.id);
        assert_eq!(node.operation, restored.operation);
        assert_eq!(node.duration, restored.duration);
        assert_eq!(node.metadata.get("k"), restored.metadata.get("k"));
    }

    #[test]
    fn test_graph_node_builder_primal() {
        let node = GraphNode::builder("n", "op").primal("squirrel").build();
        assert_eq!(node.primal, "squirrel");
    }
}
