//! Universal Configuration Builder Pattern
//!
//! **Deep Debt Principle**: No hardcoded values, all runtime configurable
//!
//! This module provides a unified builder pattern for all ToadStool configurations,
//! enabling runtime flexibility, TOML file support, and environment variable integration.
//!
//! # Example
//!
//! ```rust,ignore
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
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if the file cannot be read or TOML parsing fails.
    fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    /// Load from environment variables (with TOADSTOOL_ prefix)
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if required environment variables are missing or invalid.
    fn from_env() -> Result<Self>;

    /// Save to TOML file
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if serialization fails or the file cannot be written.
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
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if validation fails (implementation-specific).
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
                .unwrap_or_default(),
            detailed_metrics: env::var("TOADSTOOL_PROFILER_DETAILED")
                .map(|s| s == "true" || s == "1")
                .unwrap_or_default(),
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
    #[must_use]
    pub const fn quick() -> Self {
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
    #[must_use]
    pub const fn thorough() -> Self {
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
    #[must_use]
    pub const fn production() -> Self {
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ProfilerConfig::default(),
        }
    }

    #[must_use]
    pub const fn warmup_iterations(mut self, n: usize) -> Self {
        self.config.warmup_iterations = n;
        self
    }

    #[must_use]
    pub const fn benchmark_iterations(mut self, n: usize) -> Self {
        self.config.benchmark_iterations = n;
        self
    }

    #[must_use]
    pub const fn timeout_ms(mut self, ms: u64) -> Self {
        self.config.timeout_ms = Some(ms);
        self
    }

    #[must_use]
    pub const fn no_timeout(mut self) -> Self {
        self.config.timeout_ms = None;
        self
    }

    #[must_use]
    pub const fn parallel(mut self) -> Self {
        self.config.parallel = true;
        self
    }

    #[must_use]
    pub const fn sequential(mut self) -> Self {
        self.config.parallel = false;
        self
    }

    #[must_use]
    pub const fn detailed_metrics(mut self) -> Self {
        self.config.detailed_metrics = true;
        self
    }

    #[must_use]
    pub const fn output_format(mut self, format: OutputFormat) -> Self {
        self.config.output_format = format;
        self
    }

    /// Build and validate the profiler configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if validation fails.
    pub fn build(self) -> Result<ProfilerConfig> {
        self.config.validate()?;
        Ok(self.config)
    }

    #[must_use]
    pub const fn build_unchecked(self) -> ProfilerConfig {
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
    fn test_profiler_config_builder() {
        let config = ProfilerConfigBuilder::new()
            .warmup_iterations(20)
            .benchmark_iterations(500)
            .timeout_ms(30000)
            .parallel()
            .detailed_metrics()
            .build()
            .expect("valid config");

        assert_eq!(config.warmup_iterations, 20);
        assert_eq!(config.benchmark_iterations, 500);
        assert_eq!(config.timeout_ms, Some(30000));
        assert!(config.parallel);
        assert!(config.detailed_metrics);
    }

    #[test]
    fn test_profiler_config_builder_no_timeout() {
        let config = ProfilerConfigBuilder::new()
            .timeout_ms(5000)
            .no_timeout()
            .build()
            .expect("valid config");
        assert_eq!(config.timeout_ms, None);
    }

    #[test]
    fn test_profiler_config_builder_sequential() {
        let config = ProfilerConfigBuilder::new()
            .parallel()
            .sequential()
            .build()
            .expect("valid config");
        assert!(!config.parallel);
    }

    #[test]
    fn test_profiler_config_builder_output_format() {
        for (format, expected) in [
            (OutputFormat::Json, OutputFormat::Json),
            (OutputFormat::Csv, OutputFormat::Csv),
            (OutputFormat::Markdown, OutputFormat::Markdown),
            (OutputFormat::Pretty, OutputFormat::Pretty),
        ] {
            let config = ProfilerConfigBuilder::new()
                .output_format(format)
                .build()
                .expect("valid config");
            assert_eq!(config.output_format, expected);
        }
    }

    #[test]
    fn test_profiler_config_builder_build_unchecked() {
        let config = ProfilerConfigBuilder::new()
            .warmup_iterations(0)
            .benchmark_iterations(0)
            .build_unchecked();
        assert_eq!(config.warmup_iterations, 0);
        assert_eq!(config.benchmark_iterations, 0);
    }

    #[test]
    fn test_profiler_config_builder_default() {
        let config = ProfilerConfigBuilder::default()
            .build()
            .expect("valid config");
        assert_eq!(config.warmup_iterations, 10);
        assert_eq!(config.benchmark_iterations, 100);
        assert_eq!(config.output_format, OutputFormat::Pretty);
    }

    #[test]
    fn test_profiler_config_builder_full_chain() {
        let config = ProfilerConfigBuilder::new()
            .warmup_iterations(7)
            .benchmark_iterations(200)
            .timeout_ms(15000)
            .no_timeout()
            .sequential()
            .output_format(OutputFormat::Markdown)
            .build()
            .expect("valid config");
        assert_eq!(config.warmup_iterations, 7);
        assert_eq!(config.benchmark_iterations, 200);
        assert_eq!(config.timeout_ms, None);
        assert!(!config.parallel);
        assert_eq!(config.output_format, OutputFormat::Markdown);
    }

    #[test]
    fn test_profiler_build_validation_warmup_zero() {
        let err = ProfilerConfigBuilder::new()
            .warmup_iterations(0)
            .build()
            .expect_err("warmup_iterations 0 should fail");
        assert!(
            matches!(err, ConfigError::Validation(s) if s.contains("warmup_iterations")),
            "expected Validation error for warmup_iterations"
        );
    }

    #[test]
    fn test_profiler_build_validation_benchmark_zero() {
        let err = ProfilerConfigBuilder::new()
            .benchmark_iterations(0)
            .build()
            .expect_err("benchmark_iterations 0 should fail");
        assert!(
            matches!(err, ConfigError::Validation(s) if s.contains("benchmark_iterations")),
            "expected Validation error for benchmark_iterations"
        );
    }

    #[test]
    fn test_profiler_config_default() {
        let config = ProfilerConfig::default();
        assert_eq!(config.warmup_iterations, 10);
        assert_eq!(config.benchmark_iterations, 100);
        assert_eq!(config.timeout_ms, None);
        assert!(!config.parallel);
        assert!(!config.detailed_metrics);
        assert_eq!(config.output_format, OutputFormat::Pretty);
    }

    #[test]
    fn test_profiler_config_presets() {
        let quick = ProfilerConfig::quick();
        assert_eq!(quick.warmup_iterations, 5);
        assert_eq!(quick.benchmark_iterations, 50);
        assert_eq!(quick.timeout_ms, Some(5000));
        assert!(!quick.parallel);

        let thorough = ProfilerConfig::thorough();
        assert_eq!(thorough.warmup_iterations, 20);
        assert_eq!(thorough.benchmark_iterations, 500);
        assert_eq!(thorough.timeout_ms, Some(60000));
        assert!(thorough.parallel);
        assert!(thorough.detailed_metrics);

        let production = ProfilerConfig::production();
        assert_eq!(production.warmup_iterations, 10);
        assert_eq!(production.benchmark_iterations, 1000);
        assert_eq!(production.timeout_ms, None);
        assert!(production.parallel);
        assert!(production.detailed_metrics);
    }

    #[test]
    fn test_profiler_config_validate_success() {
        let config = ProfilerConfig::default();
        config.validate().expect("default config valid");
    }

    #[test]
    fn test_profiler_config_validate_failure() {
        let config = ProfilerConfig {
            warmup_iterations: 0,
            ..ProfilerConfig::default()
        };
        config.validate().expect_err("warmup 0 invalid");

        let config = ProfilerConfig {
            benchmark_iterations: 0,
            ..ProfilerConfig::default()
        };
        config.validate().expect_err("benchmark 0 invalid");
    }

    #[test]
    fn test_profiler_config_from_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("profiler.toml");
        let contents = r#"warmup_iterations = 3
benchmark_iterations = 30
timeout_ms = 5000
parallel = false
detailed_metrics = false
output_format = "json"
"#;
        std::fs::write(&path, contents).expect("write toml");
        let config = ProfilerConfig::from_file(&path).expect("load from file");
        assert_eq!(config.warmup_iterations, 3);
        assert_eq!(config.benchmark_iterations, 30);
        assert_eq!(config.output_format, OutputFormat::Json);
    }

    #[test]
    fn test_profiler_config_to_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("profiler_out.toml");
        let config = ProfilerConfig::quick();
        config.to_file(&path).expect("write toml");
        let contents = std::fs::read_to_string(&path).expect("read file");
        assert!(contents.contains("warmup_iterations"));
        assert!(contents.contains('5'));
    }

    #[test]
    fn test_profiler_config_from_file_not_found() {
        let err =
            ProfilerConfig::from_file("/nonexistent/path/to/profiler.toml").expect_err("missing");
        assert!(matches!(err, ConfigError::Io(_)));
    }

    #[test]
    fn test_profiler_config_with_defaults() {
        let config = ProfilerConfig::quick();
        let warmup = config.warmup_iterations;
        let merged = config.with_defaults();
        assert_eq!(merged.warmup_iterations, warmup);
    }

    #[test]
    fn test_profiler_config_from_env() {
        let config = ProfilerConfig::from_env().expect("from_env returns Ok");
        assert!(config.warmup_iterations > 0);
        assert!(config.benchmark_iterations > 0);
    }

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
    fn test_output_format_variants() {
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_eq!(OutputFormat::Csv, OutputFormat::Csv);
        assert_eq!(OutputFormat::Markdown, OutputFormat::Markdown);
        assert_eq!(OutputFormat::Pretty, OutputFormat::Pretty);
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::Validation("test".to_string());
        assert_eq!(err.to_string(), "Validation error: test");
    }

    #[test]
    fn test_profiler_builder_override_chain_last_wins() {
        let config = ProfilerConfigBuilder::new()
            .warmup_iterations(10)
            .warmup_iterations(20)
            .benchmark_iterations(50)
            .benchmark_iterations(100)
            .build()
            .expect("valid config");
        assert_eq!(config.warmup_iterations, 20);
        assert_eq!(config.benchmark_iterations, 100);
    }

    #[test]
    fn test_profiler_builder_override_parallel_sequential() {
        let config = ProfilerConfigBuilder::new()
            .parallel()
            .sequential()
            .build()
            .expect("valid config");
        assert!(!config.parallel);
    }

    #[test]
    fn test_profiler_from_file_invalid_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("invalid.toml");
        std::fs::write(&path, "warmup_iterations = [invalid").expect("write");
        let err = ProfilerConfig::from_file(&path).expect_err("invalid TOML");
        assert!(matches!(err, ConfigError::Toml(_)));
    }

    #[test]
    fn test_profiler_from_file_wrong_structure_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("wrong.toml");
        std::fs::write(&path, "[section]\nfoo = 1").expect("write");
        let err = ProfilerConfig::from_file(&path);
        assert!(err.is_err(), "wrong TOML structure should fail parse");
        assert!(matches!(err, Err(ConfigError::Toml(_))));
    }

    #[test]
    fn test_profiler_to_file_serialization_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.toml");
        let config = ProfilerConfig::default();
        let result = config.to_file(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_profiler_build_validation_both_zero() {
        let err = ProfilerConfigBuilder::new()
            .warmup_iterations(0)
            .benchmark_iterations(0)
            .build()
            .expect_err("both zero should fail");
        assert!(matches!(err, ConfigError::Validation(s) if s.contains("warmup")));
    }

    #[test]
    fn test_config_error_io_display() {
        let err = ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("file not found"));
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

    #[test]
    fn test_profiler_builder_empty_chain_builds_defaults() {
        let config = ProfilerConfigBuilder::new().build().expect("valid");
        assert_eq!(config.warmup_iterations, 10);
        assert_eq!(config.benchmark_iterations, 100);
    }
}
