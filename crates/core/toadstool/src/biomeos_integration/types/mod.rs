// SPDX-License-Identifier: AGPL-3.0-only
//! # BiomeOS Integration Types
//!
//! This module contains configuration structures for BiomeOS integration,
//! organized into logical domains for better maintainability and navigation.
//!
//! ## Module Organization
//!
//! The types are split into the following modules by functional concern:
//!
//! - [`manifest`] - Core biome manifest and metadata structures
//! - [`config`] - Primal service configuration (ToadStool, Songbird, BearDog, etc.)
//! - [`auth`] - Authentication, authorization, and security policy types
//! - [`storage`] - Storage configuration, volumes, and provisioning
//! - [`agent`] - AI agent deployment and model configuration
//! - [`networking`] - Network configuration, DNS, and service mesh
//! - [`resources`] - Resource allocation, GPU, and health checks
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use toadstool::biomeos_integration::types::{BiomeManifest, PrimalsConfig};
//!
//! // Create a default manifest
//! let manifest = BiomeManifest::default();
//!
//! // Access primal configurations
//! if let Some(toadstool) = &manifest.primals.toadstool {
//!     println!("ToadStool enabled: {}", toadstool.enabled);
//! }
//! ```
//!
//! ## Design Philosophy
//!
//! This refactoring treats large files as complexity indicators and splits them
//! by logical concerns, following modern Rust best practices:
//!
//! - Each module is <250 lines for easier maintenance
//! - Clear separation of concerns by domain
//! - Comprehensive documentation at module level
//! - Logical re-exports for common usage patterns
//!
//! ## Integration Patterns
//!
//! See individual module documentation for detailed examples and patterns.

/// Agent deployment and model configuration types.
pub mod agent;
/// Authentication, authorization, and security policy types.
pub mod auth;
/// Primal service configuration (ToadStool, Songbird, BearDog, etc.).
pub mod config;
/// Core biome manifest and metadata structures.
pub mod manifest;
/// Network configuration, DNS, and service mesh types.
pub mod networking;
/// Resource allocation, GPU, and health check types.
pub mod resources;
/// Storage configuration, volumes, and provisioning types.
pub mod storage;

// Re-export commonly used types for convenient access
// Explicit exports to avoid ambiguity

// From agent module
pub use agent::{AgentConfig, BootConfig, MCPConfig, ModelConfig};

// From auth module
pub use auth::{
    AuthenticationConfig, AuthorizationConfig, BiomeSecurity, NetworkPolicy, NetworkRule,
    OAuthConfig, PBACConfig, PolicyRule, RBACConfig, Role, RoleBinding, SecurityPolicy,
    TokenConfig, TokenRefreshConfig,
};

// From config module
pub use config::{
    BearDogConfig, BiomeOSConfig, NestGateConfig, PrimalsConfig, ServiceVolumeConfig,
    SongbirdConfig, SquirrelConfig, ToadStoolConfig,
};

// From manifest module
pub use manifest::{BiomeManifest, BiomeMetadata, ServiceSource};

// From networking module
pub use networking::{BiomeNetworking, DNSConfig, PortMapping, ServiceConfig, ServiceMeshConfig};

// From resources module
pub use resources::{
    BackupConfig, BiomeHealthCheckConfig, BiomeResources, BiomeStorageConfig, GpuAllocation,
    PrimalResources, ReplicationConfig, TokenPropagationConfig, TokenValidationConfig,
    VolumeConfig,
};

// From storage module
pub use storage::{
    BiomeStorage, MountStatus, PersistentVolume, ReplicationSettings, StorageClass,
    StorageProvisioningRequest, VolumeCleanupStatus, VolumeInfo, VolumeMountInfo, VolumeMountSpec,
    VolumeMountStatus, VolumeProvisioningStatus,
};
