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
//! ```rust,no_run
//! use toadstool_integration_beardog::EntropyClient;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Discover bearDog via capability discovery (no hardcoding!)
//! let client = EntropyClient::discover().await?;
//!
//! // Request high-quality seed (human-mixed entropy)
//! let seed = client.generate_seed().await?;
//!
//! // Use for GPU random operations
//! executor.uniform_random(0.0, 1.0, seed).await?;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

mod discovery;
mod seed;
mod types;

pub use discovery::EntropyClient;
pub use seed::{EphemeralSeed, SeedQuality};
pub use types::{EntropyMixing, EntropySource};

use anyhow::Result;

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
/// ```rust,no_run
/// # use toadstool_integration_beardog::discover_entropy;
/// # async fn example() -> anyhow::Result<()> {
/// match discover_entropy().await {
///     Ok(client) => {
///         // Use high-quality bearDog entropy
///         let seed = client.generate_seed().await?;
///     }
///     Err(_) => {
///         // Fallback to system entropy
///         let seed = rand::random();
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub async fn discover_entropy() -> Result<EntropyClient> {
    EntropyClient::discover().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_entropy_discovery() {
        // Should not panic - graceful if bearDog not available
        let result = discover_entropy().await;

        if let Ok(client) = result {
            // Verify client is functional
            assert!(client.is_available());
        } else {
            // OK if bearDog not running in test environment
            eprintln!("Note: bearDog not available for testing");
        }
    }
}
