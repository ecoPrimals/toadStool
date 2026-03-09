// SPDX-License-Identifier: AGPL-3.0-only
//! Substrate selection configuration builder.

use serde::{Deserialize, Serialize};

use super::{Result, ToadStoolConfigTrait};

/// Substrate selection configuration
///
/// **Deep Debt**: Runtime substrate discovery and selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubstrateConfig {
    /// Preferred substrate type
    pub preferred: SubstratePreference,

    /// Fallback order if preferred unavailable
    pub fallback_order: Vec<SubstrateType>,

    /// Power budget in watts (None = unlimited)
    pub power_budget_watts: Option<f64>,

    /// Performance target
    pub performance_target: PerformanceTarget,

    /// Enable auto-discovery
    pub auto_discover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubstratePreference {
    Auto,
    Specific(SubstrateType),
    ByCapability(Vec<String>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubstrateType {
    Cpu,
    Gpu,
    Npu,
    Tpu,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PerformanceTarget {
    Latency,
    Throughput,
    Energy,
    Balanced,
}

impl Default for SubstrateConfig {
    fn default() -> Self {
        Self {
            preferred: SubstratePreference::Auto,
            fallback_order: vec![SubstrateType::Gpu, SubstrateType::Cpu],
            power_budget_watts: None,
            performance_target: PerformanceTarget::Balanced,
            auto_discover: true,
        }
    }
}

impl ToadStoolConfigTrait for SubstrateConfig {
    fn from_env() -> Result<Self> {
        use std::env;

        let preferred =
            env::var("TOADSTOOL_SUBSTRATE_PREFERRED")
                .ok()
                .map_or(SubstratePreference::Auto, |s| {
                    match s.to_lowercase().as_str() {
                        "cpu" => SubstratePreference::Specific(SubstrateType::Cpu),
                        "gpu" => SubstratePreference::Specific(SubstrateType::Gpu),
                        "npu" => SubstratePreference::Specific(SubstrateType::Npu),
                        "tpu" => SubstratePreference::Specific(SubstrateType::Tpu),
                        _ => SubstratePreference::Auto,
                    }
                });

        Ok(Self {
            preferred,
            fallback_order: vec![SubstrateType::Gpu, SubstrateType::Cpu],
            power_budget_watts: env::var("TOADSTOOL_POWER_BUDGET")
                .ok()
                .and_then(|s| s.parse().ok()),
            performance_target: env::var("TOADSTOOL_PERFORMANCE_TARGET").ok().map_or(
                PerformanceTarget::Balanced,
                |s| match s.to_lowercase().as_str() {
                    "latency" => PerformanceTarget::Latency,
                    "throughput" => PerformanceTarget::Throughput,
                    "energy" => PerformanceTarget::Energy,
                    _ => PerformanceTarget::Balanced,
                },
            ),
            auto_discover: env::var("TOADSTOOL_AUTO_DISCOVER")
                .map(|s| s != "false" && s != "0")
                .unwrap_or(true),
        })
    }
}

impl SubstrateConfig {
    /// Edge deployment preset (power-constrained)
    #[must_use]
    pub fn edge() -> Self {
        Self {
            preferred: SubstratePreference::Auto,
            fallback_order: vec![SubstrateType::Npu, SubstrateType::Cpu],
            power_budget_watts: Some(5.0),
            performance_target: PerformanceTarget::Energy,
            auto_discover: true,
        }
    }

    /// Server deployment preset (performance-focused)
    #[must_use]
    pub fn server() -> Self {
        Self {
            preferred: SubstratePreference::Auto,
            fallback_order: vec![SubstrateType::Gpu, SubstrateType::Cpu],
            power_budget_watts: None,
            performance_target: PerformanceTarget::Throughput,
            auto_discover: true,
        }
    }
}

/// Substrate configuration builder
pub struct SubstrateConfigBuilder {
    config: SubstrateConfig,
}

impl SubstrateConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: SubstrateConfig::default(),
        }
    }

    #[must_use]
    pub fn prefer_auto(mut self) -> Self {
        self.config.preferred = SubstratePreference::Auto;
        self
    }

    #[must_use]
    pub fn prefer_cpu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Cpu);
        self
    }

    #[must_use]
    pub fn prefer_gpu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Gpu);
        self
    }

    #[must_use]
    pub fn prefer_npu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Npu);
        self
    }

    #[must_use]
    pub const fn power_budget_watts(mut self, watts: f64) -> Self {
        self.config.power_budget_watts = Some(watts);
        self
    }

    #[must_use]
    pub const fn target_latency(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Latency;
        self
    }

    #[must_use]
    pub const fn target_throughput(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Throughput;
        self
    }

    #[must_use]
    pub const fn target_energy(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Energy;
        self
    }

    #[must_use]
    pub fn build(self) -> SubstrateConfig {
        self.config
    }
}

impl Default for SubstrateConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substrate_config_default() {
        let config = SubstrateConfig::default();
        assert!(matches!(config.preferred, SubstratePreference::Auto));
        assert_eq!(config.fallback_order.len(), 2);
        assert_eq!(config.power_budget_watts, None);
        assert_eq!(config.performance_target, PerformanceTarget::Balanced);
        assert!(config.auto_discover);
    }

    #[test]
    fn test_substrate_config_builder() {
        let config = SubstrateConfigBuilder::new()
            .prefer_npu()
            .power_budget_watts(5.0)
            .target_energy()
            .build();

        assert_eq!(
            config.preferred,
            SubstratePreference::Specific(SubstrateType::Npu)
        );
        assert_eq!(config.power_budget_watts, Some(5.0));
        assert_eq!(config.performance_target, PerformanceTarget::Energy);
    }

    #[test]
    fn test_substrate_config_builder_prefer_auto() {
        let config = SubstrateConfigBuilder::new()
            .prefer_gpu()
            .prefer_auto()
            .build();
        assert!(matches!(config.preferred, SubstratePreference::Auto));
    }

    #[test]
    fn test_substrate_config_builder_prefer_cpu() {
        let config = SubstrateConfigBuilder::new().prefer_cpu().build();
        assert_eq!(
            config.preferred,
            SubstratePreference::Specific(SubstrateType::Cpu)
        );
    }

    #[test]
    fn test_substrate_config_builder_prefer_gpu() {
        let config = SubstrateConfigBuilder::new().prefer_gpu().build();
        assert_eq!(
            config.preferred,
            SubstratePreference::Specific(SubstrateType::Gpu)
        );
    }

    #[test]
    fn test_substrate_config_builder_target_latency() {
        let config = SubstrateConfigBuilder::new().target_latency().build();
        assert_eq!(config.performance_target, PerformanceTarget::Latency);
    }

    #[test]
    fn test_substrate_config_builder_target_throughput() {
        let config = SubstrateConfigBuilder::new().target_throughput().build();
        assert_eq!(config.performance_target, PerformanceTarget::Throughput);
    }

    #[test]
    fn test_substrate_config_builder_default() {
        let config = SubstrateConfigBuilder::default().build();
        assert!(matches!(config.preferred, SubstratePreference::Auto));
        assert_eq!(config.performance_target, PerformanceTarget::Balanced);
    }

    #[test]
    fn test_substrate_config_builder_full_chain() {
        let config = SubstrateConfigBuilder::new()
            .prefer_npu()
            .power_budget_watts(10.0)
            .target_throughput()
            .build();
        assert_eq!(
            config.preferred,
            SubstratePreference::Specific(SubstrateType::Npu)
        );
        assert_eq!(config.power_budget_watts, Some(10.0));
        assert_eq!(config.performance_target, PerformanceTarget::Throughput);
    }

    #[test]
    fn test_substrate_config_presets() {
        let edge = SubstrateConfig::edge();
        assert_eq!(edge.power_budget_watts, Some(5.0));
        assert_eq!(edge.performance_target, PerformanceTarget::Energy);

        let server = SubstrateConfig::server();
        assert_eq!(server.power_budget_watts, None);
        assert_eq!(server.performance_target, PerformanceTarget::Throughput);
    }

    #[test]
    fn test_substrate_config_with_defaults() {
        let config = SubstrateConfig::default();
        let merged = config.with_defaults();
        assert!(matches!(merged.preferred, SubstratePreference::Auto));
    }

    #[test]
    fn test_substrate_config_from_env() {
        let config = SubstrateConfig::from_env().expect("from_env returns Ok");
        assert!(!config.fallback_order.is_empty());
    }

    #[test]
    fn test_substrate_builder_override_chain() {
        let config = SubstrateConfigBuilder::new()
            .prefer_cpu()
            .prefer_gpu()
            .prefer_npu()
            .power_budget_watts(5.0)
            .power_budget_watts(10.0)
            .target_latency()
            .target_throughput()
            .build();
        assert_eq!(
            config.preferred,
            SubstratePreference::Specific(SubstrateType::Npu)
        );
        assert_eq!(config.power_budget_watts, Some(10.0));
        assert_eq!(config.performance_target, PerformanceTarget::Throughput);
    }
}
