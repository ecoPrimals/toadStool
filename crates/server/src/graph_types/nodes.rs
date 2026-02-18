//! Graph node types for workflow representation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use toadstool::resources::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
};

/// Graph node representing a single workload unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    #[serde(default = "default_primal")]
    pub primal: String,
    pub operation: String,
    #[serde(default)]
    pub requirements: NodeResourceRequirements,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub duration: Option<Duration>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_primal() -> String {
    "toadstool".to_string()
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuRequirements>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryRequirements>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageRequirements>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuRequirements>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkRequirements>,
}

impl GraphNode {
    pub fn builder(id: impl Into<String>, operation: impl Into<String>) -> GraphNodeBuilder {
        GraphNodeBuilder::new(id, operation)
    }

    pub fn simple(id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            primal: "toadstool".to_string(),
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
    pub fn new(id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            primal: "toadstool".to_string(),
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

    pub fn primal(mut self, primal: impl Into<String>) -> Self {
        self.primal = primal.into();
        self
    }

    pub fn cpu(mut self, cores: f64) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    pub fn memory(mut self, bytes: u64) -> Self {
        self.memory_bytes = Some(bytes);
        self
    }

    pub fn memory_gb(mut self, gb: u64) -> Self {
        self.memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    pub fn gpu_memory(mut self, bytes: u64) -> Self {
        self.gpu_memory_bytes = Some(bytes);
        self
    }

    pub fn gpu_memory_gb(mut self, gb: u64) -> Self {
        self.gpu_memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    pub fn storage(mut self, bytes: u64) -> Self {
        self.storage_bytes = Some(bytes);
        self
    }

    pub fn storage_gb(mut self, gb: u64) -> Self {
        self.storage_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    pub fn network_bandwidth(mut self, mbps: u64) -> Self {
        self.network_bandwidth_mbps = Some(mbps);
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn duration_secs(mut self, secs: u64) -> Self {
        self.duration = Some(Duration::from_secs(secs));
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

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
            let bytes_per_sec = mbps * 125000;
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
