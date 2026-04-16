// SPDX-License-Identifier: AGPL-3.0-or-later
//! In-memory synthetic Akida backend for integration tests (no PCI or `/dev` access).

use crate::backend::{BackendType, ModelHandle, NpuBackend};
use crate::capabilities::{
    BatchCapabilities, Capabilities, ChipVersion, PcieConfig, WeightMutationSupport,
};
use crate::error::Result;
use std::sync::atomic::{AtomicU32, Ordering};

/// Deterministic NPU backend used by downstream crates' coverage tests.
#[derive(Debug)]
pub struct SyntheticNpuBackend {
    caps: Capabilities,
    model_counter: AtomicU32,
}

impl SyntheticNpuBackend {
    /// Same capability profile as the historical `toadstool-core` mock (AKD1000-like).
    #[must_use]
    pub fn coverage_default() -> Self {
        let caps = Capabilities {
            chip_version: ChipVersion::Akd1000,
            npu_count: 80,
            memory_mb: 10,
            pcie: PcieConfig::new(3, 8),
            power_mw: None,
            temperature_c: None,
            mesh: None,
            clock_mode: None,
            batch: Some(BatchCapabilities {
                max_batch: 8,
                optimal_batch: 8,
                optimal_speedup: 2.35,
            }),
            weight_mutation: WeightMutationSupport::None,
        };
        Self {
            caps,
            model_counter: AtomicU32::new(0),
        }
    }
}

impl NpuBackend for SyntheticNpuBackend {
    fn init(_device_id: &str) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::coverage_default())
    }

    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    fn load_model(&mut self, _model: &[u8]) -> Result<ModelHandle> {
        let id = self.model_counter.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(ModelHandle::new(id))
    }

    fn load_reservoir(&mut self, _w_in: &[f32], _w_res: &[f32]) -> Result<()> {
        Ok(())
    }

    fn infer(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        Ok(input.to_vec())
    }

    fn measure_power(&self) -> Result<f32> {
        Ok(1500.0)
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Userspace
    }

    fn is_ready(&self) -> bool {
        true
    }
}
