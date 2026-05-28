// SPDX-License-Identifier: AGPL-3.0-or-later
//! Profiler configuration builder.

use serde::{Deserialize, Serialize};

use toadstool_common::interned_strings::socket_env;

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
            warmup_iterations: env::var(socket_env::TOADSTOOL_PROFILER_WARMUP)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            benchmark_iterations: env::var(socket_env::TOADSTOOL_PROFILER_BENCH_ITERS)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            timeout_ms: env::var(socket_env::TOADSTOOL_PROFILER_TIMEOUT_MS)
                .ok()
                .and_then(|s| s.parse().ok()),
            parallel: env::var(socket_env::TOADSTOOL_PROFILER_PARALLEL)
                .is_ok_and(|s| s == "true" || s == "1"),
            detailed_metrics: env::var(socket_env::TOADSTOOL_PROFILER_DETAILED)
                .is_ok_and(|s| s == "true" || s == "1"),
            output_format: env::var(socket_env::TOADSTOOL_PROFILER_OUTPUT)
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
#[path = "profiler_tests.rs"]
mod tests;
