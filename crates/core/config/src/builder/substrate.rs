// SPDX-License-Identifier: AGPL-3.0-or-later
//! Substrate selection configuration builder.

use serde::{Deserialize, Serialize};

use toadstool_common::interned_strings::socket_env;

use super::{ConfigError, Result, ToadStoolConfigTrait};

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

/// How to select compute substrate (CPU, GPU, NPU, TPU).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubstratePreference {
    /// Auto-discover best available substrate.
    Auto,
    /// Use a specific substrate type.
    Specific(SubstrateType),
    /// Select by capability names (e.g. `["cuda", "vulkan", "wgpu"]`).
    ByCapability(Vec<String>),
}

/// Compute substrate type. Valid values: cpu, gpu, npu, tpu.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubstrateType {
    /// CPU execution.
    Cpu,
    /// GPU (`CUDA`, Vulkan, wgpu, etc.; OpenCL removed — see S198).
    Gpu,
    /// Neural processing unit.
    Npu,
    /// Tensor processing unit.
    Tpu,
}

/// Performance optimization target.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PerformanceTarget {
    /// Minimize latency.
    Latency,
    /// Maximize throughput.
    Throughput,
    /// Minimize energy consumption.
    Energy,
    /// Balance latency, throughput, and energy.
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

        let preferred = env::var(socket_env::TOADSTOOL_SUBSTRATE_PREFERRED)
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
            power_budget_watts: env::var(socket_env::TOADSTOOL_POWER_BUDGET)
                .ok()
                .and_then(|s| s.parse().ok()),
            performance_target: env::var(socket_env::TOADSTOOL_PERFORMANCE_TARGET)
                .ok()
                .map_or(PerformanceTarget::Balanced, |s| {
                    match s.to_lowercase().as_str() {
                        "latency" => PerformanceTarget::Latency,
                        "throughput" => PerformanceTarget::Throughput,
                        "energy" => PerformanceTarget::Energy,
                        _ => PerformanceTarget::Balanced,
                    }
                }),
            auto_discover: env::var(socket_env::TOADSTOOL_AUTO_DISCOVER)
                .map_or(true, |s| s != "false" && s != "0"),
        })
    }

    fn validate(&self) -> Result<()> {
        if let Some(w) = self.power_budget_watts {
            if !w.is_finite() || w <= 0.0 {
                return Err(ConfigError::Validation(
                    "power_budget_watts must be positive and finite when set".to_string(),
                ));
            }
        }
        if self.fallback_order.is_empty() {
            return Err(ConfigError::Validation(
                "fallback_order must not be empty".to_string(),
            ));
        }
        if let SubstratePreference::ByCapability(caps) = &self.preferred
            && caps.is_empty()
        {
            return Err(ConfigError::Validation(
                "ByCapability preference requires at least one capability name".to_string(),
            ));
        }
        Ok(())
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

/// Substrate configuration builder.
///
/// Fluent API for substrate selection, power budget, and performance target.
pub struct SubstrateConfigBuilder {
    config: SubstrateConfig,
}

impl SubstrateConfigBuilder {
    /// Create a new builder with default substrate config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: SubstrateConfig::default(),
        }
    }

    /// Prefer auto-discovery of best substrate.
    #[must_use]
    pub fn prefer_auto(mut self) -> Self {
        self.config.preferred = SubstratePreference::Auto;
        self
    }

    /// Prefer CPU execution.
    #[must_use]
    pub fn prefer_cpu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Cpu);
        self
    }

    /// Prefer GPU execution.
    #[must_use]
    pub fn prefer_gpu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Gpu);
        self
    }

    /// Prefer NPU execution.
    #[must_use]
    pub fn prefer_npu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Npu);
        self
    }

    /// Set power budget in watts (for power-constrained deployments).
    #[must_use]
    pub const fn power_budget_watts(mut self, watts: f64) -> Self {
        self.config.power_budget_watts = Some(watts);
        self
    }

    /// Target minimum latency.
    #[must_use]
    pub const fn target_latency(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Latency;
        self
    }

    /// Target maximum throughput.
    #[must_use]
    pub const fn target_throughput(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Throughput;
        self
    }

    /// Target minimum energy consumption.
    #[must_use]
    pub const fn target_energy(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Energy;
        self
    }

    /// Build and validate the substrate configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if validation fails.
    pub fn build(self) -> Result<SubstrateConfig> {
        self.config.validate()?;
        Ok(self.config)
    }

    /// Build without validation (may produce invalid config).
    #[must_use]
    pub fn build_unchecked(self) -> SubstrateConfig {
        self.config
    }
}

impl Default for SubstrateConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "substrate_tests.rs"]
mod tests;
