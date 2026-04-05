// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// Substrate types — aligned with metalForge's hardware characterization model.
///
/// The 4 original types (CPU, GPU, NPU, TPU) are expanded with finer-grained
/// variants for mixed-silicon environments where different substrates within
/// the same category have qualitatively different capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubstrateType {
    /// General-purpose CPU.
    Cpu,
    /// Discrete GPU (PCIe, dedicated VRAM).
    Gpu,
    /// Integrated GPU (shared memory with CPU).
    IntegratedGpu,
    /// Neural Processing Unit (e.g. AKD1000, int8/int4 inference).
    Npu,
    /// Tensor Processing Unit (e.g. Google TPU, systolic array).
    Tpu,
    /// FPGA (reconfigurable logic, e.g. Xilinx, Intel).
    Fpga,
    /// Digital Signal Processor.
    Dsp,
    /// Quantum compute substrate (simulators or real QPUs).
    Quantum,
}

impl SubstrateType {
    /// Return lowercase string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
            Self::IntegratedGpu => "igpu",
            Self::Npu => "npu",
            Self::Tpu => "tpu",
            Self::Fpga => "fpga",
            Self::Dsp => "dsp",
            Self::Quantum => "quantum",
        }
    }

    /// Whether this substrate type is suitable for batch compute workloads.
    #[must_use]
    pub const fn is_batch_oriented(&self) -> bool {
        matches!(self, Self::Gpu | Self::Tpu | Self::Fpga)
    }

    /// Whether this substrate type is suitable for low-latency inference.
    #[must_use]
    pub const fn is_latency_oriented(&self) -> bool {
        matches!(
            self,
            Self::Cpu | Self::Npu | Self::Dsp | Self::IntegratedGpu
        )
    }
}
