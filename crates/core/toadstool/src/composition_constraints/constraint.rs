// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Constraint types for workload composition

use serde::{Deserialize, Serialize};
use std::fmt;

/// A composition constraint
///
/// Constraints are declarative requirements. They describe WHAT is needed,
/// not HOW to achieve it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Constraint {
    RequiresGPU,
    PrefersGPU,
    MinMemoryGB(f64),
    MinCPUCores(usize),
    MaxLatencyMs(u64),
    PreferredLatencyMs(u64),
    MinBandwidthGbps(f64),
    PreferredBandwidthGbps(f64),
    RequiresCapability(String),
    PrefersCapability(String),
    MustBeLocal,
    PreferLocal,
    RequiresLayer(String),
    PrefersLayer(String),
    RequiresPersistentStorage,
    MaxCostPerHour(f64),
    MinimizeCost,
    Custom {
        name: String,
        hard: bool,
        value: String,
    },
}

impl Constraint {
    pub fn requires_gpu() -> Self {
        Self::RequiresGPU
    }

    pub fn prefers_gpu() -> Self {
        Self::PrefersGPU
    }

    pub fn max_latency_ms(ms: u64) -> Self {
        Self::MaxLatencyMs(ms)
    }

    pub fn preferred_latency_ms(ms: u64) -> Self {
        Self::PreferredLatencyMs(ms)
    }

    pub fn min_bandwidth_gbps(gbps: f64) -> Self {
        Self::MinBandwidthGbps(gbps)
    }

    pub fn min_memory_gb(gb: f64) -> Self {
        Self::MinMemoryGB(gb)
    }

    pub fn min_cpu_cores(cores: usize) -> Self {
        Self::MinCPUCores(cores)
    }

    pub fn must_be_local() -> Self {
        Self::MustBeLocal
    }

    pub fn prefer_local() -> Self {
        Self::PreferLocal
    }

    pub fn requires_capability(cap: impl Into<String>) -> Self {
        Self::RequiresCapability(cap.into())
    }

    pub fn prefers_capability(cap: impl Into<String>) -> Self {
        Self::PrefersCapability(cap.into())
    }

    pub fn is_hard(&self) -> bool {
        matches!(
            self,
            Self::RequiresGPU
                | Self::MinMemoryGB(_)
                | Self::MinCPUCores(_)
                | Self::MaxLatencyMs(_)
                | Self::MinBandwidthGbps(_)
                | Self::RequiresCapability(_)
                | Self::MustBeLocal
                | Self::RequiresLayer(_)
                | Self::RequiresPersistentStorage
                | Self::MaxCostPerHour(_)
                | Self::Custom { hard: true, .. }
        )
    }

    pub fn is_soft(&self) -> bool {
        !self.is_hard()
    }

    pub fn name(&self) -> &str {
        match self {
            Self::RequiresGPU => "requires_gpu",
            Self::PrefersGPU => "prefers_gpu",
            Self::MinMemoryGB(_) => "min_memory_gb",
            Self::MinCPUCores(_) => "min_cpu_cores",
            Self::MaxLatencyMs(_) => "max_latency_ms",
            Self::PreferredLatencyMs(_) => "preferred_latency_ms",
            Self::MinBandwidthGbps(_) => "min_bandwidth_gbps",
            Self::PreferredBandwidthGbps(_) => "preferred_bandwidth_gbps",
            Self::RequiresCapability(_) => "requires_capability",
            Self::PrefersCapability(_) => "prefers_capability",
            Self::MustBeLocal => "must_be_local",
            Self::PreferLocal => "prefer_local",
            Self::RequiresLayer(_) => "requires_layer",
            Self::PrefersLayer(_) => "prefers_layer",
            Self::RequiresPersistentStorage => "requires_persistent_storage",
            Self::MaxCostPerHour(_) => "max_cost_per_hour",
            Self::MinimizeCost => "minimize_cost",
            Self::Custom { name, .. } => name,
        }
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiresGPU => write!(f, "RequiresGPU [HARD]"),
            Self::PrefersGPU => write!(f, "PrefersGPU [SOFT]"),
            Self::MinMemoryGB(gb) => write!(f, "MinMemory: {}GB [HARD]", gb),
            Self::MinCPUCores(cores) => write!(f, "MinCPU: {} cores [HARD]", cores),
            Self::MaxLatencyMs(ms) => write!(f, "MaxLatency: {}ms [HARD]", ms),
            Self::PreferredLatencyMs(ms) => write!(f, "PreferredLatency: {}ms [SOFT]", ms),
            Self::MinBandwidthGbps(gbps) => write!(f, "MinBandwidth: {}Gbps [HARD]", gbps),
            Self::PreferredBandwidthGbps(gbps) => {
                write!(f, "PreferredBandwidth: {}Gbps [SOFT]", gbps)
            }
            Self::RequiresCapability(cap) => write!(f, "RequiresCap: {} [HARD]", cap),
            Self::PrefersCapability(cap) => write!(f, "PrefersCap: {} [SOFT]", cap),
            Self::MustBeLocal => write!(f, "MustBeLocal [HARD]"),
            Self::PreferLocal => write!(f, "PreferLocal [SOFT]"),
            Self::RequiresLayer(layer) => write!(f, "RequiresLayer: {} [HARD]", layer),
            Self::PrefersLayer(layer) => write!(f, "PrefersLayer: {} [SOFT]", layer),
            Self::RequiresPersistentStorage => write!(f, "RequiresPersistentStorage [HARD]"),
            Self::MaxCostPerHour(cost) => write!(f, "MaxCost: ${}/hr [HARD]", cost),
            Self::MinimizeCost => write!(f, "MinimizeCost [SOFT]"),
            Self::Custom { name, hard, value } => {
                write!(
                    f,
                    "Custom({}={})[{}]",
                    name,
                    value,
                    if *hard { "HARD" } else { "SOFT" }
                )
            }
        }
    }
}
