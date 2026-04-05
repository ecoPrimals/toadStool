// SPDX-License-Identifier: AGPL-3.0-only
//! Coordination adapter - capability-based coordination and service discovery
//!
//! This adapter replaces the hardcoded Songbird integration with a generic
//! coordination adapter that works with ANY service providing coordination capabilities.
//!
//! # Migration from Songbird
//! ```rust,ignore
//! // ❌ OLD: Hardcoded Songbird (services/songbird.rs)
//! use crate::ecosystem::services::songbird;
//! let response = songbird::send_registration(&addr, &registration).await?;
//!
//! // ✅ NEW: Capability-based (adapters/coordination.rs)
//! use crate::ecosystem::adapters::CoordinationAdapter;
//! let token = coordination.register_service(service_info).await?;
//! ```

mod adapter;
mod types;

pub use adapter::CoordinationAdapter;
pub use types::{LockHandle, PeerInfo, RegistrationToken, ServiceInfo};

#[cfg(test)]
mod tests;
