// SPDX-License-Identifier: AGPL-3.0-only
//! Profiler configuration builder.

use serde::{Deserialize, Serialize};

use super::{ConfigError, Result, ToadStoolConfigTrait};

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

/// Profiler benchmark output format. Valid values: json, csv, markdown, pretty.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// JSON output for machine parsing.
    Json,
    /// CSV for spreadsheets.
    Csv,
    /// Markdown tables.
    Markdown,
    /// Human-readable formatted output.
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
                .is_ok_and(|s| s == "true" || s == "1"),
            detailed_metrics: env::var("TOADSTOOL_PROFILER_DETAILED")
                .is_ok_and(|s| s == "true" || s == "1"),
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
    /// Create a new profiler config builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ProfilerConfig::default(),
        }
    }

    /// Set number of warmup iterations before benchmarking.
    #[must_use]
    pub const fn warmup_iterations(mut self, n: usize) -> Self {
        self.config.warmup_iterations = n;
        self
    }

    /// Set number of benchmark iterations.
    #[must_use]
    pub const fn benchmark_iterations(mut self, n: usize) -> Self {
        self.config.benchmark_iterations = n;
        self
    }

    /// Set benchmark timeout in milliseconds.
    #[must_use]
    pub const fn timeout_ms(mut self, ms: u64) -> Self {
        self.config.timeout_ms = Some(ms);
        self
    }

    /// Disable benchmark timeout (run until complete).
    #[must_use]
    pub const fn no_timeout(mut self) -> Self {
        self.config.timeout_ms = None;
        self
    }

    /// Run benchmarks in parallel.
    #[must_use]
    pub const fn parallel(mut self) -> Self {
        self.config.parallel = true;
        self
    }

    /// Run benchmarks sequentially.
    #[must_use]
    pub const fn sequential(mut self) -> Self {
        self.config.parallel = false;
        self
    }

    /// Enable detailed metrics collection.
    #[must_use]
    pub const fn detailed_metrics(mut self) -> Self {
        self.config.detailed_metrics = true;
        self
    }

    /// Set output format (json, csv, markdown, pretty).
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

    /// Build without validation (may produce invalid config).
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
    fn test_output_format_variants() {
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_eq!(OutputFormat::Csv, OutputFormat::Csv);
        assert_eq!(OutputFormat::Markdown, OutputFormat::Markdown);
        assert_eq!(OutputFormat::Pretty, OutputFormat::Pretty);
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
    fn test_profiler_builder_empty_chain_builds_defaults() {
        let config = ProfilerConfigBuilder::new().build().expect("valid");
        assert_eq!(config.warmup_iterations, 10);
        assert_eq!(config.benchmark_iterations, 100);
    }
}
