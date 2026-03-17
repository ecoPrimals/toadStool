// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-only

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
    pub const fn requires_gpu() -> Self {
        Self::RequiresGPU
    }

    pub const fn prefers_gpu() -> Self {
        Self::PrefersGPU
    }

    pub const fn max_latency_ms(ms: u64) -> Self {
        Self::MaxLatencyMs(ms)
    }

    pub const fn preferred_latency_ms(ms: u64) -> Self {
        Self::PreferredLatencyMs(ms)
    }

    pub const fn min_bandwidth_gbps(gbps: f64) -> Self {
        Self::MinBandwidthGbps(gbps)
    }

    pub const fn min_memory_gb(gb: f64) -> Self {
        Self::MinMemoryGB(gb)
    }

    pub const fn min_cpu_cores(cores: usize) -> Self {
        Self::MinCPUCores(cores)
    }

    pub const fn must_be_local() -> Self {
        Self::MustBeLocal
    }

    pub const fn prefer_local() -> Self {
        Self::PreferLocal
    }

    pub fn requires_capability(cap: impl Into<String>) -> Self {
        Self::RequiresCapability(cap.into())
    }

    pub fn prefers_capability(cap: impl Into<String>) -> Self {
        Self::PrefersCapability(cap.into())
    }

    pub const fn is_hard(&self) -> bool {
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

    pub const fn is_soft(&self) -> bool {
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
            Self::MinMemoryGB(gb) => write!(f, "MinMemory: {gb}GB [HARD]"),
            Self::MinCPUCores(cores) => write!(f, "MinCPU: {cores} cores [HARD]"),
            Self::MaxLatencyMs(ms) => write!(f, "MaxLatency: {ms}ms [HARD]"),
            Self::PreferredLatencyMs(ms) => write!(f, "PreferredLatency: {ms}ms [SOFT]"),
            Self::MinBandwidthGbps(gbps) => write!(f, "MinBandwidth: {gbps}Gbps [HARD]"),
            Self::PreferredBandwidthGbps(gbps) => {
                write!(f, "PreferredBandwidth: {gbps}Gbps [SOFT]")
            }
            Self::RequiresCapability(cap) => write!(f, "RequiresCap: {cap} [HARD]"),
            Self::PrefersCapability(cap) => write!(f, "PrefersCap: {cap} [SOFT]"),
            Self::MustBeLocal => write!(f, "MustBeLocal [HARD]"),
            Self::PreferLocal => write!(f, "PreferLocal [SOFT]"),
            Self::RequiresLayer(layer) => write!(f, "RequiresLayer: {layer} [HARD]"),
            Self::PrefersLayer(layer) => write!(f, "PrefersLayer: {layer} [SOFT]"),
            Self::RequiresPersistentStorage => write!(f, "RequiresPersistentStorage [HARD]"),
            Self::MaxCostPerHour(cost) => write!(f, "MaxCost: ${cost}/hr [HARD]"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hard_constraints() {
        assert!(Constraint::RequiresGPU.is_hard());
        assert!(Constraint::MinMemoryGB(8.0).is_hard());
        assert!(Constraint::MinCPUCores(4).is_hard());
        assert!(Constraint::MaxLatencyMs(100).is_hard());
        assert!(Constraint::MinBandwidthGbps(1.0).is_hard());
        assert!(Constraint::RequiresCapability("cuda".to_string()).is_hard());
        assert!(Constraint::MustBeLocal.is_hard());
        assert!(Constraint::RequiresLayer("gpu".to_string()).is_hard());
        assert!(Constraint::RequiresPersistentStorage.is_hard());
        assert!(Constraint::MaxCostPerHour(10.0).is_hard());
        assert!(
            Constraint::Custom {
                name: "test".to_string(),
                hard: true,
                value: "v".to_string(),
            }
            .is_hard()
        );
    }

    #[test]
    fn test_soft_constraints() {
        assert!(Constraint::PrefersGPU.is_soft());
        assert!(Constraint::PreferredLatencyMs(50).is_soft());
        assert!(Constraint::PreferredBandwidthGbps(10.0).is_soft());
        assert!(Constraint::PrefersCapability("avx".to_string()).is_soft());
        assert!(Constraint::PreferLocal.is_soft());
        assert!(Constraint::PrefersLayer("cache".to_string()).is_soft());
        assert!(Constraint::MinimizeCost.is_soft());
        assert!(
            Constraint::Custom {
                name: "test".to_string(),
                hard: false,
                value: "v".to_string(),
            }
            .is_soft()
        );
    }

    #[test]
    fn test_constraint_names() {
        assert_eq!(Constraint::RequiresGPU.name(), "requires_gpu");
        assert_eq!(Constraint::PrefersGPU.name(), "prefers_gpu");
        assert_eq!(Constraint::MinMemoryGB(8.0).name(), "min_memory_gb");
        assert_eq!(Constraint::MinCPUCores(4).name(), "min_cpu_cores");
        assert_eq!(Constraint::MaxLatencyMs(100).name(), "max_latency_ms");
        assert_eq!(
            Constraint::PreferredLatencyMs(50).name(),
            "preferred_latency_ms"
        );
        assert_eq!(
            Constraint::MinBandwidthGbps(1.0).name(),
            "min_bandwidth_gbps"
        );
        assert_eq!(
            Constraint::PreferredBandwidthGbps(10.0).name(),
            "preferred_bandwidth_gbps"
        );
        assert_eq!(
            Constraint::RequiresCapability("cuda".to_string()).name(),
            "requires_capability"
        );
        assert_eq!(
            Constraint::PrefersCapability("avx".to_string()).name(),
            "prefers_capability"
        );
        assert_eq!(Constraint::MustBeLocal.name(), "must_be_local");
        assert_eq!(Constraint::PreferLocal.name(), "prefer_local");
        assert_eq!(
            Constraint::RequiresLayer("gpu".to_string()).name(),
            "requires_layer"
        );
        assert_eq!(
            Constraint::PrefersLayer("cache".to_string()).name(),
            "prefers_layer"
        );
        assert_eq!(
            Constraint::RequiresPersistentStorage.name(),
            "requires_persistent_storage"
        );
        assert_eq!(Constraint::MaxCostPerHour(10.0).name(), "max_cost_per_hour");
        assert_eq!(Constraint::MinimizeCost.name(), "minimize_cost");
        assert_eq!(
            Constraint::Custom {
                name: "custom_constraint".to_string(),
                hard: true,
                value: "v".to_string(),
            }
            .name(),
            "custom_constraint"
        );
    }

    #[test]
    fn test_convenience_constructors() {
        assert_eq!(Constraint::requires_gpu(), Constraint::RequiresGPU);
        assert_eq!(Constraint::prefers_gpu(), Constraint::PrefersGPU);
        assert_eq!(
            Constraint::max_latency_ms(100),
            Constraint::MaxLatencyMs(100)
        );
        assert_eq!(
            Constraint::preferred_latency_ms(50),
            Constraint::PreferredLatencyMs(50)
        );
        assert_eq!(
            Constraint::min_bandwidth_gbps(1.0),
            Constraint::MinBandwidthGbps(1.0)
        );
        assert_eq!(Constraint::min_memory_gb(8.0), Constraint::MinMemoryGB(8.0));
        assert_eq!(Constraint::min_cpu_cores(4), Constraint::MinCPUCores(4));
        assert_eq!(Constraint::must_be_local(), Constraint::MustBeLocal);
        assert_eq!(Constraint::prefer_local(), Constraint::PreferLocal);
        assert_eq!(
            Constraint::requires_capability("cuda"),
            Constraint::RequiresCapability("cuda".to_string())
        );
        assert_eq!(
            Constraint::prefers_capability("avx"),
            Constraint::PrefersCapability("avx".to_string())
        );
    }

    #[test]
    fn test_display_hard_constraints() {
        assert_eq!(format!("{}", Constraint::RequiresGPU), "RequiresGPU [HARD]");
        assert_eq!(
            format!("{}", Constraint::MinMemoryGB(8.0)),
            "MinMemory: 8GB [HARD]"
        );
        assert_eq!(
            format!("{}", Constraint::MaxLatencyMs(100)),
            "MaxLatency: 100ms [HARD]"
        );
    }

    #[test]
    fn test_display_soft_constraints() {
        assert_eq!(format!("{}", Constraint::PrefersGPU), "PrefersGPU [SOFT]");
        assert_eq!(
            format!("{}", Constraint::PreferredLatencyMs(50)),
            "PreferredLatency: 50ms [SOFT]"
        );
        assert_eq!(
            format!("{}", Constraint::MinimizeCost),
            "MinimizeCost [SOFT]"
        );
    }

    #[test]
    fn test_display_custom() {
        let hard_custom = Constraint::Custom {
            name: "test".to_string(),
            hard: true,
            value: "value".to_string(),
        };
        assert_eq!(format!("{hard_custom}"), "Custom(test=value)[HARD]");

        let soft_custom = Constraint::Custom {
            name: "test".to_string(),
            hard: false,
            value: "value".to_string(),
        };
        assert_eq!(format!("{soft_custom}"), "Custom(test=value)[SOFT]");
    }

    #[test]
    fn test_constraint_serde_roundtrip() {
        let constraints = vec![
            Constraint::RequiresGPU,
            Constraint::MinMemoryGB(16.0),
            Constraint::Custom {
                name: "special".to_string(),
                hard: true,
                value: "test_value".to_string(),
            },
        ];

        for constraint in constraints {
            let json = serde_json::to_string(&constraint).unwrap();
            let deserialized: Constraint = serde_json::from_str(&json).unwrap();
            assert_eq!(constraint, deserialized);
        }
    }
}
