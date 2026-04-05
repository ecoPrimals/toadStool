// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Ecosystem Discovery for Auto-Configuration
//!
//! Discovers available ecosystem services by capability and automatically
//! configures optimal integration settings.
//!
//! Use [`EcosystemDiscoverer::find_pattern_by_capability`] for
//! sovereignty-compliant capability-based lookup.

mod constants;
mod discoverer;
mod helpers;

#[cfg(test)]
mod tests;

// Re-export types for backward compatibility
pub use crate::ecosystem_types::{
    DiscoveredServices, DiscoverySummary, ServiceInfo, ServicePattern, ServiceStatus, ServiceType,
};

pub use discoverer::EcosystemDiscoverer;
