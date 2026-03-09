// SPDX-License-Identifier: AGPL-3.0-only
//! Benchmarking types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Duration;

use super::SystemInfo;

/// Result of a benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub platform: String,
    pub suite: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub started: std::time::SystemTime,
    pub duration: Duration,
    pub tests: Vec<BenchmarkTest>,
    pub overall_score: f64,
    pub system_info: SystemInfo,
}

/// Individual benchmark test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTest {
    pub name: String,
    pub test_type: BenchmarkType,
    pub duration: Duration,
    pub score: f64,
    pub unit: String,
    pub details: HashMap<String, serde_json::Value>,
}

/// Type of benchmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkType {
    CpuInteger,
    CpuFloat,
    Memory,
    Storage,
    Network,
    Gpu,
    WasmExecution,
    ContainerStartup,
    Custom(String),
}
