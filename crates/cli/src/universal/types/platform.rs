// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform detection types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
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
    /// Arc<str> values = zero-copy clone (serde rc feature)
    pub metadata: HashMap<String, Arc<str>>,
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
