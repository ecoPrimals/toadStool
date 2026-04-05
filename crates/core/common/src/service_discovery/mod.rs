// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service Discovery - Capability-Based Runtime Discovery
//!
//! Discover services at runtime based on capabilities, not hardcoded names.

mod config;
mod discovery_config;
#[cfg(feature = "mdns")]
mod discovery_mdns;
mod discovery_registry;
mod endpoint;
mod service;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_registry;
mod types;

pub use service::ServiceDiscovery;
pub use types::{
    DiscoveredService, DiscoveryError, DiscoveryMethod, DiscoveryResult, ServiceDiscoveryTrait,
};
