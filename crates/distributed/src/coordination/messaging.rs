// SPDX-License-Identifier: AGPL-3.0-or-later
//! Job analysis, distribution strategy, and Coordination request construction for the integration.

use bytes::Bytes;

use toadstool::error::ToadStoolResult;
use tracing::info;

use crate::UniversalJob;

use super::types::{
    CoordinationJobRequest, JobAnalysis, JobComplexity, JobDistributionStrategy, SubTask,
    SubTaskHandle, ToadStoolCoordinationIntegration,
};

impl ToadStoolCoordinationIntegration {
    /// Submit a job for execution: analyse complexity, choose strategy, and dispatch.
    ///
    /// - **Simple jobs** that fit local capacity go to `workload_scheduler`.
    /// - **Moderate/Complex** jobs get split and forwarded via Coordination.
    /// - **UltraMassive** jobs are fully distributed across the Coordination ecosystem.
    pub async fn submit_job(&self, job: UniversalJob) -> ToadStoolResult<Vec<SubTaskHandle>> {
        let analysis = self.analyze_job_for_distribution(&job).await?;
        info!(
            instance_id = %self.instance_id,
            job_id = %job.job_id,
            complexity = ?analysis.complexity,
            strategy = ?analysis.distribution_strategy,
            subtasks = analysis.estimated_subtasks,
            "dispatching job"
        );

        match analysis.distribution_strategy {
            JobDistributionStrategy::LocalOnly => {
                // Schedule directly on this primal without touching Coordination.
                self.workload_scheduler.schedule_job(job).await?;
                Ok(vec![])
            }
            JobDistributionStrategy::LoadBalanced
            | JobDistributionStrategy::CoordinationEcosystem
            | JobDistributionStrategy::ReplicateAcrossNodes
            | JobDistributionStrategy::HybridExecution => {
                // Single-task dispatch: let Coordination's internal scheduler choose the node.
                let req = self.create_coordination_job_request(&job)?;
                let subtask = super::types::SubTask {
                    id: req.job_id,
                    payload: req.job_payload.clone(), // Bytes::clone = refcount bump
                    resource_requirements: req.resource_requirements.clone(),
                    priority: req.priority,
                    constraints: req.constraints.clone(),
                };
                let handle = self
                    .submit_subtask_to_coordination(subtask, req.target_nodes)
                    .await?;
                Ok(vec![handle])
            }
            JobDistributionStrategy::SplitAndDistribute
            | JobDistributionStrategy::MassiveDistribution => {
                // Multi-task dispatch: create one subtask per partition and fan out.
                let req = self.create_coordination_job_request(&job)?;
                let subtask_count = analysis.estimated_subtasks.max(1);
                let per_cpu = req.resource_requirements.cpu.min_cores / subtask_count as f64;
                let per_mem = req.resource_requirements.memory.min_bytes / subtask_count as u64;
                let partitioned: Vec<(SubTask, Vec<String>)> = (0..subtask_count)
                    .map(|i| {
                        let mut st_req = req.resource_requirements.clone();
                        st_req.cpu.min_cores = per_cpu;
                        st_req.memory.min_bytes = per_mem;
                        let suffix =
                            Bytes::from(format!("{{\"partition\":{i},\"total\":{subtask_count}}}"));
                        let payload =
                            Bytes::from([req.job_payload.as_ref(), suffix.as_ref()].concat());
                        (
                            super::types::SubTask {
                                id: uuid::Uuid::new_v4(),
                                payload,
                                resource_requirements: st_req,
                                priority: req.priority,
                                constraints: req.constraints.clone(),
                            },
                            vec![], // Coordination resolves target nodes
                        )
                    })
                    .collect();
                self.distribute_job_subtasks(&job, partitioned).await
            }
        }
    }

    /// Analyze job to determine optimal distribution strategy
    pub(super) async fn analyze_job_for_distribution(
        &self,
        job: &UniversalJob,
    ) -> ToadStoolResult<JobAnalysis> {
        let complexity = self.analyze_job_complexity(job).await?;
        let local_capacity = self.local_capacity.get_available_capacity().await?;

        let distribution_strategy = match &complexity {
            JobComplexity::Simple => {
                // Can execute locally if we have capacity
                if local_capacity.can_handle_job(job) {
                    JobDistributionStrategy::LocalOnly
                } else {
                    JobDistributionStrategy::CoordinationEcosystem
                }
            }
            JobComplexity::Moderate => {
                // Use load balancing across available nodes
                JobDistributionStrategy::LoadBalanced
            }
            JobComplexity::Complex => JobDistributionStrategy::SplitAndDistribute,
            JobComplexity::UltraMassive => JobDistributionStrategy::MassiveDistribution,
        };

        Ok(JobAnalysis {
            complexity: complexity.clone(),
            distribution_strategy,
            estimated_subtasks: self.estimate_subtask_count(job, &complexity).await?,
            resource_requirements: job.resource_requirements.clone(),
            preferred_node_types: vec!["universal".to_owned()],
        })
    }

    /// Distribute job subtasks to multiple ToadStool instances
    pub(super) async fn distribute_job_subtasks(
        &self,
        _job: &UniversalJob,
        subtasks: Vec<(SubTask, Vec<String>)>,
    ) -> ToadStoolResult<Vec<SubTaskHandle>> {
        let mut handles = Vec::new();

        // Fix the async closure issue by using a for loop instead of map
        for (subtask, target_nodes) in subtasks {
            let handle = self
                .submit_subtask_to_coordination(subtask, target_nodes)
                .await?;
            handles.push(handle);
        }

        Ok(handles)
    }

    /// Create Coordination job request from Universal job
    fn create_coordination_job_request(
        &self,
        job: &UniversalJob,
    ) -> ToadStoolResult<CoordinationJobRequest> {
        let job_request = CoordinationJobRequest {
            job_id: job.job_id,
            job_payload: Bytes::from(
                serde_json::to_vec(&job.execution_request)
                    .map_err(|e| toadstool::error::ToadStoolError::validation(e.to_string()))?,
            ),
            target_nodes: vec![], // Will be determined by Coordination
            resource_requirements: job.resource_requirements.clone(),
            priority: job.priority as u8,
            constraints: vec![], // Add constraints if needed
        };

        Ok(job_request)
    }

    /// Estimate the number of subtasks needed for a job
    async fn estimate_subtask_count(
        &self,
        _job: &UniversalJob,
        complexity: &JobComplexity,
    ) -> ToadStoolResult<usize> {
        let count = match complexity {
            JobComplexity::Simple => 1,
            JobComplexity::Moderate => 5,
            JobComplexity::Complex => 25,
            JobComplexity::UltraMassive => 1000,
        };
        Ok(count)
    }

    /// Analyze job complexity for distribution strategy
    async fn analyze_job_complexity(&self, job: &UniversalJob) -> ToadStoolResult<JobComplexity> {
        // Use resource requirements and execution time estimates
        let cpu_cores = job.resource_requirements.cpu.min_cores;
        let memory_gb =
            job.resource_requirements.memory.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        // Estimate complexity based on resource requirements
        if cpu_cores >= 16.0 || memory_gb >= 64.0 {
            Ok(JobComplexity::UltraMassive)
        } else if cpu_cores >= 8.0 || memory_gb >= 32.0 {
            Ok(JobComplexity::Complex)
        } else if cpu_cores >= 4.0 || memory_gb >= 16.0 {
            Ok(JobComplexity::Moderate)
        } else {
            Ok(JobComplexity::Simple)
        }
    }
}

#[cfg(test)]
#[path = "messaging_tests.rs"]
mod messaging_tests;
