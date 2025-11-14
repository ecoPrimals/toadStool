use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::Hash;
use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    pub created_at: DateTime<Utc>,
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
    pub created_at: DateTime<Utc>,
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
    #[must_use]
    pub fn to_mode_string(&self) -> String {
        match self {
            Self::Native => "native".to_string(),
            Self::Container => "container".to_string(),
            Self::Emulated => "emulated".to_string(),
            Self::Hybrid => "hybrid".to_string(),
            Self::LinuxCompat => "linux_compat".to_string(),
            Self::WindowsCompat => "windows_compat".to_string(),
            Self::MacOSCompat => "macos_compat".to_string(),
            Self::ContainerCompat => "container_compat".to_string(),
            Self::LegacyCompat { system_type } => {
                format!("legacy_compat_{system_type}")
            }
        }
    }
}
