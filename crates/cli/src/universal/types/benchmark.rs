// SPDX-License-Identifier: AGPL-3.0-only
//! Benchmarking types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Duration;

use super::SystemInfo;

/// Result of a benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Platform benchmarked (native, wasm, docker, etc.)
    pub platform: String,
    /// Benchmark suite name
    pub suite: String,
    /// When the benchmark started
    #[serde(with = "toadstool_common::system_time_serde")]
    pub started: std::time::SystemTime,
    /// Total duration
    pub duration: Duration,
    /// Individual test results
    pub tests: Vec<BenchmarkTest>,
    /// Aggregated score
    pub overall_score: f64,
    /// System info at benchmark time
    pub system_info: SystemInfo,
}

/// Individual benchmark test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTest {
    /// Test name
    pub name: String,
    /// Type of benchmark (cpu, memory, gpu, etc.)
    pub test_type: BenchmarkType,
    /// Test duration
    pub duration: Duration,
    /// Score (higher is better, semantics depend on test)
    pub score: f64,
    /// Unit of the score (ops/sec, ms, etc.)
    pub unit: String,
    /// Additional details (JSON)
    pub details: HashMap<String, serde_json::Value>,
}

/// Type of benchmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkType {
    /// CPU integer arithmetic
    CpuInteger,
    /// CPU floating-point arithmetic
    CpuFloat,
    /// Memory bandwidth/latency
    Memory,
    /// Storage I/O
    Storage,
    /// Network throughput
    Network,
    /// GPU compute
    Gpu,
    /// WASM execution speed
    WasmExecution,
    /// Container startup time
    ContainerStartup,
    /// Custom benchmark type
    Custom(String),
}
