// SPDX-License-Identifier: AGPL-3.0-only

use super::substrate_kind::SubstrateType;
use serde::{Deserialize, Serialize};

/// Substrate capabilities
///
/// **Deep Debt**: Discovered at runtime, not hardcoded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateCapabilities {
    /// Substrate type
    pub substrate_type: SubstrateType,

    /// Average power consumption (watts)
    pub power_watts: f64,

    /// Peak throughput (operations/second)
    pub throughput_ops_per_sec: f64,

    /// Typical latency (milliseconds)
    pub latency_ms: f64,

    /// Best suited for batch operations
    pub best_for_batch: bool,

    /// Best suited for low latency
    pub best_for_latency: bool,

    /// Best suited for energy efficiency
    pub best_for_energy: bool,

    /// Best suited for continuous operation
    pub best_for_continuous: bool,

    /// Memory capacity in bytes (runtime-discovered, 0 = unknown).
    #[serde(default)]
    pub memory_capacity_bytes: u64,

    /// Memory bandwidth in bytes/second (runtime-discovered, 0 = unknown).
    #[serde(default)]
    pub memory_bandwidth_bps: u64,
}

impl SubstrateCapabilities {
    /// Create default capabilities for a substrate type
    ///
    /// **Note**: These are conservative estimates. Real implementations
    /// should measure actual hardware capabilities.
    #[allow(clippy::missing_const_for_fn)] // Struct has f64 fields
    pub fn default_for_type(substrate_type: SubstrateType) -> Self {
        match substrate_type {
            SubstrateType::Cpu => Self {
                substrate_type,
                power_watts: 65.0,
                throughput_ops_per_sec: 1e9,
                latency_ms: 0.1,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: false,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Gpu => Self {
                substrate_type,
                power_watts: 250.0,
                throughput_ops_per_sec: 1e12,
                latency_ms: 2.0,
                best_for_batch: true,
                best_for_latency: false,
                best_for_energy: false,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::IntegratedGpu => Self {
                substrate_type,
                power_watts: 15.0,
                throughput_ops_per_sec: 1e11,
                latency_ms: 1.0,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: true,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Npu => Self {
                substrate_type,
                power_watts: 2.0,
                throughput_ops_per_sec: 1e10,
                latency_ms: 1.0,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: true,
                best_for_continuous: false,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Tpu => Self {
                substrate_type,
                power_watts: 200.0,
                throughput_ops_per_sec: 1e13,
                latency_ms: 5.0,
                best_for_batch: true,
                best_for_latency: false,
                best_for_energy: false,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Fpga => Self {
                substrate_type,
                power_watts: 25.0,
                throughput_ops_per_sec: 1e10,
                latency_ms: 0.5,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: true,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Dsp => Self {
                substrate_type,
                power_watts: 5.0,
                throughput_ops_per_sec: 1e9,
                latency_ms: 0.2,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: true,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Quantum => Self {
                substrate_type,
                power_watts: 15_000.0,
                throughput_ops_per_sec: 1e6,
                latency_ms: 100.0,
                best_for_batch: false,
                best_for_latency: false,
                best_for_energy: false,
                best_for_continuous: false,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
        }
    }
}
