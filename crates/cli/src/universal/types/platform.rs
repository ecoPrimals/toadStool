// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform detection types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use toadstool_distributed::substrate_detection::{PlatformType, SubstrateCapabilities};

/// Information about a detected platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPlatform {
    /// Platform type (docker, wasm, native, etc.)
    pub platform_type: PlatformType,
    /// Substrate capabilities
    pub capabilities: SubstrateCapabilities,
    /// Availability status
    pub status: PlatformStatus,
    /// Performance score from benchmark (if run)
    pub performance_score: Option<f64>,
    /// When the platform was last tested
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    pub last_tested: Option<std::time::SystemTime>,
    /// Additional metadata (`Arc<str>` for zero-copy clone)
    pub metadata: HashMap<String, Arc<str>>,
}

/// Platform availability status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformStatus {
    /// Platform is available for workloads
    Available,
    /// Platform is being tested
    Testing,
    /// Platform is degraded but usable
    Degraded,
    /// Platform is unavailable
    Unavailable,
    /// Error with message
    Error(String),
}
