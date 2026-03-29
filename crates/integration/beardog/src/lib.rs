// SPDX-License-Identifier: AGPL-3.0-only
//! bearDog Entropy Integration
//!
//! High-quality, human-mixed entropy for GPU random number generation.
//!
//! ## Philosophy
//!
//! Instead of rebuilding RNG from scratch (20+ operations, 4-6 weeks),
//! leverage bearDog's sovereign entropy system:
//! - **60% machine + 40% human** entropy mixing
//! - **Cryptographic-grade** seed quality
//! - **User sovereignty** over their randomness
//! - **Capability-based discovery** (no hardcoding!)
//!
//! ## Deep Debt Principles
//!
//! - ✅ **Capability Discovery**: Finds bearDog at runtime (no hardcoding!)
//! - ✅ **Self-Knowledge**: Knows only itself, discovers bearDog dynamically
//! - ✅ **Graceful Fallback**: Uses system entropy if bearDog unavailable
//! - ✅ **No Mocks in Production**: Real discovery, real entropy
//! - ✅ **Ecosystem Integration**: Leverages existing primal (not isolated)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_integration_beardog::EntropyClient;
//!
//! async fn example() -> Result<(), BeardogError> {
//!     let client = EntropyClient::discover().await?;
//!     let seed = client.generate_seed().await?;
//!     // Use seed for GPU random operations...
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::multiple_crate_versions)]

mod discovery;
mod error;
mod seed;
mod types;

pub use discovery::{EntropyClient, SeedRequest};
pub use error::BeardogError;
pub use seed::{EphemeralSeed, SeedQuality};
pub use types::{EntropyMixing, EntropySource};

/// Discover bearDog entropy service via capability discovery
///
/// Searches for services advertising "capability:entropy:high-quality".
/// Returns client if found, error if unavailable.
///
/// ## Deep Debt Compliance
///
/// - No hardcoded URLs or ports
/// - Discovers service at runtime
/// - Graceful fallback if not found
///
/// # Errors
///
/// Returns error if:
/// - No service discovery available
/// - No bearDog service found
/// - Connection fails
///
/// # Example
///
/// ```rust,ignore
/// use toadstool_integration_beardog::discover_entropy;
///
/// async fn example() -> Result<(), BeardogError> {
///     match discover_entropy().await {
///         Ok(client) => {
///             let seed = client.generate_seed().await?;
///         }
///         Err(_) => {
///             eprintln!("bearDog unavailable, using system entropy");
///         }
///     }
///     Ok(())
/// }
/// ```
pub async fn discover_entropy() -> Result<EntropyClient, BeardogError> {
    EntropyClient::discover().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_fallback() {
        // Test entropy fallback path (avoids live network discovery that causes
        // nested runtime panics in test environment)
        let seed = EntropyClient::system_entropy_fallback();

        // System entropy should always produce data
        assert!(
            !seed.seed_data.is_empty(),
            "System entropy fallback should produce data"
        );
        assert_eq!(seed.source, EntropySource::Machine);
    }
}
