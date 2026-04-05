// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Primal Capabilities Loader
//!
//! **Universal, Agnostic, Sovereignty-First**
//!
//! This module provides runtime loading of primal capabilities from `primal-capabilities.toml`,
//! eliminating ALL hardcoded endpoint/port references.
//!
//! ## Philosophy
//!
//! **"Each primal knows only itself. Everything else is discovered."**
//!
//! - **Self-Knowledge**: Toadstool knows what IT can do
//! - **Runtime Discovery**: Find other primals by capability, not name
//! - **No Hardcoding**: Zero assumptions about other primals
//! - **Capability-Based**: Discover by WHAT you need, not WHO
//!
//! ## Usage
//!
//! ```no_run
//! # use toadstool_config::primal_capabilities::*;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load capabilities at runtime
//! let registry = PrimalCapabilitiesRegistry::load_from_file("primal-capabilities.toml")?;
//!
//! // Find primals by what they can do, not their name
//! let crypto_services = registry.find_by_capability("cryptographic-operations");
//! let storage_services = registry.find_by_capability("storage");
//!
//! // Get endpoint for first available service
//! if let Some(crypto) = crypto_services.first() {
//!     let endpoint = registry.get_endpoint(crypto, "localhost")?;
//!     println!("Crypto service: {}", endpoint);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Submodules
//!
//! - `parsing` — TOML file loading, serde types, and default search paths.
//! - `registry` — Capability/role lookup, endpoint URLs, and self-knowledge helper.

mod parsing;
mod registry;

pub use parsing::{
    CapabilityError, CapabilityResult, DiscoveryConfig, DiscoveryPreferences, MigrationMapping,
    PrimalCapabilitiesRegistry, PrimalDefinition, RegistryMetadata,
};
pub use registry::get_self_capabilities;

#[cfg(test)]
mod tests;
