// SPDX-License-Identifier: AGPL-3.0-or-later
//! # biomeOS Integration for Universal Orchestration
//!
//! This module provides comprehensive biomeOS integration capabilities for orchestrating
//! the core services (orchestration, coordination, security, storage, intelligence) from a single
//! `biome.yaml` manifest file.
//!
//! ## Phase 4 Features
//!
//! - **Single manifest orchestration**: `biome.yaml` configures all Primals
//! - **Zero-configuration deployment**: Auto-discovery and configuration
//! - **Cross-Primal authentication**: Token propagation via capability discovery
//! - **Automated provisioning**: Storage and AI agent deployment
//! - **Sub-60-second bootstrap**: Optimized startup performance
//!
//! ## Module Structure
//!
//! - `types` - Core configuration and state types
//! - `auth` - Cross-Primal authentication manager (uses `auth_backend`)
//! - `auth_backend` - Trait-based auth backends (legacy direct endpoints, in-memory)
//! - `auth_backend_evolved` - Capability-based auth backend (RECOMMENDED)
//! - `storage` - Storage provisioning manager (uses `storage_backend`)
//! - `storage_backend` - Trait-based storage backends (legacy direct storage, in-memory)
//! - `storage_backend_evolved` - Capability-based storage backend (RECOMMENDED)
//! - `agents` - AI agent deployment manager (uses `agent_backend`)
//! - `agent_backend` - Trait-based agent backends (legacy intelligence endpoint, in-memory)
//! - `agent_backend_evolved` - Capability-based agent backend (RECOMMENDED)
//!
//! ## Deep Debt Evolution
//!
//! The `*_evolved` modules represent the new capability-based architecture:
//! - Runtime discovery instead of hardcoded primal names
//! - Proper error handling with thiserror
//! - Semantic method naming (wateringHole standard)
//! - Zero `unwrap()` in production code

// Module declarations
/// Legacy agent backend (trait-based, hardcoded providers).
pub mod agent_backend;
/// Capability-based agent backend (recommended).
pub mod agent_backend_evolved;
/// AI agent deployment manager
pub mod agents;
/// Cross-Primal authentication manager
pub mod auth;
/// Legacy auth backends (trait-based)
pub mod auth_backend;
/// Capability-based auth backend (recommended)
#[cfg(test)]
pub mod auth_backend_evolved;
/// Storage provisioning manager
pub mod storage;
/// Legacy storage backends (trait-based)
pub mod storage_backend;
/// Capability-based storage backend (recommended)
#[cfg(test)]
pub mod storage_backend_evolved;
/// Core configuration and state types
pub mod types;

// Re-export legacy backends for backward compatibility
pub use agent_backend::{
    AgentBackend, AgentBackendDispatch, InMemoryAgentBackend, IntelligenceBackend,
};
pub use agents::*;
pub use auth::*;
#[cfg(any(test, feature = "test-mocks"))]
pub use auth_backend::InMemoryAuthBackend;
#[expect(
    deprecated,
    reason = "legacy auth backends during capability migration"
)]
pub use auth_backend::{AuthBackend, BearDogBackend, SecurityBackend};
pub use storage::*;
pub use storage_backend::{
    InMemoryBackend, SocketStorageBackend, StorageBackend, StorageBackendDispatch, VolumeStatus,
};
pub use types::*;

// Re-export evolved backends (RECOMMENDED for new code)
pub use agent_backend_evolved::AgentBackend as AgentBackendEvolved;
#[cfg(test)]
pub use auth_backend_evolved::AuthBackend as AuthBackendEvolved;
#[cfg(test)]
pub use storage_backend_evolved::StorageBackend as StorageBackendEvolved;
