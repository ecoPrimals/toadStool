// SPDX-License-Identifier: AGPL-3.0-or-later
//! Enum dispatch wrappers for [`UniversalComputeResource`] and [`ComputeContext`].
//!
//! Separated from `cpu_resource.rs` to keep per-file complexity under 750 lines.
//! When new compute backends are added (GPU, TPU), their variants go here
//! alongside `Cpu`.

use std::time::Duration;
use toadstool::error::ToadStoolResult;
use uuid::Uuid;

use crate::cpu_resource::{CpuComputeContext, CpuComputeResource};
use crate::universal::{
    ComputeCapabilities, ComputeContext, ComputeRequirements, UniversalComputeResource,
    UniversalWorkload, WorkloadResult,
};

/// Enum dispatch for [`UniversalComputeResource`](crate::universal::execution::UniversalComputeResource).
pub enum UniversalComputeResourceDispatch {
    /// CPU (Rayon-backed) resource.
    Cpu(CpuComputeResource),
}

/// Enum dispatch for [`ComputeContext`](crate::universal::execution::ComputeContext).
pub enum ComputeContextDispatch {
    /// CPU execution context.
    Cpu(CpuComputeContext),
}

impl ComputeContextDispatch {
    /// Close this context (see [`ComputeContext::close`]).
    pub async fn close(self) -> ToadStoolResult<()> {
        <Self as ComputeContext>::close(Box::new(self)).await
    }
}

impl UniversalComputeResource for UniversalComputeResourceDispatch {
    fn capabilities(&self) -> &ComputeCapabilities {
        match self {
            Self::Cpu(r) => r.capabilities(),
        }
    }

    fn resource_id(&self) -> &str {
        match self {
            Self::Cpu(r) => r.resource_id(),
        }
    }

    async fn create_context(&self) -> ToadStoolResult<ComputeContextDispatch> {
        match self {
            Self::Cpu(r) => r.create_context().await,
        }
    }

    async fn utilization(&self) -> f32 {
        match self {
            Self::Cpu(r) => r.utilization().await,
        }
    }

    fn estimate_execution_time(&self, requirements: &ComputeRequirements) -> Duration {
        match self {
            Self::Cpu(r) => r.estimate_execution_time(requirements),
        }
    }
}

impl ComputeContext for ComputeContextDispatch {
    fn context_id(&self) -> Uuid {
        match self {
            Self::Cpu(c) => c.context_id(),
        }
    }

    fn resource_id(&self) -> &str {
        match self {
            Self::Cpu(c) => c.resource_id(),
        }
    }

    async fn execute(&mut self, workload: &UniversalWorkload) -> ToadStoolResult<WorkloadResult> {
        match self {
            Self::Cpu(c) => c.execute(workload).await,
        }
    }

    async fn close(self: Box<Self>) -> ToadStoolResult<()> {
        match *self {
            Self::Cpu(c) => <CpuComputeContext as ComputeContext>::close(Box::new(c)).await,
        }
    }
}
