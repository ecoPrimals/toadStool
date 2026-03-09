// SPDX-License-Identifier: AGPL-3.0-only
//! # biomeOS Integration for Universal Orchestration
//!
//! This module provides comprehensive biomeOS integration capabilities for orchestrating
//! all 5 Primals (`ToadStool`, Songbird, `BearDog`, `NestGate`, Squirrel) from a single
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
//! - `auth_backend` - Trait-based auth backends (legacy: BearDog, in-memory)
//! - `auth_backend_evolved` - Capability-based auth backend (RECOMMENDED)
//! - `storage` - Storage provisioning manager (uses `storage_backend`)
//! - `storage_backend` - Trait-based storage backends (legacy: NestGate, in-memory)
//! - `storage_backend_evolved` - Capability-based storage backend (RECOMMENDED)
//! - `agents` - AI agent deployment manager (uses `agent_backend`)
//! - `agent_backend` - Trait-based agent backends (legacy: Squirrel, in-memory)
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
pub mod agent_backend;
pub mod agent_backend_evolved;
pub mod agents;
pub mod auth;
pub mod auth_backend;
#[cfg(test)]
pub mod auth_backend_evolved;
pub mod storage;
pub mod storage_backend;
#[cfg(test)]
pub mod storage_backend_evolved;
pub mod types;

// Re-export legacy backends for backward compatibility
pub use agent_backend::{AgentBackend, InMemoryAgentBackend, SquirrelBackend};
pub use agents::*;
pub use auth::*;
pub use auth_backend::{AuthBackend, BearDogBackend, InMemoryAuthBackend};
pub use storage::*;
pub use storage_backend::{InMemoryBackend, NestGateBackend, StorageBackend, VolumeStatus};
pub use types::*;

// Re-export evolved backends (RECOMMENDED for new code)
pub use agent_backend_evolved::AgentBackend as AgentBackendEvolved;
#[cfg(test)]
pub use auth_backend_evolved::AuthBackend as AuthBackendEvolved;
#[cfg(test)]
pub use storage_backend_evolved::StorageBackend as StorageBackendEvolved;
