// SPDX-License-Identifier: AGPL-3.0-or-later
//! Homomorphic encryption schemes
//!
//! This module implements different homomorphic encryption schemes:
//! - BFV: Brakerski-Fan-Vercauteren (exact integer arithmetic)
//! - CKKS: Cheon-Kim-Kim-Song (approximate arithmetic, better for ML)

pub mod bfv;
pub mod ckks;

pub use bfv::BfvScheme;
pub use ckks::CkksScheme;

use anyhow::Result;

/// Trait for homomorphic encryption schemes
pub trait HomomorphicScheme {
    /// Encrypt a plaintext value
    fn encrypt(&self, plaintext: &[u64]) -> Result<Vec<u64>>;

    /// Decrypt a ciphertext value
    fn decrypt(&self, ciphertext: &[u64]) -> Result<Vec<u64>>;

    /// Homomorphic addition (on encrypted data)
    fn add(&self, a: &[u64], b: &[u64]) -> Result<Vec<u64>>;

    /// Homomorphic multiplication (on encrypted data)
    fn multiply(&self, a: &[u64], b: &[u64]) -> Result<Vec<u64>>;
}
