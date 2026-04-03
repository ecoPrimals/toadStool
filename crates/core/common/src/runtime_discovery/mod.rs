// SPDX-License-Identifier: AGPL-3.0-only
//! Runtime Service Discovery - Zero Hardcoding
//!
//! This module provides runtime discovery of services based on capabilities,
//! completely eliminating hardcoded primal names and URLs.
//!
//! ## Design Principles
//!
//! 1. **Capability-Based**: Find services by what they can do, not who they are
//! 2. **Runtime Discovery**: No compile-time knowledge of other services
//! 3. **Protocol Agnostic**: Support multiple discovery protocols
//! 4. **Fallback Strategy**: Graceful degradation when discovery unavailable

mod cache;
mod client;
mod localhost;
mod service;

pub use client::DiscoveryClient;
pub use localhost::LocalhostDiscoveryClient;
pub use service::RuntimeDiscovery;
