// SPDX-License-Identifier: AGPL-3.0-or-later
//! mDNS Discovery Client Implementation
//!
//! Implements multicast DNS (mDNS) service discovery for zero-config
//! capability-based service discovery in local networks.
//!
//! # Architecture
//!
//! - Uses mDNS (RFC 6762) for zero-config service discovery
//! - Services advertise capabilities via TXT records
//! - Automatic service detection without configuration
//! - Falls back gracefully when mDNS unavailable
//!
//! # Philosophy: "Each Primal Knows Only Itself"
//!
//! - Services broadcast WHAT they can do (capabilities)
//! - Consumers discover by WHAT they need (not WHO)
//! - No hardcoded knowledge of other services
//! - Runtime resolution of all dependencies
//!
//! # Submodules
//!
//! - `client` — discovery client and trait implementation.
//! - `parser` — TXT record parsing helpers (tests and future mdns-sd work).
//! - `tests` — integration-style unit tests for cache, discovery, and parsing.
//!
//! # Implementation status
//!
//! The public surface matches `DiscoveryClient`: discover, register, and health checks are
//! implemented against an in-memory cache so callers can integrate without a live mDNS stack.
//! Network browsing and long-lived advertisement remain future work (see `mdns-sd` on crates.io).
//!
//! # Dependencies
//!
//! This module depends on `tokio` for async locks, `async_trait` for the discovery trait, and
//! `toadstool-common` for capability and service types. Parsing logic is isolated so a future
//! network backend can reuse it without duplicating string conventions.
//!
//! The service type constant `MDNS_SERVICE_TYPE` is shared by registration paths once real
//! advertisement is wired up; callers outside this module should treat it as an implementation detail.

mod client;
#[cfg(test)]
mod parser;
#[cfg(test)]
mod tests;

pub use client::MdnsDiscoveryClient;

/// mDNS service type for ecoPrimals services
pub(crate) const MDNS_SERVICE_TYPE: &str = "_ecoprimals._tcp.local.";
