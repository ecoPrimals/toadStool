// SPDX-License-Identifier: AGPL-3.0-or-later
//! Interned Strings - Zero-Allocation Constants
//!
//! This module provides static string constants for common values throughout the codebase.
//! Using these interned strings eliminates unnecessary string allocations.
//!
//! ## `WateringHole` Sovereignty: Discover by Capability, Address by Name
//!
//! - **`capabilities::*`** — Use for DISCOVERY. Scan for what a service CAN DO.
//!   Example: `if capabilities.contains(capabilities::CRYPTO)` or
//!   `discover_capability(capabilities::STORAGE)`.
//!
//! - **`primals::*`** — Use for IPC ADDRESSING only (socket paths, endpoint IDs).
//!   These are stable routing labels for legacy paths, NOT for capability matching.
//!   Never branch on a hardcoded legacy route label to select a service; use capability checks.
//!
//! # Performance Impact
//!
//! **Before** (allocation per use):
//! ```rust
//! let cap = "encryption".to_string();  // Heap allocation
//! ```
//!
//! **After** (zero allocation):
//! ```rust
//! use toadstool_common::interned_strings::capabilities;
//! let cap = capabilities::ENCRYPTION;  // Static reference, no allocation
//! ```
//!
//! # Usage Example
//!
//! ```rust
//! use toadstool_common::interned_strings::{capabilities, protocols, primals};
//!
//! // Capability-based discovery (Deep Debt compliant)
//! let security_cap = capabilities::SECURITY;
//! let storage_cap = capabilities::STORAGE;
//!
//! // Protocol constants
//! let http = protocols::HTTP;
//! let grpc = protocols::GRPC;
//!
//! // Legacy routing labels (deprecated for discovery; use capabilities instead)
//! let _legacy_crypto_route = primals::LEGACY_SECURITY_LABEL;
//! ```

pub mod biomeos_manifest_serde;
pub mod capabilities;
pub mod primals;
pub mod protocols;
pub mod socket_env;

#[inline]
fn label_eq(left: &str, right: &str) -> bool {
    left == right
}

/// Typed capability domain — use instead of scattered string literals.
///
/// Each variant carries the canonical `&str` constant so callers can go from
/// enum to string (`domain.as_str()`) and back (`CapabilityDomain::from_str`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityDomain {
    /// Security / cryptography / PKI.
    Security,
    /// Coordination / discovery / mesh.
    Coordination,
    /// Storage / artifacts / pipelines.
    Storage,
    /// Compute / CPU / GPU / specialized hardware.
    Compute,
    /// AI routing / MCP-style agent IPC.
    Routing,
    /// AI/ML inference / training.
    Intelligence,
    /// Monitoring / metrics / tracing.
    Monitoring,
}

impl CapabilityDomain {
    /// Canonical capability string for this domain.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Security => capabilities::CRYPTO,
            Self::Coordination => capabilities::COORDINATION,
            Self::Storage => capabilities::STORAGE,
            Self::Compute => capabilities::COMPUTE,
            Self::Routing => capabilities::ROUTING,
            Self::Intelligence => capabilities::INTELLIGENCE,
            Self::Monitoring => capabilities::MONITORING,
        }
    }

    /// Preferred explicit socket env var for this domain (capability-based naming).
    #[must_use]
    pub const fn biomeos_socket_env(self) -> &'static str {
        match self {
            Self::Security => socket_env::BIOMEOS_CRYPTO_SOCKET,
            Self::Coordination => socket_env::BIOMEOS_COORDINATION_SOCKET,
            Self::Storage => socket_env::BIOMEOS_STORAGE_SOCKET,
            Self::Routing => socket_env::BIOMEOS_ROUTING_SOCKET,
            Self::Compute => socket_env::TOADSTOOL_SOCKET,
            Self::Intelligence => socket_env::TOADSTOOL_INTELLIGENCE_SOCKET,
            Self::Monitoring => socket_env::TOADSTOOL_TELEMETRY,
        }
    }

    /// Preferred toadStool-prefixed socket env var for this domain.
    #[must_use]
    pub const fn toadstool_socket_env(self) -> &'static str {
        match self {
            Self::Security => socket_env::TOADSTOOL_SECURITY_SOCKET,
            Self::Coordination => socket_env::TOADSTOOL_COORDINATION_SOCKET,
            Self::Storage => socket_env::TOADSTOOL_STORAGE_SOCKET,
            Self::Routing => socket_env::TOADSTOOL_INTELLIGENCE_SOCKET,
            Self::Compute => socket_env::TOADSTOOL_SOCKET,
            Self::Intelligence => socket_env::TOADSTOOL_INTELLIGENCE_SOCKET,
            Self::Monitoring => socket_env::TOADSTOOL_TELEMETRY,
        }
    }

    /// Resolve a capability id or legacy route label to a domain.
    ///
    /// Capability strings (`capabilities::*`) are preferred; legacy primal route
    /// labels (`primals::LEGACY_*_LABEL`) are accepted for older manifests.
    ///
    /// Returns `None` for unrecognised strings.
    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        let lower = s.to_ascii_lowercase();
        let l = lower.as_str();
        if label_eq(l, capabilities::CRYPTO)
            || label_eq(l, capabilities::SECURITY)
            || label_eq(l, capabilities::PKI)
            || label_eq(l, primals::LEGACY_SECURITY_LABEL)
            || label_eq(l, primals::LEGACY_SECURITY_KEBAB)
        {
            Some(Self::Security)
        } else if label_eq(l, capabilities::COORDINATION)
            || label_eq(l, capabilities::ORCHESTRATION)
            || label_eq(l, primals::LEGACY_COORDINATION_LABEL)
            || label_eq(l, primals::LEGACY_COORDINATION_KEBAB)
        {
            Some(Self::Coordination)
        } else if label_eq(l, capabilities::STORAGE)
            || label_eq(l, primals::LEGACY_STORAGE_LABEL)
            || label_eq(l, primals::LEGACY_STORAGE_KEBAB)
        {
            Some(Self::Storage)
        } else if label_eq(l, capabilities::COMPUTE) || label_eq(l, primals::TOADSTOOL) || l == "toad-stool"
        {
            Some(Self::Compute)
        } else if label_eq(l, capabilities::ROUTING) || label_eq(l, primals::LEGACY_INTELLIGENCE_LABEL) {
            Some(Self::Routing)
        } else if label_eq(l, capabilities::INTELLIGENCE) || l == "ai" {
            Some(Self::Intelligence)
        } else if label_eq(l, capabilities::MONITORING) || l == "metrics" {
            Some(Self::Monitoring)
        } else {
            None
        }
    }

    /// All known domains.
    pub const ALL: [CapabilityDomain; 7] = [
        Self::Security,
        Self::Coordination,
        Self::Storage,
        Self::Compute,
        Self::Routing,
        Self::Intelligence,
        Self::Monitoring,
    ];
}

impl std::fmt::Display for CapabilityDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for CapabilityDomain {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_label(s).ok_or(())
    }
}

/// Common status strings
pub mod status {
    /// Service/process is running; used in health checks and status reports.
    pub const RUNNING: &str = "running";
    /// Service/process is stopped; used in status reports.
    pub const STOPPED: &str = "stopped";
    /// Service/process is in startup; used during bootstrap.
    pub const STARTING: &str = "starting";
    /// Service/process is shutting down; used during teardown.
    pub const STOPPING: &str = "stopping";
    /// Service/process has failed; used in error reporting.
    pub const FAILED: &str = "failed";
    /// Service is healthy; used in health checks.
    pub const HEALTHY: &str = "healthy";
    /// Service is degraded; used in health checks.
    pub const DEGRADED: &str = "degraded";
    /// Status is unknown; used when state cannot be determined.
    pub const UNKNOWN: &str = "unknown";
}

/// Common environment strings
pub mod env {
    /// Development environment; used for config selection and logging.
    pub const DEVELOPMENT: &str = "development";
    /// Staging environment; used for config selection and logging.
    pub const STAGING: &str = "staging";
    /// Production environment; used for config selection and logging.
    pub const PRODUCTION: &str = "production";
    /// Test environment; used for config selection in tests.
    pub const TEST: &str = "test";
}

/// Common content types
pub mod content_types {
    /// JSON MIME type; used for Content-Type headers and serialization.
    pub const JSON: &str = "application/json";
    /// YAML MIME type; used for Content-Type headers and config parsing.
    pub const YAML: &str = "application/yaml";
    /// TOML MIME type; used for Content-Type headers and config parsing.
    pub const TOML: &str = "application/toml";
    /// Plain text MIME type; used for Content-Type headers.
    pub const TEXT: &str = "text/plain";
    /// HTML MIME type; used for Content-Type headers.
    pub const HTML: &str = "text/html";
    /// XML MIME type; used for Content-Type headers.
    pub const XML: &str = "application/xml";
    /// Binary octet-stream MIME type; used for raw binary payloads.
    pub const BINARY: &str = "application/octet-stream";
}

/// Runtime type names (for `RuntimeType::Custom` and display)
///
/// Use these when converting `RuntimeType` to string for logging, metrics, or IPC.
pub mod runtime_types {
    /// Native process execution
    pub const NATIVE: &str = "native";
    /// WebAssembly execution
    pub const WASM: &str = "wasm";
    /// Container execution
    pub const CONTAINER: &str = "container";
    /// GPU acceleration
    pub const GPU: &str = "gpu";
    /// Python runtime
    pub const PYTHON: &str = "python";
    /// BiomeOS integration runtime
    pub const BIOMEOS: &str = "biomeos";
}

/// Common discovery sources
pub mod discovery_sources {
    /// mDNS discovery; used when service was found via multicast DNS.
    pub const MDNS: &str = "mdns";
    /// Environment variable discovery; used when service came from env vars.
    pub const ENVIRONMENT: &str = "environment";
    /// Config file discovery; used when service came from config.
    pub const CONFIG_FILE: &str = "config-file";
    /// Service mesh discovery; used when service came from mesh.
    pub const SERVICE_MESH: &str = "service-mesh";
    /// Fallback discovery; used when no other source matched.
    pub const FALLBACK: &str = "fallback";
    /// Universal adapter discovery; used when service came from adapter.
    pub const UNIVERSAL_ADAPTER: &str = "universal-adapter";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities() {
        assert_eq!(capabilities::SECURITY, "security");
        assert_eq!(capabilities::CRYPTO, "crypto");
        assert_eq!(capabilities::STORAGE, "storage");
        assert_eq!(capabilities::COORDINATION, "coordination");
        assert_eq!(capabilities::ROUTING, "routing");
        assert_eq!(capabilities::ENCRYPTION, "encryption");
        assert_eq!(capabilities::GPU_DISPATCH, "gpu.dispatch");
        assert_eq!(capabilities::SCIENCE_GPU_DISPATCH, "science.gpu.dispatch");
        assert_eq!(capabilities::SHADER_COMPILE, "shader.compile");
        assert_eq!(capabilities::ORCHESTRATION, "orchestration");
        assert_eq!(capabilities::BIOLOGY, "biology");
        assert_eq!(capabilities::HEALTH, "health");
        assert_eq!(capabilities::MEASUREMENT, "measurement");
        assert_eq!(capabilities::OPTIMIZATION, "optimization");
        assert_eq!(capabilities::VISUALIZATION, "visualization");
    }

    #[test]
    fn test_protocols() {
        assert_eq!(protocols::HTTP, "http");
        assert_eq!(protocols::GRPC, "grpc");
        assert_eq!(protocols::JSONRPC, "jsonrpc");
        assert_eq!(protocols::UNIX, "unix");
        assert_eq!(protocols::TARPC, "tarpc");
    }

    #[test]
    fn test_legacy_route_labels() {
        assert_eq!(primals::LEGACY_SECURITY_LABEL, "beardog");
        assert_eq!(primals::LEGACY_COORDINATION_LABEL, "songbird");
        assert_eq!(primals::LEGACY_STORAGE_LABEL, "nestgate");
        assert_eq!(primals::LEGACY_INTELLIGENCE_LABEL, "squirrel");
    }

    #[test]
    #[allow(deprecated, reason = "testing legacy BEARDOG_* env constant values")]
    fn test_socket_env_names_match_runtime() {
        assert_eq!(socket_env::LEGACY_BEARDOG_SOCKET_ENV, "BEARDOG_SOCKET");
        assert_eq!(socket_env::BIOMEOS_CRYPTO_SOCKET, "BIOMEOS_CRYPTO_SOCKET");
        assert_eq!(
            socket_env::TOADSTOOL_SECURITY_SOCKET,
            "TOADSTOOL_SECURITY_SOCKET"
        );
    }

    #[test]
    fn test_biomeos_manifest_serde_tags() {
        assert_eq!(biomeos_manifest_serde::COORDINATION, "Coordination");
        assert_eq!(biomeos_manifest_serde::LEGACY_SONGBIRD_PASCAL, "Songbird");
        assert_eq!(
            biomeos_manifest_serde::LEGACY_BEARDOG_LOWER,
            primals::LEGACY_SECURITY_LABEL
        );
    }

    #[test]
    fn test_capability_domain_as_str() {
        assert_eq!(CapabilityDomain::Security.as_str(), "crypto");
        assert_eq!(CapabilityDomain::Coordination.as_str(), "coordination");
        assert_eq!(CapabilityDomain::Storage.as_str(), "storage");
        assert_eq!(CapabilityDomain::Compute.as_str(), "compute");
        assert_eq!(CapabilityDomain::Routing.as_str(), "routing");
        assert_eq!(CapabilityDomain::Intelligence.as_str(), "intelligence");
        assert_eq!(CapabilityDomain::Monitoring.as_str(), "monitoring");
    }

    #[test]
    fn test_capability_domain_from_label_capability_ids() {
        assert_eq!(
            CapabilityDomain::from_label(capabilities::CRYPTO),
            Some(CapabilityDomain::Security)
        );
        assert_eq!(
            CapabilityDomain::from_label(capabilities::COORDINATION),
            Some(CapabilityDomain::Coordination)
        );
        assert_eq!(
            CapabilityDomain::from_label(capabilities::STORAGE),
            Some(CapabilityDomain::Storage)
        );
        assert_eq!(
            CapabilityDomain::from_label(capabilities::ROUTING),
            Some(CapabilityDomain::Routing)
        );
        assert_eq!(
            CapabilityDomain::from_label(capabilities::INTELLIGENCE),
            Some(CapabilityDomain::Intelligence)
        );
        assert_eq!(
            CapabilityDomain::from_label(capabilities::MONITORING),
            Some(CapabilityDomain::Monitoring)
        );
    }

    #[test]
    fn test_capability_domain_from_label_legacy_route_labels() {
        assert_eq!(
            CapabilityDomain::from_label(primals::LEGACY_SECURITY_LABEL),
            Some(CapabilityDomain::Security)
        );
        assert_eq!(
            CapabilityDomain::from_label(primals::LEGACY_COORDINATION_LABEL),
            Some(CapabilityDomain::Coordination)
        );
        assert_eq!(
            CapabilityDomain::from_label(primals::LEGACY_STORAGE_LABEL),
            Some(CapabilityDomain::Storage)
        );
        assert_eq!(
            CapabilityDomain::from_label(primals::LEGACY_INTELLIGENCE_LABEL),
            Some(CapabilityDomain::Routing)
        );
        assert_eq!(
            CapabilityDomain::from_label(primals::TOADSTOOL),
            Some(CapabilityDomain::Compute)
        );
        assert_eq!(CapabilityDomain::from_label("unknown-thing"), None);
    }

    #[test]
    fn test_capability_domain_socket_env_names() {
        assert_eq!(
            CapabilityDomain::Security.biomeos_socket_env(),
            socket_env::BIOMEOS_CRYPTO_SOCKET
        );
        assert_eq!(
            CapabilityDomain::Coordination.toadstool_socket_env(),
            socket_env::TOADSTOOL_COORDINATION_SOCKET
        );
    }

    #[test]
    fn test_capability_domain_from_str() {
        assert_eq!(
            capabilities::STORAGE.parse::<CapabilityDomain>(),
            Ok(CapabilityDomain::Storage)
        );
        assert!("legacy-unknown".parse::<CapabilityDomain>().is_err());
    }

    #[test]
    fn test_capability_domain_display() {
        assert_eq!(format!("{}", CapabilityDomain::Security), "crypto");
        assert_eq!(
            format!("{}", CapabilityDomain::Coordination),
            "coordination"
        );
    }

    #[test]
    fn test_capability_domain_all() {
        assert_eq!(CapabilityDomain::ALL.len(), 7);
    }

    #[test]
    fn test_status() {
        assert_eq!(status::RUNNING, "running");
        assert_eq!(status::HEALTHY, "healthy");
    }

    #[test]
    fn test_env() {
        assert_eq!(env::PRODUCTION, "production");
        assert_eq!(env::TEST, "test");
    }

    #[test]
    fn test_content_types() {
        assert_eq!(content_types::JSON, "application/json");
        assert_eq!(content_types::YAML, "application/yaml");
    }

    #[test]
    fn test_runtime_types() {
        assert_eq!(runtime_types::NATIVE, "native");
        assert_eq!(runtime_types::WASM, "wasm");
        assert_eq!(runtime_types::CONTAINER, "container");
        assert_eq!(runtime_types::GPU, "gpu");
        assert_eq!(runtime_types::PYTHON, "python");
        assert_eq!(runtime_types::BIOMEOS, "biomeos");
    }
}
