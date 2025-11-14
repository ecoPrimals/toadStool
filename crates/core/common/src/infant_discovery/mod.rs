// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! Infant Discovery System
//!
//! This module implements `ToadStool`'s "infant discovery" pattern where services
//! start with zero knowledge and discover everything dynamically through capabilities.
//!
//! # Core Principle
//! **"Each primal knows only itself. Everything else is discovered."**
//!
//! This eliminates all hardcoded service names, URLs, and ports while maintaining
//! perfect interoperability.

pub mod capabilities;
pub mod detectors;
pub mod engine;
pub mod sources;

// Re-export key types for convenience
pub use capabilities::{
    CapabilityDiscovery, DetectedSubstrate, DiscoveredService, DiscoveryError,
    DiscoveryPreferences, DiscoverySource, EndpointResolver, EndpointSource, ServiceHealth,
    ServiceMetadata, SubstrateCapability, SubstrateDetector, SubstrateType,
};
pub use engine::{DiscoveryEngine, DiscoveryEngineBuilder, ServiceDiscoveryConfig};
// Type alias for backward compatibility
pub use engine::ServiceDiscoveryConfig as DiscoveryConfig;
