// SPDX-License-Identifier: AGPL-3.0-only
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
    pub token: AuthenticationToken,
    pub source_primal: String,
    pub target_primal: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
    pub signature: String,
}

/// Token propagation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenPropagationStatus {
    Success,
    Failed(String),
    Pending,
    Skipped(String),
}

/// Result of token propagation across Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationResult {
    pub total_primals: usize,
    pub successful_propagations: usize,
    pub results: HashMap<String, TokenPropagationStatus>,
    pub token_id: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub propagation_time: SystemTime,
}

/// Result of token verification across Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub total_primals: usize,
    pub valid_tokens: usize,
    pub results: HashMap<String, TokenVerificationStatus>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub verification_time: SystemTime,
}

/// Primal type configuration enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimalTypeConfig {
    ToadStool(ToadStoolConfig),
    Songbird(SongbirdConfig),
    BearDog(BearDogConfig),
    NestGate(NestGateConfig),
    Squirrel(SquirrelConfig),
    BiomeOS(BiomeOSConfig),
}
