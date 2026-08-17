// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical NUCLEUS Composition Manifest — `biome.yaml`
//!
//! Unified manifest type for all ToadStool subsystems (CLI, daemon, biomeOS,
//! integration-primals). A biome manifest declares a **composition graph**:
//! a set of primals and services wired together with explicit dependencies
//! and grouped into atomic sub-graphs (compositions).
//!
//! ## NUCLEUS Architecture
//!
//! Each atomic composition (Tower, Nest, Node) is a sub-graph with internal
//! dependency ordering. biomeOS graph executor starts compositions, routes
//! through them, and orchestrates multi-step workflows.
//!
//! A primal can appear in **multiple** compositions. Compositions are graphs —
//! primals are nodes — same node, multiple graphs. `biome.yaml` is the
//! composition manifest (BYOB per gate).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Canonical biome manifest — NUCLEUS sub-graph definition.
///
/// All ToadStool subsystems (CLI, daemon, biomeOS graph executor,
/// integration-primals orchestrator) consume this single type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    /// Schema version (e.g. `"v1"`)
    #[serde(default = "default_api_version")]
    pub api_version: String,

    /// Manifest kind — always `"Biome"` for biome manifests
    #[serde(default = "default_kind")]
    pub kind: String,

    /// Biome identity and metadata
    pub metadata: BiomeMetadata,

    /// Primal configurations keyed by primal name
    #[serde(default)]
    pub primals: HashMap<String, ManifestPrimalConfig>,

    /// Service definitions keyed by service name
    #[serde(default)]
    pub services: HashMap<String, ManifestServiceConfig>,

    /// NUCLEUS composition sub-graphs
    #[serde(default)]
    pub compositions: Vec<CompositionGraph>,

    /// Resource limits for the entire biome
    #[serde(default)]
    pub resources: Option<ManifestResources>,

    /// Security policies
    #[serde(default)]
    pub security: Option<ManifestSecurity>,

    /// Network configuration
    #[serde(default)]
    pub networking: Option<ManifestNetworking>,

    /// Storage configuration
    #[serde(default)]
    pub storage: Option<ManifestStorage>,

    /// AI agent deployment configurations
    #[serde(default)]
    pub agents: Option<Vec<ManifestAgentConfig>>,

    /// Federation configuration (cross-gate)
    #[serde(default)]
    pub federation: Option<ManifestFederation>,
}

fn default_api_version() -> String {
    "v1".to_string()
}

fn default_kind() -> String {
    "Biome".to_string()
}

/// Biome metadata — identity, versioning, labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    /// Biome display name
    pub name: String,

    /// Semantic version string
    pub version: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Author or maintainer
    #[serde(default)]
    pub author: Option<String>,

    /// Team or organization
    #[serde(default)]
    pub team: Option<String>,

    /// Deployment environment (dev, staging, prod)
    #[serde(default)]
    pub environment: Option<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    /// Key-value labels (for selector queries)
    #[serde(default)]
    pub labels: HashMap<String, String>,

    /// Annotations (opaque metadata, not used for selection)
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

/// NUCLEUS composition sub-graph.
///
/// A composition groups primals and services into an atomic unit with
/// internal dependency ordering. One primal can appear in multiple
/// compositions across different biome manifests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionGraph {
    /// Composition name (e.g. `"tower-atomic"`, `"nest-storage"`)
    pub name: String,

    /// Composition kind — Tower, Nest, Node, or custom
    #[serde(default = "default_composition_kind")]
    pub kind: CompositionKind,

    /// Primal names included in this composition (must exist in top-level `primals`)
    #[serde(default)]
    pub members: Vec<String>,

    /// Dependency edges between members: `{"songBird": ["swarmVine", "bearDog"]}`
    /// means songBird depends on swarmVine and bearDog starting first.
    #[serde(default)]
    pub dependencies: HashMap<String, Vec<String>>,

    /// Whether this composition should start automatically
    #[serde(default = "default_true")]
    pub auto_start: bool,

    /// Start order priority (lower = starts first)
    #[serde(default)]
    pub priority: u32,

    /// Health check requirements before composition is considered ready
    #[serde(default)]
    pub readiness: Option<CompositionReadiness>,
}

fn default_composition_kind() -> CompositionKind {
    CompositionKind::Custom
}

fn default_true() -> bool {
    true
}

/// Atomic composition types in the NUCLEUS architecture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompositionKind {
    /// Tower Atomic — core infrastructure primals
    Tower,
    /// Nest Atomic — storage and data federation
    Nest,
    /// Node Atomic — compute dispatch and silicon
    Node,
    /// Custom composition
    Custom,
}

/// Readiness criteria for a composition sub-graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionReadiness {
    /// All listed members must pass health checks
    #[serde(default)]
    pub require_healthy: Vec<String>,

    /// Timeout before marking composition as failed (seconds)
    #[serde(default = "default_readiness_timeout")]
    pub timeout_secs: u64,
}

fn default_readiness_timeout() -> u64 {
    120
}

/// Primal configuration within a biome manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPrimalConfig {
    /// Primal version
    #[serde(default)]
    pub version: Option<String>,

    /// Whether the primal is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Workload source specification
    #[serde(default)]
    pub source: Option<ManifestWorkloadSource>,

    /// Arbitrary primal-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,

    /// Declared capabilities (e.g. `["compute.dispatch", "shader.compile"]`)
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Primal names this depends on
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Health check configuration
    #[serde(default)]
    pub health_check: Option<ManifestHealthCheck>,

    /// Resource requirements specific to this primal
    #[serde(default)]
    pub resources: Option<ManifestResources>,

    /// Gossip injection points — events this primal announces to swarmVine
    #[serde(default)]
    pub gossip_events: Vec<String>,
}

/// Source for loading a workload (container, WASM, native binary, git, local).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ManifestWorkloadSource {
    /// OCI container image
    Container {
        /// Image name (e.g. `"toadstool"`)
        image: String,
        /// Image tag
        #[serde(default = "default_latest")]
        tag: String,
        /// Registry host
        #[serde(default)]
        registry: Option<String>,
        /// Digest for pinning
        #[serde(default)]
        digest: Option<String>,
    },
    /// WebAssembly module
    Wasm {
        /// Path or URL to the WASM module
        source: String,
        /// Content hash for verification
        #[serde(default)]
        checksum: Option<String>,
        /// WASI runtime configuration
        #[serde(default)]
        wasi_config: HashMap<String, serde_json::Value>,
    },
    /// Native binary
    Native {
        /// Path to the binary (resolved from depot or local)
        path: String,
        /// Command-line arguments
        #[serde(default)]
        args: Vec<String>,
    },
    /// Git repository
    Git {
        /// Repository URL
        repository: String,
        /// Branch
        #[serde(default)]
        branch: Option<String>,
        /// Commit hash or tag
        #[serde(default)]
        commit: Option<String>,
        /// Subpath within the repo
        #[serde(default)]
        path: Option<String>,
    },
    /// Local filesystem path
    Local {
        /// Path to workload file or directory
        path: String,
    },
}

fn default_latest() -> String {
    "latest".to_string()
}

/// Service configuration within a biome manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestServiceConfig {
    /// Service version
    #[serde(default)]
    pub version: Option<String>,

    /// Workload source
    #[serde(default)]
    pub source: Option<ManifestWorkloadSource>,

    /// Number of replicas
    #[serde(default = "default_one")]
    pub replicas: u32,

    /// Resource limits
    #[serde(default)]
    pub resources: Option<ManifestResources>,

    /// Environment variables
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// Port mappings
    #[serde(default)]
    pub ports: Vec<ManifestPort>,

    /// Volume mounts
    #[serde(default)]
    pub volumes: Vec<ManifestVolume>,

    /// Service names this depends on
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Health check
    #[serde(default)]
    pub health_check: Option<ManifestHealthCheck>,
}

fn default_one() -> u32 {
    1
}

/// Resource limits and requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestResources {
    /// CPU cores limit
    #[serde(default)]
    pub cpu_limit: Option<f64>,

    /// Memory limit (e.g. `"512Mi"`, `"2Gi"`)
    #[serde(default)]
    pub memory_limit: Option<String>,

    /// Storage limit
    #[serde(default)]
    pub storage_limit: Option<String>,

    /// GPU count limit
    #[serde(default)]
    pub gpu_limit: Option<u32>,
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSecurity {
    /// Isolation level (`"process"`, `"container"`, `"vm"`)
    #[serde(default = "default_isolation")]
    pub isolation_level: String,

    /// Trust level (`"low"`, `"medium"`, `"high"`)
    #[serde(default = "default_trust")]
    pub trust_level: String,

    /// Whether a crypto provider (bearDog) is required
    #[serde(default)]
    pub crypto_required: bool,

    /// Crypto policy names to apply
    #[serde(default)]
    pub crypto_policies: Vec<String>,

    /// Allowed network CIDRs
    #[serde(default)]
    pub allowed_networks: Vec<String>,
}

fn default_isolation() -> String {
    "process".to_string()
}

fn default_trust() -> String {
    "medium".to_string()
}

/// Network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestNetworking {
    /// Network mode (`"bridge"`, `"host"`, `"none"`)
    #[serde(default = "default_bridge")]
    pub mode: String,

    /// DNS server addresses
    #[serde(default)]
    pub dns_servers: Vec<String>,

    /// Port mappings
    #[serde(default)]
    pub port_mappings: Vec<ManifestPort>,
}

fn default_bridge() -> String {
    "bridge".to_string()
}

/// Storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestStorage {
    /// Storage service integration
    #[serde(default)]
    pub integration: Option<String>,

    /// Volume definitions
    #[serde(default)]
    pub volumes: Vec<ManifestVolume>,

    /// Backup policy
    #[serde(default)]
    pub backup_policy: Option<String>,
}

/// Port mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPort {
    /// Port inside the container/process
    pub container_port: u16,

    /// Host port (defaults to container_port)
    #[serde(default)]
    pub host_port: Option<u16>,

    /// Protocol (`"tcp"`, `"udp"`)
    #[serde(default = "default_tcp")]
    pub protocol: String,
}

fn default_tcp() -> String {
    "tcp".to_string()
}

/// Volume mount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestVolume {
    /// Source path or volume name
    pub source: String,

    /// Mount target path
    pub target: String,

    /// Whether the mount is read-only
    #[serde(default)]
    pub read_only: bool,
}

/// Health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestHealthCheck {
    /// Command to run for health check
    #[serde(default)]
    pub command: Vec<String>,

    /// Interval between checks (seconds)
    #[serde(default = "default_30")]
    pub interval_secs: u64,

    /// Timeout per check (seconds)
    #[serde(default = "default_5")]
    pub timeout_secs: u64,

    /// Consecutive failures before unhealthy
    #[serde(default = "default_3")]
    pub retries: u32,
}

fn default_30() -> u64 {
    30
}

fn default_5() -> u64 {
    5
}

fn default_3() -> u32 {
    3
}

/// AI agent deployment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestAgentConfig {
    /// Agent name
    pub name: String,

    /// Agent type/model
    #[serde(default)]
    pub agent_type: Option<String>,

    /// Agent-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Federation configuration for cross-gate deployments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFederation {
    /// Whether this biome participates in federation
    #[serde(default)]
    pub enabled: bool,

    /// Peer gate names
    #[serde(default)]
    pub peers: Vec<String>,

    /// Replication strategy
    #[serde(default)]
    pub replication: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let yaml = r#"
metadata:
  name: test-biome
  version: "1.0.0"
"#;
        let manifest: BiomeManifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(manifest.metadata.name, "test-biome");
        assert_eq!(manifest.api_version, "v1");
        assert_eq!(manifest.kind, "Biome");
        assert!(manifest.primals.is_empty());
        assert!(manifest.compositions.is_empty());
    }

    #[test]
    fn parse_composition_graph() {
        let yaml = r#"
api_version: v1
kind: Biome
metadata:
  name: strandgate-tower
  version: "157e"
  environment: production
  team: ecoPrimals
primals:
  toadstool:
    capabilities: ["compute.dispatch", "silicon.discover"]
    gossip_events: ["hardware.gpu.added", "hardware.gpu.removed"]
  coralreef:
    capabilities: ["shader.compile"]
    dependencies: ["toadstool"]
  barracuda:
    capabilities: ["tensor.compute"]
    dependencies: ["toadstool", "coralreef"]
compositions:
  - name: node-atomic
    kind: Node
    members: ["toadstool", "coralreef", "barracuda"]
    dependencies:
      coralreef: ["toadstool"]
      barracuda: ["toadstool", "coralreef"]
    priority: 0
  - name: tower-atomic
    kind: Tower
    members: ["songbird", "biomeos", "swarmvine"]
    auto_start: true
"#;
        let manifest: BiomeManifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(manifest.metadata.name, "strandgate-tower");
        assert_eq!(manifest.compositions.len(), 2);
        assert_eq!(manifest.compositions[0].name, "node-atomic");
        assert_eq!(manifest.compositions[0].kind, CompositionKind::Node);
        assert_eq!(manifest.compositions[0].members.len(), 3);
        let deps = &manifest.compositions[0].dependencies;
        assert_eq!(deps["barracuda"], vec!["toadstool", "coralreef"]);
        assert_eq!(manifest.primals.len(), 3);
        let ts = &manifest.primals["toadstool"];
        assert_eq!(
            ts.gossip_events,
            vec!["hardware.gpu.added", "hardware.gpu.removed"]
        );
    }

    #[test]
    fn roundtrip_json() {
        let manifest = BiomeManifest {
            api_version: "v1".into(),
            kind: "Biome".into(),
            metadata: BiomeMetadata {
                name: "test".into(),
                version: "1.0".into(),
                description: None,
                author: None,
                team: None,
                environment: None,
                tags: vec![],
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            primals: HashMap::new(),
            services: HashMap::new(),
            compositions: vec![],
            resources: None,
            security: None,
            networking: None,
            storage: None,
            agents: None,
            federation: None,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: BiomeManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.metadata.name, "test");
    }
}
