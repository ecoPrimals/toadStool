// SPDX-License-Identifier: AGPL-3.0-or-later
//! Interned Strings - Zero-Allocation Constants
//!
//! This module provides static string constants for common values throughout the codebase.
//! Using these interned strings eliminates unnecessary string allocations.
//!
//! ## WateringHole Sovereignty: Discover by Capability, Address by Name
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

    /// Squirrel AI service identifier
    /// **DEPRECATED**: Use `capabilities::INTELLIGENCE` instead
    pub const SQUIRREL: &str = "squirrel";

    /// ToadStool compute service identifier
    pub const TOADSTOOL: &str = "toadstool";
}

/// Common status strings
pub mod status {
    pub const RUNNING: &str = "running";
    pub const STOPPED: &str = "stopped";
    pub const STARTING: &str = "starting";
    pub const STOPPING: &str = "stopping";
    pub const FAILED: &str = "failed";
    pub const HEALTHY: &str = "healthy";
    pub const DEGRADED: &str = "degraded";
    pub const UNKNOWN: &str = "unknown";
}

/// Common environment strings
pub mod env {
    pub const DEVELOPMENT: &str = "development";
    pub const STAGING: &str = "staging";
    pub const PRODUCTION: &str = "production";
    pub const TEST: &str = "test";
}

/// Common content types
pub mod content_types {
    pub const JSON: &str = "application/json";
    pub const YAML: &str = "application/yaml";
    pub const TOML: &str = "application/toml";
    pub const TEXT: &str = "text/plain";
    pub const HTML: &str = "text/html";
    pub const XML: &str = "application/xml";
    pub const BINARY: &str = "application/octet-stream";
}

/// Common discovery sources
pub mod discovery_sources {
    pub const MDNS: &str = "mdns";
    pub const ENVIRONMENT: &str = "environment";
    pub const CONFIG_FILE: &str = "config-file";
    pub const SERVICE_MESH: &str = "service-mesh";
    pub const FALLBACK: &str = "fallback";
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
        assert_eq!(capabilities::ENCRYPTION, "encryption");
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
}
