// SPDX-License-Identifier: AGPL-3.0-only
//! Priority queues, dependency graphs, and resource indexes for universal jobs.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::SystemTime;

use toadstool::JobPriority;
use toadstool::ToadStoolResult;
use uuid::Uuid;

use super::universal_job::{UniversalJob, UniversalJobType};
use crate::types::resources::ResourceRequirements;

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

/// Job metadata for tracking and analytics.
#[derive(Debug, Clone)]
pub struct JobMetadata {
    /// Job identifier.
    pub job_id: Uuid,
    /// Job type for scheduling decisions.
    pub job_type: UniversalJobType,
    /// Creation timestamp.
    pub created_at: SystemTime,
    /// Priority for queue ordering.
    pub priority: JobPriority,
    /// Estimated duration for scheduling (optional).
    pub estimated_duration: Option<std::time::Duration>,
}

/// Resource requirement index for efficient job matching
#[derive(Debug)]
pub struct ResourceRequirementIndex {
    cpu_index: HashMap<Uuid, f64>,
    memory_index: HashMap<Uuid, u64>,
    gpu_jobs: Vec<Uuid>,
}

impl Default for UniversalJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalJobQueue {
    /// Creates an empty job queue with default dependency graph and resource index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            priority_queues: BTreeMap::new(),
            dependency_graph: DependencyGraph::new(),
            job_metadata: HashMap::new(),
            resource_index: ResourceRequirementIndex::new(),
        }
    }

    /// Adds a job to the queue, resolving dependencies and resource index.
    pub async fn add_job(&mut self, job: UniversalJob) -> ToadStoolResult<()> {
        let job_id = job.job_id;
        let dependencies = job.dependencies.clone();

        self.dependency_graph.add_job(job_id, dependencies)?;

        let metadata = JobMetadata::from_job(&job);
        self.job_metadata.insert(job_id, metadata);

        self.resource_index
            .add_job(job_id, job.resource_requirements)?;

        Ok(())
    }

    /// Returns the total number of jobs in all priority queues.
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
    /// Creates an empty dependency graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
            reverse_graph: HashMap::new(),
        }
    }

    /// Adds a job and its dependencies to the graph.
    pub fn add_job(&mut self, job_id: Uuid, dependencies: Vec<Uuid>) -> ToadStoolResult<()> {
        self.graph.insert(job_id, dependencies.clone());

        for dep in dependencies {
            self.reverse_graph.entry(dep).or_default().push(job_id);
        }

        Ok(())
    }
}

impl JobMetadata {
    /// Builds metadata from a universal job.
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
    /// Creates an empty resource requirement index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cpu_index: HashMap::new(),
            memory_index: HashMap::new(),
            gpu_jobs: Vec::new(),
        }
    }

    /// Indexes a job by its resource requirements for matching.
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
