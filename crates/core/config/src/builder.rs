//! Universal Configuration Builder Pattern
//!
//! **Deep Debt Principle**: No hardcoded values, all runtime configurable
//!
//! This module provides a unified builder pattern for all ToadStool configurations,
//! enabling runtime flexibility, TOML file support, and environment variable integration.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool_config::builder::*;
//!
//! // Method 1: Builder pattern
//! let config = ProfilerConfigBuilder::new()
//!     .warmup_iterations(20)
//!     .benchmark_iterations(500)
//!     .timeout_ms(30000)
//!     .parallel()
//!     .build();
//!
//! // Method 2: From TOML file
//! let config = ProfilerConfig::from_file("profiler.toml")?;
//!
//! // Method 3: From environment variables
//! let config = ProfilerConfig::from_env()?;
//!
//! // Method 4: Quick presets
//! let config = ProfilerConfig::quick();  // Fast benchmarks
//! let config = ProfilerConfig::thorough();  // Comprehensive benchmarks
//! ```

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Environment variable error: {0}")]
    EnvVar(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Base trait for all ToadStool configurations
///
/// **Deep Debt**: All configs support multiple sources (file, env, builder)
pub trait ToadStoolConfigTrait: Serialize + for<'de> Deserialize<'de> + Default + Sized {
    /// Load from TOML file
    fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    /// Load from environment variables (with TOADSTOOL_ prefix)
    fn from_env() -> Result<Self>;

    /// Save to TOML file
    fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let contents =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Validation(e.to_string()))?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Merge with defaults (self takes precedence)
    fn with_defaults(self) -> Self {
        self // Default implementation: no merge needed
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        Ok(()) // Default: always valid
    }
}

// ═══════════════════════════════════════════════════════════
// PROFILER CONFIGURATION
// ═══════════════════════════════════════════════════════════

/// Profiler configuration
///
/// **Deep Debt**: No hardcoded values, all runtime configurable
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfilerConfig {
    /// Number of warmup iterations
    pub warmup_iterations: usize,

    /// Number of benchmark iterations
    pub benchmark_iterations: usize,

    /// Timeout in milliseconds (None = unlimited)
    pub timeout_ms: Option<u64>,

    /// Run benchmarks in parallel
    pub parallel: bool,

    /// Collect detailed metrics
    pub detailed_metrics: bool,

    /// Output format (json, csv, markdown)
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Csv,
    Markdown,
    Pretty,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 10,
            benchmark_iterations: 100,
            timeout_ms: None,
            parallel: false,
            detailed_metrics: false,
            output_format: OutputFormat::Pretty,
        }
    }
}

impl ToadStoolConfigTrait for ProfilerConfig {
    fn from_env() -> Result<Self> {
        use std::env;

        Ok(Self {
            warmup_iterations: env::var("TOADSTOOL_PROFILER_WARMUP")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            benchmark_iterations: env::var("TOADSTOOL_PROFILER_BENCH_ITERS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            timeout_ms: env::var("TOADSTOOL_PROFILER_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok()),
            parallel: env::var("TOADSTOOL_PROFILER_PARALLEL")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(false),
            detailed_metrics: env::var("TOADSTOOL_PROFILER_DETAILED")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(false),
            output_format: env::var("TOADSTOOL_PROFILER_OUTPUT")
                .ok()
                .and_then(|s| match s.to_lowercase().as_str() {
                    "json" => Some(OutputFormat::Json),
                    "csv" => Some(OutputFormat::Csv),
                    "markdown" => Some(OutputFormat::Markdown),
                    "pretty" => Some(OutputFormat::Pretty),
                    _ => None,
                })
                .unwrap_or(OutputFormat::Pretty),
        })
    }

    fn validate(&self) -> Result<()> {
        if self.warmup_iterations == 0 {
            return Err(ConfigError::Validation(
                "warmup_iterations must be > 0".to_string(),
            ));
        }
        if self.benchmark_iterations == 0 {
            return Err(ConfigError::Validation(
                "benchmark_iterations must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl ProfilerConfig {
    /// Quick configuration for fast benchmarks
    pub fn quick() -> Self {
        Self {
            warmup_iterations: 5,
            benchmark_iterations: 50,
            timeout_ms: Some(5000),
            parallel: false,
            detailed_metrics: false,
            output_format: OutputFormat::Pretty,
        }
    }

    /// Thorough configuration for comprehensive benchmarks
    pub fn thorough() -> Self {
        Self {
            warmup_iterations: 20,
            benchmark_iterations: 500,
            timeout_ms: Some(60000),
            parallel: true,
            detailed_metrics: true,
            output_format: OutputFormat::Json,
        }
    }

    /// Production configuration for real-world benchmarks
    pub fn production() -> Self {
        Self {
            warmup_iterations: 10,
            benchmark_iterations: 1000,
            timeout_ms: None,
            parallel: true,
            detailed_metrics: true,
            output_format: OutputFormat::Json,
        }
    }
}

/// Profiler configuration builder
///
/// **Deep Debt**: Fluent API for runtime configuration
pub struct ProfilerConfigBuilder {
    config: ProfilerConfig,
}

impl ProfilerConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ProfilerConfig::default(),
        }
    }

    pub fn warmup_iterations(mut self, n: usize) -> Self {
        self.config.warmup_iterations = n;
        self
    }

    pub fn benchmark_iterations(mut self, n: usize) -> Self {
        self.config.benchmark_iterations = n;
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.config.timeout_ms = Some(ms);
        self
    }

    pub fn no_timeout(mut self) -> Self {
        self.config.timeout_ms = None;
        self
    }

    pub fn parallel(mut self) -> Self {
        self.config.parallel = true;
        self
    }

    pub fn sequential(mut self) -> Self {
        self.config.parallel = false;
        self
    }

    pub fn detailed_metrics(mut self) -> Self {
        self.config.detailed_metrics = true;
        self
    }

    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.config.output_format = format;
        self
    }

    pub fn build(self) -> Result<ProfilerConfig> {
        self.config.validate()?;
        Ok(self.config)
    }

    pub fn build_unchecked(self) -> ProfilerConfig {
        self.config
    }
}

impl Default for ProfilerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════
// SUBSTRATE CONFIGURATION
// ═══════════════════════════════════════════════════════════

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SubstratePreference {
    Auto,
    Specific(SubstrateType),
    ByCapability(Vec<String>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SubstrateType {
    Cpu,
    Gpu,
    Npu,
    Tpu,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PerformanceTarget {
    Latency,    // Minimize latency
    Throughput, // Maximize throughput
    Energy,     // Minimize energy
    Balanced,   // Balance all factors
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

        let preferred = env::var("TOADSTOOL_SUBSTRATE_PREFERRED")
            .ok()
            .map(|s| match s.to_lowercase().as_str() {
                "auto" => SubstratePreference::Auto,
                "cpu" => SubstratePreference::Specific(SubstrateType::Cpu),
                "gpu" => SubstratePreference::Specific(SubstrateType::Gpu),
                "npu" => SubstratePreference::Specific(SubstrateType::Npu),
                "tpu" => SubstratePreference::Specific(SubstrateType::Tpu),
                _ => SubstratePreference::Auto,
            })
            .unwrap_or(SubstratePreference::Auto);

        Ok(Self {
            preferred,
            fallback_order: vec![SubstrateType::Gpu, SubstrateType::Cpu],
            power_budget_watts: env::var("TOADSTOOL_POWER_BUDGET")
                .ok()
                .and_then(|s| s.parse().ok()),
            performance_target: env::var("TOADSTOOL_PERFORMANCE_TARGET")
                .ok()
                .map(|s| match s.to_lowercase().as_str() {
                    "latency" => PerformanceTarget::Latency,
                    "throughput" => PerformanceTarget::Throughput,
                    "energy" => PerformanceTarget::Energy,
                    _ => PerformanceTarget::Balanced,
                })
                .unwrap_or(PerformanceTarget::Balanced),
            auto_discover: env::var("TOADSTOOL_AUTO_DISCOVER")
                .map(|s| s != "false" && s != "0")
                .unwrap_or(true),
        })
    }
}

impl SubstrateConfig {
    /// Edge deployment preset (power-constrained)
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
    pub fn new() -> Self {
        Self {
            config: SubstrateConfig::default(),
        }
    }

    pub fn prefer_auto(mut self) -> Self {
        self.config.preferred = SubstratePreference::Auto;
        self
    }

    pub fn prefer_cpu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Cpu);
        self
    }

    pub fn prefer_gpu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Gpu);
        self
    }

    pub fn prefer_npu(mut self) -> Self {
        self.config.preferred = SubstratePreference::Specific(SubstrateType::Npu);
        self
    }

    pub fn power_budget_watts(mut self, watts: f64) -> Self {
        self.config.power_budget_watts = Some(watts);
        self
    }

    pub fn target_latency(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Latency;
        self
    }

    pub fn target_throughput(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Throughput;
        self
    }

    pub fn target_energy(mut self) -> Self {
        self.config.performance_target = PerformanceTarget::Energy;
        self
    }

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
    fn test_profiler_config_builder() {
        let config = ProfilerConfigBuilder::new()
            .warmup_iterations(20)
            .benchmark_iterations(500)
            .timeout_ms(30000)
            .parallel()
            .detailed_metrics()
            .build()
            .unwrap();

        assert_eq!(config.warmup_iterations, 20);
        assert_eq!(config.benchmark_iterations, 500);
        assert_eq!(config.timeout_ms, Some(30000));
        assert!(config.parallel);
        assert!(config.detailed_metrics);
    }

    #[test]
    fn test_profiler_config_presets() {
        let quick = ProfilerConfig::quick();
        assert_eq!(quick.warmup_iterations, 5);
        assert_eq!(quick.benchmark_iterations, 50);

        let thorough = ProfilerConfig::thorough();
        assert_eq!(thorough.warmup_iterations, 20);
        assert_eq!(thorough.benchmark_iterations, 500);
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
    fn test_substrate_config_presets() {
        let edge = SubstrateConfig::edge();
        assert_eq!(edge.power_budget_watts, Some(5.0));
        assert_eq!(edge.performance_target, PerformanceTarget::Energy);

        let server = SubstrateConfig::server();
        assert_eq!(server.power_budget_watts, None);
        assert_eq!(server.performance_target, PerformanceTarget::Throughput);
    }
}
