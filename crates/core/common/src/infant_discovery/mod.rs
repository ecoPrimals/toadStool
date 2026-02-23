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
//!
//! # Evolution (Feb 15, 2026) - Songbird Delegation
//!
//! ToadStool focuses on **hardware capabilities** (GPU, NPU, CPU).
//! Network service discovery is **delegated to Songbird** (comms primal).
//!
//! **Removed**: Vendor-specific detectors (K8s, Docker, Consul, AWS/GCP/Azure)
//! **Kept**: `BareMetalDetector` (hardware capabilities), mDNS (exposed to Songbird)
//!
//! ## Separation of Concerns
//!
//! - **ToadStool**: Hardware discovery, compute routing, unified math language
//! - **Songbird**: Network discovery, service mesh, primal coordination
//!
//! This means ToadStool no longer attempts to detect Kubernetes, Docker,
//! or cloud providers. Those are Songbird's responsibility.

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
