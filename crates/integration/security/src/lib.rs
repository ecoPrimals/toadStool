// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security service entropy integration
//!
//! High-quality, human-mixed entropy for GPU random number generation.
//!
//! ## Philosophy
//!
//! Instead of rebuilding RNG from scratch (20+ operations, 4-6 weeks),
//! leverage the ecosystem security service's sovereign entropy system:
//! - **60% machine + 40% human** entropy mixing
//! - **Cryptographic-grade** seed quality
//! - **User sovereignty** over their randomness
//! - **Capability-based discovery** (no hardcoding!)
//!
//! ## Deep Debt Principles
//!
//! - ✅ **Capability Discovery**: Finds the security service at runtime (no hardcoding!)
//! - ✅ **Self-Knowledge**: Knows only itself, discovers entropy capability dynamically
//! - ✅ **Graceful Fallback**: Uses system entropy if the security service is unavailable
//! - ✅ **No Mocks in Production**: Real discovery, real entropy
//! - ✅ **Ecosystem Integration**: Leverages existing primal (not isolated)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_integration_security::EntropyClient;
//!
//! async fn example() -> Result<(), SecurityError> {
//!     let client = EntropyClient::discover().await?;
//!     let seed = client.generate_seed().await?;
//!     // Use seed for GPU random operations...
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![expect(
    clippy::multiple_crate_versions,
    reason = "transitive deps pull different minor versions"
)]

mod discovery;
mod error;
mod seed;
mod types;

pub use discovery::{EntropyClient, SeedRequest};
pub use error::SecurityError;
pub use seed::{EphemeralSeed, SeedQuality};
pub use types::{EntropyMixing, EntropySource};

/// Discover high-quality entropy via the security service (capability discovery)
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
/// - No entropy-capable security service found
/// - Connection fails
///
/// # Example
///
/// ```rust,ignore
/// use toadstool_integration_security::discover_entropy;
///
/// async fn example() -> Result<(), SecurityError> {
///     match discover_entropy().await {
///         Ok(client) => {
///             let seed = client.generate_seed().await?;
///         }
///         Err(_) => {
///             eprintln!("security entropy service unavailable, using system entropy");
///         }
///     }
///     Ok(())
/// }
/// ```
pub async fn discover_entropy() -> Result<EntropyClient, SecurityError> {
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
