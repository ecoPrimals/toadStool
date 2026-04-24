// SPDX-License-Identifier: AGPL-3.0-or-later
//! Enum dispatch for [`ComputeUnit`](super::compute_unit::ComputeUnit).

use super::capabilities::Capabilities;
use super::compute_unit::ComputeUnit;
use super::error::ComputeError;
use super::output::Output;
use super::workload::Workload;

#[cfg(feature = "cpu")]
use crate::backends::CpuComputeUnit;
#[cfg(feature = "wgpu-backend")]
use crate::backends::WgpuComputeUnit;

/// Closed set of in-tree [`ComputeUnit`] implementations.
pub enum ComputeUnitDispatch {
    /// CPU parallel unit.
    #[cfg(feature = "cpu")]
    Cpu(CpuComputeUnit),
    /// `wgpu` GPU discovery unit.
    #[cfg(feature = "wgpu-backend")]
    Wgpu(WgpuComputeUnit),
}

impl ComputeUnit for ComputeUnitDispatch {
    fn capabilities(&self) -> &Capabilities {
        match self {
            #[cfg(feature = "cpu")]
            Self::Cpu(u) => u.capabilities(),
            #[cfg(feature = "wgpu-backend")]
            Self::Wgpu(u) => u.capabilities(),
        }
    }

    fn name(&self) -> &str {
        match self {
            #[cfg(feature = "cpu")]
            Self::Cpu(u) => u.name(),
            #[cfg(feature = "wgpu-backend")]
            Self::Wgpu(u) => u.name(),
        }
    }

    async fn execute(&self, workload: Workload) -> Result<Output, ComputeError> {
        match self {
            #[cfg(feature = "cpu")]
            Self::Cpu(u) => u.execute(workload).await,
            #[cfg(feature = "wgpu-backend")]
            Self::Wgpu(u) => u.execute(workload).await,
        }
    }

    fn optimal_batch_size(&self) -> usize {
        match self {
            #[cfg(feature = "cpu")]
            Self::Cpu(u) => u.optimal_batch_size(),
            #[cfg(feature = "wgpu-backend")]
            Self::Wgpu(u) => u.optimal_batch_size(),
        }
    }

    fn estimate_duration(&self, workload: &Workload) -> std::time::Duration {
        match self {
            #[cfg(feature = "cpu")]
            Self::Cpu(u) => u.estimate_duration(workload),
            #[cfg(feature = "wgpu-backend")]
            Self::Wgpu(u) => u.estimate_duration(workload),
        }
    }
}
