// SPDX-License-Identifier: AGPL-3.0-only
//! # Secure Enclave Runtime for ToadStool
//!
//! This crate provides **zero-knowledge compute** capabilities for ToadStool,
//! enabling privacy-preserving computation where the compute provider cannot
//! access plaintext data.
//!
//! ## Architecture (Node Atomic)
//!
//! Follows the Node Atomic pattern: BearDog (crypto) + Songbird (discovery) + ToadStool (compute).
//! All encryption/decryption is delegated to BearDog via JSON-RPC (`crypto.encrypt` / `crypto.decrypt`).
//! This crate owns **memory isolation only** — never bundles its own crypto primitives.
//!
//! ```text
//! Compressed Data (NestGate) → Encrypted (BearDog) → Isolated Compute → Re-encrypted Result (BearDog)
//! ```
//!
//! ## Core Guarantees
//!
//! 1. **Memory Isolation**: Plaintext never touches disk (mlock, madvise)
//! 2. **Key Ephemeral**: Keys wiped explicitly after use
//! 3. **Zero Disk I/O**: No writes during sensitive processing
//! 4. **Auditable**: Cryptographic proof of isolation
//! 5. **Provider Blind**: Only sees encrypted blobs (entropy > 7.95)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_runtime_secure_enclave::{SecureEnclaveRuntime, ComputeRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let runtime = SecureEnclaveRuntime::new()?;
//!     
//!     let result = runtime.process_encrypted(
//!         &encrypted_data,
//!         btsp_session,
//!         |plaintext| {
//!             // Your compute function here
//!             Ok(process(plaintext))
//!         },
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Security Model
//!
//! - **Threat Model**: Honest-but-curious cloud provider
//! - **Guarantees**: Computational, not information-theoretic
//! - **Assumptions**: BTSP channel secure, crypto primitives sound
//!
//! ## Performance
//!
//! - **Overhead**: < 10% vs plaintext compute
//! - **Energy**: 70-80% savings from pre-compression (NestGate)
//! - **Latency**: Decompression ~5ms/MB, encryption ~2ms/MB

#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)] // Transitive dep conflicts not actionable

// Public modules
pub mod audit;
pub mod decompression;
pub mod error;
pub mod isolated_memory;
pub mod key_store;
pub mod runtime;

// Re-exports
pub use audit::{AuditEvent, AuditEventType, AuditLogger};
pub use decompression::{CompressionAlgorithm, DecompressionStats, decompress_isolated};
pub use error::{Error, Result};
pub use isolated_memory::IsolatedMemoryRegion;
pub use key_store::EphemeralKeyStore;
pub use runtime::SecureEnclaveRuntime;
