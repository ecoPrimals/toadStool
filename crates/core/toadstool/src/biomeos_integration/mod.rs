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
//! - **Cross-Primal authentication**: `BearDog` token propagation
//! - **Automated provisioning**: Storage and AI agent deployment
//! - **Sub-60-second bootstrap**: Optimized startup performance
//!
//! ## Module Structure
//!
//! - `types` - Core configuration and state types
//! - `auth` - Cross-Primal authentication manager (uses auth_backend)
//! - `auth_backend` - Trait-based auth backends (BearDog, in-memory)
//! - `storage` - Storage provisioning manager (uses storage_backend)
//! - `storage_backend` - Trait-based storage backends (NestGate, in-memory)
//! - `agents` - AI agent deployment manager (uses agent_backend)
//! - `agent_backend` - Trait-based agent backends (Squirrel, in-memory)

// Module declarations
pub mod agent_backend;
pub mod agents;
pub mod auth;
pub mod auth_backend;
pub mod storage;
pub mod storage_backend;
pub mod types;

// Re-export everything for backward compatibility
pub use agent_backend::{AgentBackend, InMemoryAgentBackend, SquirrelBackend};
pub use agents::*;
pub use auth::*;
pub use auth_backend::{AuthBackend, BearDogBackend, InMemoryAuthBackend};
pub use storage::*;
pub use storage_backend::{InMemoryBackend, NestGateBackend, StorageBackend, VolumeStatus};
pub use types::*;
