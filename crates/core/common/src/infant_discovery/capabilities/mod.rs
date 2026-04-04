// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Capability-based discovery system - zero hardcoded primal names
//!
//! This module implements pure capability-based service discovery where
//! `ToadStool` discovers services by what they do, not by who they are.
//!
//! # Core Principle
//! **"Each primal knows only itself. Everything else is discovered."**
//!
//! # Layout
//!
//! The implementation is split for clarity:
//!
//! - **discovered** — discovered endpoints, metadata, health, discovery sources, and preferences.
//! - **discovery_traits** — `CapabilityDiscovery` and `DiscoveryError`.
//! - **substrate** — substrate detection (`DetectedSubstrate`, `SubstrateCapability`, `SubstrateDetector`).
//! - **endpoint** — `EndpointResolver` and `EndpointSource`.
//! - **standard_capabilities** — stable string constants (`capabilities` submodule) for well-known names.
//!
//! Callers should keep importing from `infant_discovery::capabilities` (or parent re-exports); inner
//! module paths are not part of the public API surface.
//!
//! # Re-exports
//!
//! This module’s public items are also re-exported at the `infant_discovery` crate boundary for
//! convenience (`DiscoveryEngineBuilder`, integration tests, and CLI helpers). Those re-exports must
//! stay aligned with the `pub use` list below whenever types move between files.
//!
//! Unit tests live in `tests.rs` and exercise serialization, defaults, and error formatting.

mod discovered;
mod discovery_traits;
mod endpoint;
mod standard_capabilities;
mod substrate;

#[cfg(test)]
mod tests;

pub use discovered::{
    DiscoveredService, DiscoveryPreferences, DiscoverySource, ServiceHealth, ServiceMetadata,
};
pub use discovery_traits::{CapabilityDiscovery, DiscoveryError};
pub use endpoint::{EndpointResolver, EndpointSource};
pub use standard_capabilities::capabilities;
pub use substrate::{DetectedSubstrate, SubstrateCapability, SubstrateDetector, SubstrateType};
