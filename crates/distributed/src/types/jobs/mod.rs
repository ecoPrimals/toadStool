// SPDX-License-Identifier: AGPL-3.0-or-later
//! Job types, queues, and scheduling for distributed workloads.

mod execution_target;
mod hosting;
mod queue;
mod universal_job;

#[cfg(test)]
mod tests;

pub use execution_target::{ExecutionTarget, LoadBalancingStrategy};
pub use hosting::{CompatibilityMode, ToadStoolHostingConfig};
pub use queue::{DependencyGraph, JobMetadata, ResourceRequirementIndex, UniversalJobQueue};
pub use toadstool::JobPriority;
pub use universal_job::{UniversalJob, UniversalJobType};
