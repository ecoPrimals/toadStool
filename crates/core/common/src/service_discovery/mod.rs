//! Service Discovery - Capability-Based Runtime Discovery
//!
//! Discover services at runtime based on capabilities, not hardcoded names.

mod endpoint;
mod service;
#[cfg(test)]
mod tests;
mod types;

pub use service::ServiceDiscovery;
pub use types::{
    DiscoveredService, DiscoveryError, DiscoveryMethod, DiscoveryResult, ServiceDiscoveryTrait,
};
