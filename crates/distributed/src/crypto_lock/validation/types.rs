// SPDX-License-Identifier: AGPL-3.0-only
//! Domain types for cryptographic validation (proofs, algorithms, outcomes).

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Security provider cryptographic proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProof {
    /// Cryptographic signature
    pub signature: Vec<u8>,
    /// Signature algorithm used
    pub algorithm: CryptoAlgorithm,
    /// Public key identifier
    pub public_key_id: String,
    /// Proof timestamp
    pub timestamp: SystemTime,
    /// Additional proof metadata
    pub metadata: ProofMetadata,
}

/// Cryptographic algorithms supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CryptoAlgorithm {
    /// Ed25519 signatures.
    Ed25519,
    /// ECDSA P-256.
    EcdsaP256,
    /// RSA-4096.
    Rsa4096,
    /// BearDog-specific custom algorithm.
    BearDogCustom,
}

/// Proof metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Entity that issued the proof.
    pub issuer: String,
    /// Intended use or audience for the proof.
    pub purpose: String,
    /// Additional string claims attached to the proof.
    pub additional_claims: HashMap<String, String>,
}

/// Permission validation result
#[derive(Debug, Clone)]
pub enum PermissionValidationResult {
    /// Permission is valid for use.
    Valid,
    /// Permission failed validation (e.g. bad signature).
    Invalid,
    /// Permission is outside its valid time window.
    Expired,
    /// Permission was revoked.
    Revoked,
}

/// Verification level for identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationLevel {
    /// No verification performed.
    Unverified,
    /// Email ownership verified.
    EmailVerified,
    /// Government or KYC-style identity verified.
    IdentityVerified,
    /// Institution affiliation verified.
    InstitutionVerified,
}

/// Security provider public key (opaque key material for verification).
pub struct SecurityPublicKey {
    /// Raw key bytes (e.g. 32-byte ed25519 public key).
    pub bytes: Vec<u8>,
    /// Algorithm identifier.
    pub algorithm: String,
}
