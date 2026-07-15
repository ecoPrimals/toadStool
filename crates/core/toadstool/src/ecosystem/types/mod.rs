// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Ecosystem Types
//!
//! Core type definitions for ecosystem coordination and service integration.

mod config;
mod connection;
mod messaging;

pub use config::{DiscoveryMethodConfig, EcosystemConfig, EcosystemConfigBuilder, ServiceInstance};
#[cfg(all(feature = "networking", unix))]
pub use connection::TarpcClientWrapper;
pub use connection::{ServiceChannel, ServiceClient, ServiceStatus};
pub use messaging::{EcosystemMessage, EcosystemMessageType};

#[cfg(test)]
mod tests;
