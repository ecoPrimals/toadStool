// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ecosystem Integration - Sovereign Science Network
//!
//! Integration with the ecoPrimals ecosystem for distributed sovereign computing:
//! - Coordination: Service discovery and orchestration
//! - Security: Cryptographic permissions and PKI
//! - Storage: Distributed storage and data management
//!
//! ## Module Structure (Refactored by Protocol)
//!
//! - `types`: Type definitions (EcosystemIntegrator, ServiceEndpoint, etc.)
//! - `adapters/`: Capability-based service adapters (crypto, coordination, storage)
//! - `capabilities/`: Capability resolution and discovery
//! - `discovery`: Service discovery and scanning logic
//! - `connection`: Connection management
//! - `integrator_impl`: Core EcosystemIntegrator implementation

// Public modules
pub mod adapters;
pub mod capabilities;
pub mod config;
pub mod constants; // Zero-copy constants
pub mod service_type;
pub mod services;
pub mod types;

// Internal modules
mod connection;
pub mod discovery;

// Public re-exports
#[expect(
    deprecated,
    reason = "re-exporting deprecated EcosystemService for backward compatibility"
)]
pub use types::{
    BearDogPermission, CryptoVerificationContext, DiscoveredService, DiscoveryResult,
    EcosystemIntegrator, EcosystemService, NestGateMount, SecurityPermission, ServiceEndpoint,
    ServiceSignature, ServiceType, SignedServiceResponse, StorageMount, TrustLevel,
};

// Internal types
use types::{ConnectionStatus, EcosystemStatus, ServiceConnection};

impl Default for EcosystemIntegrator {
    fn default() -> Self {
        Self::new()
    }
}

// Include the implementation
include!("integrator_impl.rs");

#[cfg(test)]
mod tests;
