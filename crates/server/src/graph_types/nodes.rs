// SPDX-License-Identifier: AGPL-3.0-or-later
//! Graph node types for workflow representation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use toadstool::resources::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
};
use toadstool_common::constants::PRIMAL_NAME;

/// Graph node representing a single workload unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Node identifier.
    pub id: String,
    /// Primal that executes this node.
    #[serde(default = "default_primal")]
    pub primal: String,
    /// Operation type.
    pub operation: String,
    /// Resource requirements.
    #[serde(default)]
    pub requirements: NodeResourceRequirements,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    /// Estimated duration.
    pub duration: Option<Duration>,
    /// Optional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_primal() -> String {
    PRIMAL_NAME.to_string()
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeResourceRequirements {
    /// CPU requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuRequirements>,
    /// Memory requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryRequirements>,
    /// Storage requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageRequirements>,
    /// GPU requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuRequirements>,
    /// Network requirements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkRequirements>,
}

impl GraphNode {
    /// Creates a builder for constructing a graph node.
    pub fn builder(id: impl Into<String>, operation: impl Into<String>) -> GraphNodeBuilder {
        GraphNodeBuilder::new(id, operation)
    }

    /// Creates a minimal node with default requirements.
    pub fn simple(id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            primal: PRIMAL_NAME.to_string(),
            operation: operation.into(),
            requirements: NodeResourceRequirements::default(),
            duration: None,
            metadata: HashMap::new(),
        }
    }
}

/// Builder for GraphNode with fluent API
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
    /// Creates a new builder with the given node ID and operation.
    pub fn new(id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            primal: PRIMAL_NAME.to_string(),
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

    /// Sets the primal type.
    #[must_use]
    pub fn primal(mut self, primal: impl Into<String>) -> Self {
        self.primal = primal.into();
        self
    }

    /// Sets CPU core requirement.
    #[must_use]
    pub fn cpu(mut self, cores: f64) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    /// Sets memory requirement in bytes.
    #[must_use]
    pub fn memory(mut self, bytes: u64) -> Self {
        self.memory_bytes = Some(bytes);
        self
    }

    /// Sets memory requirement in gigabytes.
    #[must_use]
    pub fn memory_gb(mut self, gb: u64) -> Self {
        self.memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Sets GPU memory requirement in bytes.
    #[must_use]
    pub fn gpu_memory(mut self, bytes: u64) -> Self {
        self.gpu_memory_bytes = Some(bytes);
        self
    }

    /// Sets GPU memory requirement in gigabytes.
    #[must_use]
    pub fn gpu_memory_gb(mut self, gb: u64) -> Self {
        self.gpu_memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Sets storage requirement in bytes.
    #[must_use]
    pub fn storage(mut self, bytes: u64) -> Self {
        self.storage_bytes = Some(bytes);
        self
    }

    /// Sets storage requirement in gigabytes.
    #[must_use]
    pub fn storage_gb(mut self, gb: u64) -> Self {
        self.storage_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Sets network bandwidth requirement in Mbps.
    #[must_use]
    pub fn network_bandwidth(mut self, mbps: u64) -> Self {
        self.network_bandwidth_mbps = Some(mbps);
        self
    }

    /// Sets estimated duration.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Sets estimated duration in seconds.
    #[must_use]
    pub fn duration_secs(mut self, secs: u64) -> Self {
        self.duration = Some(Duration::from_secs(secs));
        self
    }

    /// Adds a metadata key-value pair.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Builds the graph node.
    pub fn build(self) -> GraphNode {
        let mut requirements = NodeResourceRequirements::default();

        if let Some(cores) = self.cpu_cores {
            requirements.cpu = Some(CpuRequirements {
                min_cores: cores,
                max_cores: None,
                architecture: None,
            });
        }

        if let Some(bytes) = self.memory_bytes {
            requirements.memory = Some(MemoryRequirements {
                min_bytes: bytes,
                max_bytes: None,
            });
        }

        if let Some(bytes) = self.gpu_memory_bytes {
            requirements.gpu = Some(GpuRequirements {
                min_units: 1,
                max_units: None,
                gpu_type: None,
                min_memory_bytes: Some(bytes),
            });
        }

        if let Some(bytes) = self.storage_bytes {
            requirements.storage = Some(StorageRequirements {
                min_bytes: bytes,
                max_bytes: None,
                storage_type: None,
            });
        }

        if let Some(mbps) = self.network_bandwidth_mbps {
            let bytes_per_sec = mbps * 125_000;
            requirements.network = Some(NetworkRequirements {
                min_bandwidth: Some(bytes_per_sec),
                max_bandwidth: None,
                max_latency_ms: None,
            });
        }

        GraphNode {
            id: self.id,
            primal: self.primal,
            operation: self.operation,
            duration: self.duration,
            requirements,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_node_simple() {
        let node = GraphNode::simple("n1", "cpu_compute");
        assert_eq!(node.id, "n1");
        assert_eq!(node.operation, "cpu_compute");
        assert!(node.requirements.cpu.is_none());
        assert!(node.metadata.is_empty());
    }

    #[test]
    fn test_graph_node_builder_minimal() {
        let node = GraphNode::builder("n2", "storage").build();
        assert_eq!(node.id, "n2");
        assert_eq!(node.operation, "storage");
    }

    #[test]
    #[expect(clippy::float_cmp, reason = "exact builder values in test")]
    fn test_graph_node_builder_with_resources() {
        let node = GraphNode::builder("n3", "gpu_compute")
            .cpu(8.0)
            .memory_gb(16)
            .gpu_memory_gb(4)
            .duration_secs(120)
            .metadata("key", "value")
            .build();
        assert_eq!(node.id, "n3");
        assert_eq!(node.operation, "gpu_compute");
        assert_eq!(node.requirements.cpu.as_ref().unwrap().min_cores, 8.0);
        assert_eq!(
            node.requirements.memory.as_ref().unwrap().min_bytes,
            16 * 1024 * 1024 * 1024
        );
        assert_eq!(
            node.requirements
                .gpu
                .as_ref()
                .and_then(|g| g.min_memory_bytes)
                .unwrap(),
            4 * 1024 * 1024 * 1024
        );
        assert_eq!(node.duration, Some(Duration::from_secs(120)));
        assert_eq!(node.metadata.get("key").map(String::as_str), Some("value"));
    }

    #[test]
    fn test_node_resource_requirements_default() {
        let req = NodeResourceRequirements::default();
        assert!(req.cpu.is_none());
        assert!(req.memory.is_none());
        assert!(req.gpu.is_none());
    }

    #[test]
    fn test_graph_node_serialization_roundtrip() {
        let node = GraphNode::simple("serial", "cpu_compute");
        let json = serde_json::to_string(&node).unwrap();
        let restored: GraphNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node.id, restored.id);
        assert_eq!(node.operation, restored.operation);
    }
}
