// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Platform information for distributed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Platform name
    pub name: String,
    /// Platform version
    pub version: String,
    /// Architecture
    pub architecture: String,
    /// Available features
    pub features: Vec<String>,
    /// Capabilities
    pub capabilities: HashMap<String, bool>,
}

impl Default for PlatformInfo {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            version: "unknown".to_string(),
            architecture: "unknown".to_string(),
            features: Vec::new(),
            capabilities: HashMap::new(),
        }
    }
}

/// Platform detection utilities
pub struct PlatformDetector;

impl PlatformDetector {
    /// Detect current platform
    #[must_use]
    pub fn detect() -> PlatformInfo {
        PlatformInfo {
            name: std::env::consts::OS.to_string(),
            version: "unknown".to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            features: vec!["basic".to_string()],
            capabilities: HashMap::new(),
        }
    }
}
