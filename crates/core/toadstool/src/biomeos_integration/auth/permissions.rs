// SPDX-License-Identifier: AGPL-3.0-or-later
//! Permission and propagation types

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use toadstool_common::interned_strings::biomeos_manifest_serde as manifest_serde;

use super::tokens::{AuthenticationToken, TokenVerificationStatus};
use crate::biomeos_integration::types::{
    BiomeOSConfig, CoordinationConfig, IntelligenceConfig, SecurityConfig, StorageConfig,
    ToadStoolConfig,
};

/// Token propagation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPropagationRequest {
    /// Token to propagate.
    pub token: AuthenticationToken,
    /// Primal originating the token.
    pub source_primal: String,
    /// Primal to receive the token.
    pub target_primal: String,
    /// Request timestamp for replay protection.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
    /// Ed25519 signature over the propagation payload.
    pub signature: String,
}

/// Token propagation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenPropagationStatus {
    /// Token propagated successfully.
    Success,
    /// Propagation failed with error message.
    Failed(String),
    /// Propagation in progress.
    Pending,
    /// Propagation skipped (e.g. primal unreachable).
    Skipped(String),
}

/// Result of token propagation across Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationResult {
    /// Total number of target Primals.
    pub total_primals: usize,
    /// Number of successful propagations.
    pub successful_propagations: usize,
    /// Per-primal propagation status.
    pub results: HashMap<String, TokenPropagationStatus>,
    /// Token ID that was propagated.
    pub token_id: String,
    /// When propagation completed.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub propagation_time: SystemTime,
}

/// Result of token verification across Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Total number of Primals verified.
    pub total_primals: usize,
    /// Number with valid tokens.
    pub valid_tokens: usize,
    /// Per-primal verification status.
    pub results: HashMap<String, TokenVerificationStatus>,
    /// When verification completed.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub verification_time: SystemTime,
}

/// BiomeOS manifest sections by service capability (legacy primal manifest keys via
/// [`toadstool_common::interned_strings::biomeos_manifest_serde`]).
#[derive(Debug, Clone)]
pub enum PrimalTypeConfig {
    /// ToadStool orchestration primal.
    ToadStool(ToadStoolConfig),
    /// Coordination / discovery service.
    Coordination(CoordinationConfig),
    /// Security / crypto service.
    SecurityService(SecurityConfig),
    /// Storage service.
    StorageService(StorageConfig),
    /// Intelligence / ML agent service.
    IntelligenceService(IntelligenceConfig),
    /// Full biomeOS manifest.
    BiomeOS(BiomeOSConfig),
}

impl Serialize for PrimalTypeConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::ToadStool(c) => map.serialize_entry(manifest_serde::TOADSTOOL, c)?,
            Self::Coordination(c) => map.serialize_entry(manifest_serde::COORDINATION, c)?,
            Self::SecurityService(c) => map.serialize_entry(manifest_serde::SECURITY_SERVICE, c)?,
            Self::StorageService(c) => map.serialize_entry(manifest_serde::STORAGE_SERVICE, c)?,
            Self::IntelligenceService(c) => {
                map.serialize_entry(manifest_serde::INTELLIGENCE_SERVICE, c)?;
            }
            Self::BiomeOS(c) => map.serialize_entry(manifest_serde::BIOME_OS, c)?,
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for PrimalTypeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let map = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
        if map.len() != 1 {
            return Err(D::Error::custom(format!(
                "expected exactly one primal manifest key, found {}",
                map.len()
            )));
        }
        let Some((k, v)) = map.into_iter().next() else {
            return Err(D::Error::custom("empty manifest map after len check"));
        };
        match k.as_str() {
            manifest_serde::TOADSTOOL => ToadStoolConfig::deserialize(v)
                .map(Self::ToadStool)
                .map_err(D::Error::custom),
            manifest_serde::COORDINATION
            | manifest_serde::LEGACY_SONGBIRD_PASCAL
            | manifest_serde::LEGACY_SONGBIRD_LOWER => CoordinationConfig::deserialize(v)
                .map(Self::Coordination)
                .map_err(D::Error::custom),
            manifest_serde::SECURITY_SERVICE
            | manifest_serde::LEGACY_BEARDOG_PASCAL
            | manifest_serde::LEGACY_BEARDOG_CAMEL
            | manifest_serde::LEGACY_BEARDOG_LOWER => SecurityConfig::deserialize(v)
                .map(Self::SecurityService)
                .map_err(D::Error::custom),
            manifest_serde::STORAGE_SERVICE
            | manifest_serde::LEGACY_NESTGATE_PASCAL
            | manifest_serde::LEGACY_NESTGATE_LOWER
            | manifest_serde::LEGACY_NESTGATE_KEBAB => StorageConfig::deserialize(v)
                .map(Self::StorageService)
                .map_err(D::Error::custom),
            manifest_serde::INTELLIGENCE_SERVICE
            | manifest_serde::LEGACY_SQUIRREL_PASCAL
            | manifest_serde::LEGACY_SQUIRREL_LOWER => IntelligenceConfig::deserialize(v)
                .map(Self::IntelligenceService)
                .map_err(D::Error::custom),
            manifest_serde::BIOME_OS => BiomeOSConfig::deserialize(v)
                .map(Self::BiomeOS)
                .map_err(D::Error::custom),
            other => Err(D::Error::custom(format!(
                "unknown primal manifest tag: {other}"
            ))),
        }
    }
}
