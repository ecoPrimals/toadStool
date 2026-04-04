// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Resource limit specification
///
/// Follows Kubernetes-style resource specification with requests and limits.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceLimit {
    /// Resource limit (maximum)
    pub limit: Option<String>,

    /// Resource request (minimum/guaranteed)
    pub request: Option<String>,
}

/// Base resource configuration
///
/// Provides common CPU, memory, and storage resource specifications.
/// Can be extended for domain-specific resource types.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseResourceConfig {
    /// CPU resource limits
    #[serde(default)]
    pub cpu: ResourceLimit,

    /// Memory resource limits
    #[serde(default)]
    pub memory: ResourceLimit,

    /// Storage resource limits (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<ResourceLimit>,
}

/// Base validation configuration
///
/// Provides common validation parameters for tokens, certificates,
/// and other security-related validation operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable validation
    #[serde(default = "crate::config_bases::serde_defaults::default_true")]
    pub enabled: bool,

    /// Validate expiration timestamps
    #[serde(default = "crate::config_bases::serde_defaults::default_true")]
    pub validate_expiration: bool,

    /// Clock skew tolerance for time-based validation
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "humantime_serde_optional"
    )]
    pub clock_skew: Option<Duration>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            validate_expiration: true,
            clock_skew: Some(Duration::from_secs(60)),
        }
    }
}

// Serde helper for optional Duration
mod humantime_serde_optional {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    #[allow(clippy::ref_option)] // Required by serde derive macro
    pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(duration) => humantime_serde::serialize(duration, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<Duration>::deserialize(deserializer)
    }
}
