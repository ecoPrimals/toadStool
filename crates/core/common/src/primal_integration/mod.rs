// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inter-Primal Integration Discovery
//!
//! Runtime discovery of ecoPrimal services by capability rather than by primal name.
//!
//! ## Capability-Based Philosophy
//!
//! **Primals only have self-knowledge and discover other primals at runtime.**
//! ToadStool does not hardcode which primal provides which capability. Discovery is
//! by capability (e.g., `security`, `storage`, `orchestration`), not by primal name
//! (e.g., beardog, nestgate, songbird). Any service may advertise a capability.
//!
//! ## Deep Debt Principles
//!
//! 1. **Self-Knowledge Only**: ToadStool knows only itself
//! 2. **Runtime Discovery**: Other primals discovered at deployment time
//! 3. **No Hardcoding**: Zero hardcoded addresses/ports/names
//! 4. **Capability-Based**: Discover by capability, not by name
//! 5. **Graceful Degradation**: Works with or without other primals
//!
//! ## Integration Patterns
//!
//! ### bearDog Integration (Encryption)
//!
//! ```ignore
//! use toadstool_common::primal_integration::*;
//!
//! // Discover bearDog at runtime
//! let beardog = discover_encryption_service().await?;
//!
//! // Use encryption capability
//! let encrypted = beardog.encrypt(data).await?;
//! ```
//!
//! ### nestGate Integration (Compression/Persistence)
//!
//! ```ignore
//! use toadstool_common::primal_integration::*;
//!
//! // Discover nestGate at runtime
//! let nestgate = discover_storage_service().await?;
//!
//! // Use compression capability
//! let compressed = nestgate.compress(data).await?;
//!
//! // Use persistence capability
//! nestgate.store(key, value).await?;
//! ```
//!
//! ### songBird Integration (Coordination)
//!
//! ```ignore
//! use toadstool_common::primal_integration::*;
//!
//! // Discover songBird at runtime
//! let songbird = discover_coordination_service().await?;
//!
//! // Register capabilities
//! songbird.register_capabilities(capabilities).await?;
//! ```
//!
//! ## Discovery Methods
//!
//! 1. **Environment Variables** (highest priority) - capability-based, no primal names
//!    - `TOADSTOOL_ENCRYPTION_ENDPOINT` (crypto capability)
//!    - `TOADSTOOL_STORAGE_ENDPOINT` (storage capability)
//!    - `TOADSTOOL_COORDINATION_ENDPOINT` (coordination capability)
//!    - `TOADSTOOL_MCP_ENDPOINT` (mcp capability)
//!
//! 2. **mDNS/DNS-SD** (local network) - discover by capability tag
//!    - `_encryption._tcp.local.`
//!    - `_storage._tcp.local.`
//!    - `_coordination._tcp.local.`
//!
//! 3. **Kubernetes Service Discovery** - discover by label/capability
//!    - Query by capability label, not by service name
//!
//! 4. **Docker Compose** - discover via service mesh labels
//!
//! 5. **Runtime Registry** (consul, etcd)
//!    - Query by capability tag
//!    - Load balance across instances
//!
//! ## External System Integration
//!
//! ToadStool can also discover and use non-ecoPrimal services:
//!
//! - **Redis**: Cache, pub/sub
//! - **`PostgreSQL`**: Relational data
//! - **S3-compatible**: Object storage
//! - **Custom Services**: Via generic HTTP/gRPC discovery
//!
//! ```ignore
//! use toadstool_common::primal_integration::*;
//!
//! // Discover external services
//! let cache = discover_cache_service().await?;
//! let db = discover_database_service().await?;
//! let storage = discover_object_storage().await?;
//! ```

use serde::{Deserialize, Serialize};

// ============================================================================
// Types (kept in mod.rs for public API surface)
// ============================================================================

/// Service endpoint discovered at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalEndpoint {
    /// Service identifier (e.g., "beardog-1", "nestgate-primary")
    pub service_id: String,

    /// Base URL; discovered via capability resolution at runtime.
    pub url: String,

    /// Capabilities this service provides
    pub capabilities: Vec<String>,

    /// Health status
    pub healthy: bool,

    /// Last health check timestamp
    pub last_check: std::time::SystemTime,
}

/// Discovery result with multiple endpoints (for load balancing)
pub type DiscoveryResult = Result<Vec<PrimalEndpoint>, DiscoveryError>;

/// Discovery errors
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// No service advertising the capability was found
    #[error("No service found with capability: {capability}")]
    NoServiceFound {
        /// Capability that was requested
        capability: String,
    },

    /// Discovered service failed health check
    #[error("Service unhealthy: {service_id}")]
    ServiceUnhealthy {
        /// Service identifier
        service_id: String,
    },

    /// Discovery method (mDNS, env, etc.) failed
    #[error("Discovery method failed: {method}: {reason}")]
    DiscoveryFailed {
        /// Discovery method that failed
        method: String,
        /// Failure reason
        reason: String,
    },

    /// Network-level error during discovery
    #[error("Network error: {0}")]
    Network(String),
}

// ============================================================================
// Submodules
// ============================================================================

pub mod capabilities;

mod discovery;
mod socket;

// ============================================================================
// Re-exports (backward compatibility)
// ============================================================================

pub use discovery::{
    discover_cache_service, discover_coordination_service, discover_database_service,
    discover_encryption_service, discover_mcp_service, discover_object_storage,
    discover_service_by_capability, discover_storage_service,
};
pub use socket::discover_service_socket_by_capability;

#[cfg(test)]
mod tests;
