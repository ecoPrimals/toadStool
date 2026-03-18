// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::Hash;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use toadstool::{ExecutionRequest, ToadStoolError, ToadStoolResult};
// Re-export canonical JobPriority for convenience
pub use toadstool::JobPriority;

use super::resources::{DistributedRetryConfig, ResourceConstraints, ResourceRequirements};

/// Universal job for cross-platform execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalJob {
    /// Job identification
    pub job_id: Uuid,
    /// Job type (optional for auto-detection)
    pub job_type: Option<UniversalJobType>,
    /// Execution request
    pub execution_request: ExecutionRequest,
    /// Target destination
    pub target: ExecutionTarget,
    /// Priority level
    pub priority: JobPriority,
    /// Dependencies
    pub dependencies: Vec<Uuid>,
    /// Resource requirements
    pub resource_requirements: ResourceRequirements,
    /// Retry configuration
    pub retry_config: DistributedRetryConfig,
    /// Created timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
}

/// Universal job types for different execution scenarios
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UniversalJobType {
    /// Local execution
    Local,
    /// Remote `ToadStool` execution
    RemoteToadStool { endpoint: String },
    /// Ecosystem tool execution
    EcosystemTool { tool_name: String, endpoint: String },
    /// Recursive `ToadStool` hosting
    RecursiveHosting {
        toadstool_config: ToadStoolHostingConfig,
    },
    /// OS-layer compatibility execution
    OSLayerCompatibility {
        compatibility_mode: CompatibilityMode,
    },

    // Job classification types for distributed scheduling
    /// CPU-intensive computational work
    ComputeIntensive,
    /// Memory-intensive workloads
    MemoryIntensive,
    /// Network-intensive workloads
    NetworkIntensive,
    /// Storage-intensive workloads
    StorageIntensive,
    /// Hybrid workloads combining multiple resource types
    Hybrid,
    /// Data processing and analytics
    DataProcessing,
    /// Machine learning and AI workloads
    MachineLearning,
    /// Scientific simulations
    Simulation,
    /// Native execution
    Native,
    /// Container-based execution
    Container,
    /// WebAssembly execution
    WASM,
    /// GPU-accelerated execution
    GPU,
    /// Custom workload type
    Custom(String),
}

impl FromStr for UniversalJobType {
    type Err = ToadStoolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "compute_intensive" => Ok(Self::ComputeIntensive),
            "memory_intensive" => Ok(Self::MemoryIntensive),
            "network_intensive" => Ok(Self::NetworkIntensive),
            "storage_intensive" => Ok(Self::StorageIntensive),
            "hybrid" => Ok(Self::Hybrid),
            "data_processing" => Ok(Self::DataProcessing),
            "machine_learning" => Ok(Self::MachineLearning),
            "simulation" => Ok(Self::Simulation),
            "native" => Ok(Self::Native),
            "container" => Ok(Self::Container),
            "wasm" => Ok(Self::WASM),
            "gpu" => Ok(Self::GPU),
            _ => Ok(Self::Custom(s.to_string())),
        }
    }
}

/// Execution target for job placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionTarget {
    /// Execute locally
    Local,
    /// Execute on specific `ToadStool` instance
    ToadStool {
        instance_id: String,
        endpoint: String,
    },
    /// Execute on ecosystem service
    EcosystemService {
        service_name: String,
        endpoint: String,
    },
    /// Execute on best available resource
    BestAvailable { constraints: ResourceConstraints },
    /// Execute with load balancing
    LoadBalanced { strategy: LoadBalancingStrategy },
}

// JobPriority is now imported from toadstool core (canonical definition in universal.rs)

/// Universal job queue for managing multiple job types
#[derive(Debug)]
pub struct UniversalJobQueue {
    /// Priority queues for different job types
    priority_queues: BTreeMap<JobPriority, VecDeque<UniversalJob>>,
    /// Dependency graph for job ordering
    dependency_graph: DependencyGraph,
    /// Job metadata storage
    job_metadata: HashMap<Uuid, JobMetadata>,
    /// Resource requirements index
    resource_index: ResourceRequirementIndex,
}

/// Dependency graph for job ordering
#[derive(Debug)]
pub struct DependencyGraph {
    graph: HashMap<Uuid, Vec<Uuid>>,
    reverse_graph: HashMap<Uuid, Vec<Uuid>>,
}

/// Job metadata for tracking and analytics
#[derive(Debug, Clone)]
pub struct JobMetadata {
    pub job_id: Uuid,
    pub job_type: UniversalJobType,
    pub created_at: SystemTime,
    pub priority: JobPriority,
    pub estimated_duration: Option<Duration>,
}

/// Resource requirement index for efficient job matching
#[derive(Debug)]
pub struct ResourceRequirementIndex {
    cpu_index: HashMap<Uuid, f64>,
    memory_index: HashMap<Uuid, u64>,
    gpu_jobs: Vec<Uuid>,
}

/// Load balancing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin { weights: HashMap<String, u32> },
    ResourceAware,
    LatencyBased,
}

/// Compatibility mode for execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CompatibilityMode {
    Native,
    Container,
    Emulated,
    Hybrid,
    LinuxCompat,
    WindowsCompat,
    MacOSCompat,
    ContainerCompat,
    LegacyCompat { system_type: String },
}

/// Configuration for `ToadStool` hosting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToadStoolHostingConfig {
    /// Enable hosting
    pub enabled: bool,
    /// Hosting mode
    pub mode: String,
    /// Resource limits
    pub resource_limits: HashMap<String, u64>,
    /// Security settings
    pub security_settings: HashMap<String, String>,
    /// Resource allocation
    pub resource_allocation: Option<crate::types::resources::ResourceAllocation>,
}

impl Hash for ToadStoolHostingConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.enabled.hash(state);
        self.mode.hash(state);

        // Hash resource_limits as sorted vector
        let mut resource_limits: Vec<_> = self.resource_limits.iter().collect();
        resource_limits.sort_by_key(|&(k, _)| k);
        for (k, v) in resource_limits {
            k.hash(state);
            v.hash(state);
        }

        // Hash security_settings as sorted vector
        let mut security_settings: Vec<_> = self.security_settings.iter().collect();
        security_settings.sort_by_key(|&(k, _)| k);
        for (k, v) in security_settings {
            k.hash(state);
            v.hash(state);
        }

        // Hash resource_allocation
        if let Some(ref allocation) = self.resource_allocation {
            allocation.hash(state);
        }
    }
}

impl Default for UniversalJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalJobQueue {
    #[must_use]
    pub fn new() -> Self {
        Self {
            priority_queues: BTreeMap::new(),
            dependency_graph: DependencyGraph::new(),
            job_metadata: HashMap::new(),
            resource_index: ResourceRequirementIndex::new(),
        }
    }

    pub async fn add_job(&mut self, job: UniversalJob) -> ToadStoolResult<()> {
        let job_id = job.job_id;
        let dependencies = job.dependencies.clone();

        self.dependency_graph.add_job(job_id, dependencies)?;

        let metadata = JobMetadata::from_job(&job);
        self.job_metadata.insert(job_id, metadata);

        // Add to resource index
        self.resource_index
            .add_job(job_id, job.resource_requirements)?;

        Ok(())
    }

    #[must_use]
    pub fn total_jobs(&self) -> usize {
        self.priority_queues
            .values()
            .map(std::collections::VecDeque::len)
            .sum()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
            reverse_graph: HashMap::new(),
        }
    }

    pub fn add_job(&mut self, job_id: Uuid, dependencies: Vec<Uuid>) -> ToadStoolResult<()> {
        self.graph.insert(job_id, dependencies.clone());

        for dep in dependencies {
            self.reverse_graph.entry(dep).or_default().push(job_id);
        }

        Ok(())
    }
}

impl JobMetadata {
    #[must_use]
    pub fn from_job(job: &UniversalJob) -> Self {
        Self {
            job_id: job.job_id,
            job_type: job.job_type.clone().unwrap_or(UniversalJobType::Local),
            created_at: job.created_at,
            priority: job.priority,
            estimated_duration: None,
        }
    }
}

impl Default for ResourceRequirementIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceRequirementIndex {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cpu_index: HashMap::new(),
            memory_index: HashMap::new(),
            gpu_jobs: Vec::new(),
        }
    }

    pub fn add_job(
        &mut self,
        job_id: Uuid,
        requirements: ResourceRequirements,
    ) -> ToadStoolResult<()> {
        self.cpu_index.insert(job_id, requirements.cpu.min_cores);
        self.memory_index
            .insert(job_id, requirements.memory.min_bytes);

        if requirements.gpu.is_some() {
            self.gpu_jobs.push(job_id);
        }

        Ok(())
    }
}

impl CompatibilityMode {
    /// Get mode as string (zero-copy for standard modes)
    ///
    /// Returns a static string for standard modes, only allocates for LegacyCompat
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Container => "container",
            Self::Emulated => "emulated",
            Self::Hybrid => "hybrid",
            Self::LinuxCompat => "linux_compat",
            Self::WindowsCompat => "windows_compat",
            Self::MacOSCompat => "macos_compat",
            Self::ContainerCompat => "container_compat",
            Self::LegacyCompat { .. } => "legacy_compat", // Generic for legacy
        }
    }

    // ✅ REMOVED: to_mode_string() - deprecated since 0.1.0
    // Use as_str() instead to avoid allocation, or call .to_string() on as_str() if ownership is needed
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::SystemTime;
    use uuid::Uuid;

    fn make_universal_job(
        job_type: Option<UniversalJobType>,
        target: ExecutionTarget,
        priority: JobPriority,
    ) -> UniversalJob {
        UniversalJob {
            job_id: Uuid::new_v4(),
            job_type,
            execution_request: toadstool::ExecutionRequest::default(),
            target,
            priority,
            dependencies: vec![],
            resource_requirements: ResourceRequirements::default(),
            retry_config: DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        }
    }

    #[test]
    fn universal_job_type_from_str_standard_variants() {
        assert!(matches!(
            UniversalJobType::from_str("local").unwrap(),
            UniversalJobType::Local
        ));
        assert!(matches!(
            UniversalJobType::from_str("compute_intensive").unwrap(),
            UniversalJobType::ComputeIntensive
        ));
        assert!(matches!(
            UniversalJobType::from_str("gpu").unwrap(),
            UniversalJobType::GPU
        ));
        assert!(matches!(
            UniversalJobType::from_str("WASM").unwrap(),
            UniversalJobType::WASM
        ));
    }

    #[test]
    fn universal_job_type_from_str_custom() {
        let custom = UniversalJobType::from_str("my_custom_type").unwrap();
        match &custom {
            UniversalJobType::Custom(s) => assert_eq!(s, "my_custom_type"),
            _ => panic!("expected Custom variant"),
        }
    }

    #[test]
    fn resource_requirements_default_values() {
        let req = ResourceRequirements::default();
        assert_eq!(req.cpu.min_cores, 1.0);
        assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
        assert!(req.gpu.is_none());
    }

    #[test]
    fn job_priority_ordering() {
        use std::cmp::Ordering;
        // Higher priority = lower Ord value (Emergency < Normal)
        assert!(JobPriority::Emergency < JobPriority::Normal);
        assert!(JobPriority::High < JobPriority::Low);
        assert_eq!(
            JobPriority::Normal.cmp(&JobPriority::Normal),
            Ordering::Equal
        );
    }

    #[test]
    fn universal_job_creation_with_variants() {
        let _job_local = make_universal_job(
            Some(UniversalJobType::Local),
            ExecutionTarget::Local,
            JobPriority::Normal,
        );
        let _job_best_available = make_universal_job(
            Some(UniversalJobType::GPU),
            ExecutionTarget::BestAvailable {
                constraints: ResourceConstraints {
                    max_cpu_cores: Some(8.0),
                    max_memory_bytes: None,
                    required_features: vec![],
                    excluded_nodes: vec![],
                },
            },
            JobPriority::High,
        );
    }

    #[test]
    fn execution_target_variants() {
        let _local = ExecutionTarget::Local;
        let _toadstool = ExecutionTarget::ToadStool {
            instance_id: "inst-1".to_string(),
            endpoint: toadstool_common::constants::network::default_http_url(),
        };
        let _best = ExecutionTarget::BestAvailable {
            constraints: ResourceConstraints {
                max_cpu_cores: Some(8.0),
                max_memory_bytes: Some(16 * 1024 * 1024 * 1024),
                required_features: vec!["gpu".to_string()],
                excluded_nodes: vec![],
            },
        };
    }

    #[test]
    fn compatibility_mode_as_str() {
        assert_eq!(CompatibilityMode::Native.as_str(), "native");
        assert_eq!(CompatibilityMode::Container.as_str(), "container");
        assert_eq!(
            CompatibilityMode::LegacyCompat {
                system_type: "old".to_string()
            }
            .as_str(),
            "legacy_compat"
        );
    }

    #[test]
    fn load_balancing_strategy_construction() {
        let _rr = LoadBalancingStrategy::RoundRobin;
        let _lc = LoadBalancingStrategy::LeastConnections;
        let mut weights = HashMap::new();
        weights.insert("a".to_string(), 1);
        let _wrr = LoadBalancingStrategy::WeightedRoundRobin { weights };
    }

    #[test]
    fn universal_job_queue_new_and_default() {
        let queue = UniversalJobQueue::new();
        assert_eq!(queue.total_jobs(), 0);
        let default_queue = UniversalJobQueue::default();
        assert_eq!(default_queue.total_jobs(), 0);
    }

    #[test]
    fn dependency_graph_add_job() {
        let mut graph = DependencyGraph::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        graph.add_job(id1, vec![]).expect("add root");
        graph.add_job(id2, vec![id1]).expect("add dependent");
    }

    #[test]
    fn job_metadata_from_job() {
        let job = make_universal_job(
            Some(UniversalJobType::Local),
            ExecutionTarget::Local,
            JobPriority::Normal,
        );
        let meta = JobMetadata::from_job(&job);
        assert_eq!(meta.job_id, job.job_id);
        assert_eq!(meta.priority, job.priority);
    }

    #[test]
    fn resource_requirement_index_add_job() {
        let mut index = ResourceRequirementIndex::new();
        let job = make_universal_job(
            Some(UniversalJobType::Local),
            ExecutionTarget::Local,
            JobPriority::Normal,
        );
        index
            .add_job(job.job_id, job.resource_requirements)
            .expect("add job");
    }

    #[test]
    fn toadstool_hosting_config_construction() {
        let config = ToadStoolHostingConfig {
            enabled: true,
            mode: "standalone".to_string(),
            resource_limits: HashMap::new(),
            security_settings: HashMap::new(),
            resource_allocation: None,
        };
        assert!(config.enabled);
        assert_eq!(config.mode, "standalone");
    }

    #[test]
    fn universal_job_serde_roundtrip() {
        let job = make_universal_job(
            Some(UniversalJobType::Local),
            ExecutionTarget::Local,
            JobPriority::Normal,
        );
        let json = serde_json::to_string(&job).unwrap();
        let parsed: UniversalJob = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.job_id, job.job_id);
    }

    #[test]
    fn execution_target_serde_roundtrip() {
        let targets = [
            ExecutionTarget::Local,
            ExecutionTarget::ToadStool {
                instance_id: "i1".to_string(),
                endpoint: "http://localhost:8080".to_string(),
            },
            ExecutionTarget::EcosystemService {
                service_name: "svc".to_string(),
                endpoint: "http://svc:8080".to_string(),
            },
        ];
        for t in targets {
            let json = serde_json::to_string(&t).unwrap();
            let _: ExecutionTarget = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn load_balancing_strategy_serde_roundtrip() {
        let mut weights = HashMap::new();
        weights.insert("a".to_string(), 2);
        let strategies = [
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::LeastConnections,
            LoadBalancingStrategy::WeightedRoundRobin { weights },
            LoadBalancingStrategy::ResourceAware,
            LoadBalancingStrategy::LatencyBased,
        ];
        for s in strategies {
            let json = serde_json::to_string(&s).unwrap();
            let _: LoadBalancingStrategy = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn compatibility_mode_serde_roundtrip() {
        for mode in [
            CompatibilityMode::Native,
            CompatibilityMode::Container,
            CompatibilityMode::Emulated,
            CompatibilityMode::Hybrid,
            CompatibilityMode::LinuxCompat,
            CompatibilityMode::WindowsCompat,
            CompatibilityMode::MacOSCompat,
            CompatibilityMode::ContainerCompat,
            CompatibilityMode::LegacyCompat {
                system_type: "old".to_string(),
            },
        ] {
            let json = serde_json::to_string(&mode).unwrap();
            let _: CompatibilityMode = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn toadstool_hosting_config_serde_roundtrip() {
        let mut limits = HashMap::new();
        limits.insert("cpu".to_string(), 8);
        let config = ToadStoolHostingConfig {
            enabled: true,
            mode: "standalone".to_string(),
            resource_limits: limits,
            security_settings: HashMap::new(),
            resource_allocation: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ToadStoolHostingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mode, config.mode);
    }

    #[test]
    fn universal_job_type_serde_roundtrip() {
        let types = [
            UniversalJobType::Local,
            UniversalJobType::RemoteToadStool {
                endpoint: "http://x:8080".to_string(),
            },
            UniversalJobType::EcosystemTool {
                tool_name: "t".to_string(),
                endpoint: "http://t:8080".to_string(),
            },
            UniversalJobType::ComputeIntensive,
            UniversalJobType::GPU,
            UniversalJobType::Custom("custom".to_string()),
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let _: UniversalJobType = serde_json::from_str(&json).unwrap();
        }
    }

    #[tokio::test]
    async fn universal_job_queue_add_job() {
        let mut queue = UniversalJobQueue::new();
        let job = make_universal_job(
            Some(UniversalJobType::Local),
            ExecutionTarget::Local,
            JobPriority::Normal,
        );
        queue.add_job(job).await.expect("add job");
    }

    #[test]
    fn execution_target_best_available_serde() {
        let target = ExecutionTarget::BestAvailable {
            constraints: ResourceConstraints {
                max_cpu_cores: Some(16.0),
                max_memory_bytes: Some(32 * 1024 * 1024 * 1024),
                required_features: vec!["gpu".to_string()],
                excluded_nodes: vec![],
            },
        };
        let json = serde_json::to_string(&target).unwrap();
        let _: ExecutionTarget = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn execution_target_load_balanced_serde() {
        let target = ExecutionTarget::LoadBalanced {
            strategy: LoadBalancingStrategy::RoundRobin,
        };
        let json = serde_json::to_string(&target).unwrap();
        let _: ExecutionTarget = serde_json::from_str(&json).unwrap();
    }
}
