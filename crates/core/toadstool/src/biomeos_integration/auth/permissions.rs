// SPDX-License-Identifier: AGPL-3.0-or-later
//! Permission and propagation types

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::tokens::{AuthenticationToken, TokenVerificationStatus};
use crate::biomeos_integration::types::{
    BearDogConfig, BiomeOSConfig, NestGateConfig, SongbirdConfig, SquirrelConfig, ToadStoolConfig,
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

/// Primal type configuration enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimalTypeConfig {
    /// ToadStool orchestration primal.
    ToadStool(ToadStoolConfig),
    /// Songbird messaging primal.
    Songbird(SongbirdConfig),
    /// BearDog crypto primal.
    BearDog(BearDogConfig),
    /// NestGate storage primal.
    NestGate(NestGateConfig),
    /// Squirrel AI agent primal.
    Squirrel(SquirrelConfig),
    /// Full biomeOS manifest.
    BiomeOS(BiomeOSConfig),
}
