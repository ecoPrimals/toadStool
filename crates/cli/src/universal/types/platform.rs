// SPDX-License-Identifier: AGPL-3.0-only
//! Platform detection types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toadstool_distributed::substrate_detection::{PlatformType, SubstrateCapabilities};

/// Information about a detected platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPlatform {
    pub platform_type: PlatformType,
    pub capabilities: SubstrateCapabilities,
    pub status: PlatformStatus,
    pub performance_score: Option<f64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    pub last_tested: Option<std::time::SystemTime>,
    pub metadata: HashMap<String, String>,
}

/// Platform availability status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformStatus {
    Available,
    Testing,
    Degraded,
    Unavailable,
    Error(String),
}
