//! # Secure Enclave Runtime for ToadStool
//!
//! This crate provides **zero-knowledge compute** capabilities for ToadStool,
//! enabling privacy-preserving computation where the compute provider cannot
//! access plaintext data.
//!
//! ## Architecture
//!
//! The secure enclave runtime implements the pattern:
//! ```text
//! Compressed Data (NestGate) → Encrypted (BearDog) → Isolated Compute → Re-encrypted Result
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

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)] // Common in this domain

// Public modules
pub mod audit;
pub mod decompression;
pub mod error;
pub mod isolated_memory;
pub mod key_store;
pub mod runtime;

// Re-exports
pub use audit::{AuditEvent, AuditEventType, AuditLogger};
pub use decompression::{decompress_isolated, CompressionAlgorithm, DecompressionStats};
pub use error::{Error, Result};
pub use isolated_memory::IsolatedMemoryRegion;
pub use key_store::EphemeralKeyStore;
pub use runtime::SecureEnclaveRuntime;

