// SPDX-License-Identifier: AGPL-3.0-only
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
//!   These are canonical names for routing messages, NOT for capability matching.
//!   Never use `if name == primals::BEARDOG` to select a service; use capability checks.
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
//! // Legacy primal names (deprecated, use capabilities instead!)
//! #[allow(deprecated)]
//! let beardog = primals::BEARDOG;
//! ```

/// Capability type constants (Deep Debt compliant)
///
/// These represent WHAT services do, not WHO provides them.
/// Use these for capability-based discovery! Never match on primal names.
pub mod capabilities {
    /// Security capabilities (encryption, signing, key management)
    pub const SECURITY: &str = "security";

    /// Cryptographic capabilities (encryption, signing, key management, PKI).
    /// Use for discovery: `discover_capability(capabilities::CRYPTO)`.
    pub const CRYPTO: &str = "crypto";

    /// Storage capabilities (persistence, compression, versioning)
    pub const STORAGE: &str = "storage";

    /// Coordination capabilities (service mesh, discovery, orchestration)
    pub const COORDINATION: &str = "coordination";

    /// Workload routing / MCP-style agent IPC (replaces legacy “squirrel” name in discovery)
    pub const ROUTING: &str = "routing";

    /// AI/ML capabilities (inference, training, natural language)
    pub const INTELLIGENCE: &str = "intelligence";

    /// Compute capabilities (CPU, GPU, specialized hardware)
    pub const COMPUTE: &str = "compute";

    /// Monitoring capabilities (metrics, logging, tracing)
    pub const MONITORING: &str = "monitoring";

    /// Networking capabilities (routing, tunneling, VPN)
    pub const NETWORKING: &str = "networking";

    // Specific capability features

    /// Encryption capability
    pub const ENCRYPTION: &str = "encryption";

    /// Digital signing capability
    pub const SIGNING: &str = "signing";

    /// Key management capability
    pub const KEY_MANAGEMENT: &str = "key-management";

    /// Public Key Infrastructure
    pub const PKI: &str = "pki";

    /// Audit logging capability
    pub const AUDIT: &str = "audit";

    /// Data persistence capability
    pub const PERSISTENCE: &str = "persistence";

    /// Data compression capability
    pub const COMPRESSION: &str = "compression";

    /// Version control capability
    pub const VERSIONING: &str = "versioning";

    /// GPU dispatch capability (coralReef/barraCuda compute triangle)
    pub const GPU_DISPATCH: &str = "gpu.dispatch";

    /// Science GPU dispatch (JSON-RPC method family)
    pub const SCIENCE_GPU_DISPATCH: &str = "science.gpu.dispatch";

    /// Shader compilation capability (sovereign pipeline)
    pub const SHADER_COMPILE: &str = "shader.compile";

    /// Native shader compilation pipeline
    pub const SHADER_COMPILE_NATIVE: &str = "shader.compile.native";

    /// GPU hardware calibration (NVVM safety, precision tier probing).
    pub const GPU_CALIBRATION: &str = "gpu.calibration";

    /// Workload routing (substrate selection based on problem size).
    pub const WORKLOAD_ROUTING: &str = "workload.routing";

    /// Orchestration capability
    pub const ORCHESTRATION: &str = "orchestration";

    /// Ecology domain capability (airSpring)
    pub const ECOLOGY: &str = "ecology";

    /// Science domain capability
    pub const SCIENCE: &str = "science";

    /// Activation function capabilities (barraCuda)
    pub const ACTIVATIONS: &str = "science.activations";

    /// RNG capabilities
    pub const RNG: &str = "science.rng";

    /// Special math functions
    pub const SPECIAL_FUNCTIONS: &str = "science.special";

    /// Biology domain capability (wetSpring — metagenomics, phylogenetics, mass spec)
    pub const BIOLOGY: &str = "biology";

    /// Health domain capability (healthSpring — PK/PD, NLME, biosignal)
    pub const HEALTH: &str = "health";

    /// Measurement/uncertainty domain capability (groundSpring — UQ, validation)
    pub const MEASUREMENT: &str = "measurement";

    /// Optimization domain capability (neuralSpring — ML, evolutionary computation)
    pub const OPTIMIZATION: &str = "optimization";

    /// Visualization / streaming pipeline capability (petalTongue)
    pub const VISUALIZATION: &str = "visualization";
}

/// Protocol constants
pub mod protocols {
    /// HTTP protocol
    pub const HTTP: &str = "http";

    /// HTTPS protocol
    pub const HTTPS: &str = "https";

    /// gRPC protocol
    pub const GRPC: &str = "grpc";

    /// `WebSocket` protocol
    #[deprecated(
        since = "0.5.0",
        note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
    )]
    pub const WEBSOCKET: &str = "websocket";

    /// Secure `WebSocket` protocol
    #[deprecated(
        since = "0.5.0",
        note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
    )]
    pub const WSS: &str = "wss";

    /// JSON-RPC protocol
    pub const JSONRPC: &str = "jsonrpc";

    /// TCP protocol
    pub const TCP: &str = "tcp";

    /// UDP protocol
    pub const UDP: &str = "udp";

    /// Unix domain socket
    pub const UNIX: &str = "unix";

    /// tarpc protocol
    pub const TARPC: &str = "tarpc";
}

/// ⚠️ DEPRECATED: Legacy primal name constants
///
/// **For IPC addressing only** (socket paths, endpoint IDs, message routing).
/// These are canonical names for addressing — NOT for capability matching.
/// Use `capabilities::*` for discovery; use these only when you already have
/// a discovered service and need its name for socket paths or routing.
///
/// # Migration Guide
///
/// ```ignore
/// // ❌ OLD (hardcoded WHO):
/// use toadstool_common::interned_strings::primals;
/// let service = discover_service(primals::BEARDOG).await?;
///
/// // ✅ NEW (capability-based WHAT):
/// use toadstool_common::interned_strings::capabilities;
/// let service = discover_capability(capabilities::SECURITY).await?;
/// ```
#[deprecated(
    since = "0.4.0",
    note = "Use capability-based discovery (capabilities::*) instead of hardcoded primal names"
)]
pub mod primals {
    /// Beardog security service identifier
    /// **DEPRECATED**: Use `capabilities::SECURITY` instead
    pub const BEARDOG: &str = "beardog";

    /// Songbird coordination service identifier
    /// **DEPRECATED**: Use `capabilities::COORDINATION` instead
    pub const SONGBIRD: &str = "songbird";

    /// Nestgate storage service identifier
    /// **DEPRECATED**: Use `capabilities::STORAGE` instead
    pub const NESTGATE: &str = "nestgate";

    /// Squirrel AI service identifier (legacy addressing only)
    /// **DEPRECATED**: Use `capabilities::ROUTING` or `capabilities::INTELLIGENCE` for discovery
    pub const SQUIRREL: &str = "squirrel";

    /// ToadStool compute service identifier
    pub const TOADSTOOL: &str = "toadstool";
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

    /// Resolve a legacy primal name or capability string to a domain.
    ///
    /// Returns `None` for unrecognised strings.
    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "crypto" | "security" | "beardog" | "bear-dog" | "pki" => Some(Self::Security),
            "coordination" | "orchestration" | "songbird" | "song-bird" => Some(Self::Coordination),
            "storage" | "nestgate" | "nest-gate" => Some(Self::Storage),
            "compute" | "toadstool" | "toad-stool" => Some(Self::Compute),
            "routing" | "squirrel" => Some(Self::Routing),
            "intelligence" | "ai" => Some(Self::Intelligence),
            "monitoring" | "metrics" => Some(Self::Monitoring),
            _ => None,
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
    #[allow(deprecated)]
    fn test_protocols() {
        assert_eq!(protocols::HTTP, "http");
        assert_eq!(protocols::GRPC, "grpc");
        assert_eq!(protocols::WEBSOCKET, "websocket");
        assert_eq!(protocols::JSONRPC, "jsonrpc");
        assert_eq!(protocols::UNIX, "unix");
        assert_eq!(protocols::TARPC, "tarpc");
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_primals() {
        assert_eq!(primals::BEARDOG, "beardog");
        assert_eq!(primals::SONGBIRD, "songbird");
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
    fn test_capability_domain_from_label_legacy_names() {
        assert_eq!(
            CapabilityDomain::from_label("beardog"),
            Some(CapabilityDomain::Security)
        );
        assert_eq!(
            CapabilityDomain::from_label("songbird"),
            Some(CapabilityDomain::Coordination)
        );
        assert_eq!(
            CapabilityDomain::from_label("nestgate"),
            Some(CapabilityDomain::Storage)
        );
        assert_eq!(
            CapabilityDomain::from_label("squirrel"),
            Some(CapabilityDomain::Routing)
        );
        assert_eq!(
            CapabilityDomain::from_label("toadstool"),
            Some(CapabilityDomain::Compute)
        );
        assert_eq!(CapabilityDomain::from_label("unknown-thing"), None);
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
