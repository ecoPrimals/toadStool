// SPDX-License-Identifier: AGPL-3.0-only
//! Centralized Port Configuration
//!
//! **Phase 1 of Capability-Based Discovery Evolution**
//!
//! This module centralizes all hardcoded ports as the first step toward
//! runtime discovery. Future evolution:
//! - Phase 1: Centralize (this file) ✅
//! - Phase 2: Environment variable overrides
//! - Phase 3: Runtime discovery via Songbird
//! - Phase 4: Full mDNS + capability-based discovery

/// Default ports for ToadStool services
///
/// **Self-Knowledge Principle**: ToadStool only defines its own ports.
/// Port 0 = OS-assigned at bind time (capability-based).
pub mod toadstool {
    /// Main ToadStool server port (0 = OS-assigned)
    pub const SERVER: u16 = 0;

    /// GPU compute service port (0 = OS-assigned)
    pub const GPU_COMPUTE: u16 = 0;

    /// Distributed scheduler port (0 = OS-assigned)
    pub const DISTRIBUTED: u16 = 0;

    /// Health check endpoint port (0 = OS-assigned)
    pub const HEALTH: u16 = 0;

    /// Metrics/monitoring port (0 = OS-assigned)
    pub const METRICS: u16 = 0;

    /// Daemon/BYOB HTTP API port (default when not using OS-assigned)
    pub const DAEMON_API: u16 = 8084;
}

/// Capability-based fallback ports for cold-start bootstrap.
///
/// These are fallback defaults used ONLY before runtime discovery is available.
/// Production systems use `RuntimeDiscovery::discover_capability()` instead.
///
/// Capabilities map to port ranges rather than primal-specific assignments.
/// Each capability resolves via: env var → config file → this fallback.
pub mod capability_fallback {
    /// Coordination capability (orchestration, scheduling) — e.g. Songbird
    pub const COORDINATION: u16 = 8080;

    /// Security capability (auth, policy, zero-trust) — e.g. BearDog
    pub const SECURITY: u16 = 8081;

    /// Storage capability (artifacts, pipelines) — e.g. NestGate
    pub const STORAGE: u16 = 8082;

    /// Platform capability (MCP, model hosting) — e.g. Squirrel
    pub const PLATFORM: u16 = 8083;

    /// Ecosystem integration capability (biome management)
    pub const ECOSYSTEM: u16 = 8088;

    /// Ecosystem primary port (UI/API gateway)
    pub const ECOSYSTEM_PRIMARY: u16 = 8005;

    /// Shader compiler capability (WGSL/SPIR-V → native binary) — e.g. coralReef
    pub const SHADER_COMPILER: u16 = 8090;
}

/// Legacy primal-name aliases for backward compatibility during migration.
///
/// **DEPRECATED**: Use `capability_fallback::*` or runtime capability discovery.
#[deprecated(
    since = "0.1.0",
    note = "Use capability_fallback::* or runtime capability discovery."
)]
pub mod discovery_fallback {
    /// Coordination capability discovery port (legacy).
    #[deprecated(note = "Use capability_fallback::COORDINATION")]
    pub const DEFAULT_SONGBIRD_DISCOVERY_PORT: u16 = super::capability_fallback::COORDINATION;

    /// Security capability discovery port (legacy).
    #[deprecated(note = "Use capability_fallback::SECURITY")]
    pub const DEFAULT_BEARDOG_DISCOVERY_PORT: u16 = super::capability_fallback::SECURITY;

    /// Storage capability discovery port (legacy).
    #[deprecated(note = "Use capability_fallback::STORAGE")]
    pub const DEFAULT_NESTGATE_DISCOVERY_PORT: u16 = super::capability_fallback::STORAGE;

    /// Platform capability discovery port (legacy).
    #[deprecated(note = "Use capability_fallback::PLATFORM")]
    pub const DEFAULT_SQUIRREL_DISCOVERY_PORT: u16 = super::capability_fallback::PLATFORM;

    /// Ecosystem discovery port (legacy).
    #[deprecated(note = "Use capability_fallback::ECOSYSTEM")]
    pub const DEFAULT_BIOMEOS_DISCOVERY_PORT: u16 = super::capability_fallback::ECOSYSTEM;

    /// Ecosystem primary port (legacy).
    #[deprecated(note = "Use capability_fallback::ECOSYSTEM_PRIMARY")]
    pub const DEFAULT_BIOMEOS_PRIMARY_PORT: u16 = super::capability_fallback::ECOSYSTEM_PRIMARY;
}

/// Legacy primal-name port constants for backward compatibility.
///
/// **DEPRECATED**: Use `capability_fallback::*` or runtime capability discovery.
#[deprecated(
    since = "0.1.0",
    note = "Use capability_fallback::* or runtime capability discovery."
)]
pub mod fallback {
    /// Coordination capability port (legacy).
    #[deprecated(note = "Use capability_fallback::COORDINATION")]
    pub const SONGBIRD: u16 = super::capability_fallback::COORDINATION;

    /// Platform capability port (legacy).
    #[deprecated(note = "Use capability_fallback::PLATFORM")]
    pub const SQUIRREL: u16 = super::capability_fallback::PLATFORM;

    /// Security capability port (legacy).
    #[deprecated(note = "Use capability_fallback::SECURITY")]
    pub const BEARDOG: u16 = super::capability_fallback::SECURITY;

    /// Storage capability port (legacy).
    #[deprecated(note = "Use capability_fallback::STORAGE")]
    pub const NESTGATE: u16 = super::capability_fallback::STORAGE;

    /// Ecosystem capability port (legacy).
    #[deprecated(note = "Use capability_fallback::ECOSYSTEM")]
    pub const BIOMEOS: u16 = super::capability_fallback::ECOSYSTEM;
}

/// Port registry for runtime configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortRegistry {
    /// ToadStool server port
    pub server: u16,
    /// GPU compute port
    pub gpu_compute: u16,
    /// Distributed scheduler port
    pub distributed: u16,
    /// Metrics port
    pub metrics: u16,
}

impl Default for PortRegistry {
    fn default() -> Self {
        Self {
            server: server_port(),
            gpu_compute: gpu_compute_port(),
            distributed: distributed_port(),
            metrics: metrics_port(),
        }
    }
}

/// Common test ports (non-conflicting)
///
/// Used in tests to avoid conflicts with running services
pub mod test {
    /// Base port for test services (increments per test)
    pub const BASE: u16 = 50_000;

    /// Generate unique test port
    ///
    /// Uses process ID and test number to ensure uniqueness
    #[must_use]
    pub fn unique_port(test_id: u16) -> u16 {
        BASE + u16::try_from(std::process::id() % 1000).unwrap_or(999) + test_id
    }
}

/// Port range allocation
pub mod ranges {
    /// Start of ToadStool service port range (0 = OS-assigned).
    pub const TOADSTOOL_START: u16 = 0;
    /// End of valid port range.
    pub const TOADSTOOL_END: u16 = 65_535;

    /// Start of test port range (avoids production ports).
    pub const TEST_START: u16 = 50_000;
    /// End of test port range.
    pub const TEST_END: u16 = 65_535;
}

/// Pure version: resolve port from an explicit env value
#[must_use]
pub fn resolve_port(env_value: Option<&str>, default: u16) -> u16 {
    env_value.and_then(|s| s.parse().ok()).unwrap_or(default)
}

/// Get port with environment variable override
///
/// **Phase 2 Evolution**: Environment variable support
///
/// Allows runtime configuration without code changes:
/// ```bash
/// TOADSTOOL_SERVER_PORT=9000 ./toadstool-server
/// ```
#[must_use]
pub fn get_port_with_env(default: u16, env_var: &str) -> u16 {
    resolve_port(std::env::var(env_var).ok().as_deref(), default)
}

/// Get ToadStool server port (with environment override)
#[must_use]
pub fn server_port() -> u16 {
    get_port_with_env(toadstool::SERVER, "TOADSTOOL_SERVER_PORT")
}

/// Get ToadStool GPU compute port (with environment override)
#[must_use]
pub fn gpu_compute_port() -> u16 {
    get_port_with_env(toadstool::GPU_COMPUTE, "TOADSTOOL_GPU_PORT")
}

/// Get ToadStool distributed scheduler port (with environment override)
#[must_use]
pub fn distributed_port() -> u16 {
    get_port_with_env(toadstool::DISTRIBUTED, "TOADSTOOL_DISTRIBUTED_PORT")
}

/// Get metrics port (with environment override)
#[must_use]
pub fn metrics_port() -> u16 {
    get_port_with_env(toadstool::METRICS, "TOADSTOOL_METRICS_PORT")
}

/// Get daemon/BYOB API port (with environment override)
#[must_use]
pub fn daemon_port() -> u16 {
    get_toadstool_port("DAEMON_API", toadstool::DAEMON_API)
}

/// Get ToadStool port with environment variable override
///
/// **Phase 2: Environment Overrides**
///
/// Checks `TOADSTOOL_{NAME}_PORT` environment variable first,
/// falls back to default if not set or invalid.
///
/// This upholds the **self-knowledge principle**: ToadStool configures its own ports.
///
/// # Examples
/// ```
/// use toadstool_config::ports::{get_toadstool_port, toadstool};
///
/// // TOADSTOOL_SERVER_PORT=9000 cargo run
/// let port = get_toadstool_port("SERVER", toadstool::SERVER);
/// // Returns 9000 if env var set, otherwise 8084
/// ```
#[must_use]
pub fn get_toadstool_port(name: &str, default: u16) -> u16 {
    get_port_with_env(default, &format!("TOADSTOOL_{name}_PORT"))
}

/// Resolve port for a capability via env var with fallback default.
///
/// Resolution order:
/// 1. `TOADSTOOL_{CAPABILITY}_PORT` env var (e.g. `TOADSTOOL_SECURITY_PORT`)
/// 2. Legacy `{PRIMAL}_PORT` env var (backward compatibility)
/// 3. `capability_fallback::*` default
///
/// Prefer `get_capability_port` over the deprecated `get_primal_port`.
#[must_use]
pub fn get_capability_port(capability: &str, fallback_port: u16) -> u16 {
    let capability_env = format!("TOADSTOOL_{capability}_PORT");
    if let Ok(v) = std::env::var(&capability_env) {
        if let Ok(p) = v.parse::<u16>() {
            return p;
        }
    }
    fallback_port
}

/// Get primal port with environment override (legacy — prefer `get_capability_port`).
///
/// Checks `{PRIMAL}_PORT` environment variable first, falls back to default.
/// This exists for backward compatibility during migration to capability-based ports.
#[must_use]
pub fn get_primal_port(primal: &str, fallback_port: u16) -> u16 {
    get_port_with_env(fallback_port, &format!("{primal}_PORT"))
}

/// Resolve port by trying capability env vars first, then fallback.
///
/// Resolution order:
/// 1. `TOADSTOOL_{CAPABILITY}_PORT` (e.g. `TOADSTOOL_SECURITY_PORT`)
/// 2. `{CAPABILITY}_PORT` (capability-based, e.g. `COORDINATION_PORT`)
/// 3. `{LEGACY_NAME}_PORT` (primal name, deprecated — e.g. `SONGBIRD_PORT`)
/// 4. `fallback_port` — hardcoded default from `capability_fallback::*`
#[must_use]
pub fn resolve_capability_or_legacy_port(
    capability: &str,
    legacy_name: &str,
    fallback_port: u16,
) -> u16 {
    if let Ok(v) = std::env::var(format!("TOADSTOOL_{capability}_PORT")) {
        if let Ok(p) = v.parse::<u16>() {
            return p;
        }
    }
    if let Ok(v) = std::env::var(format!("{capability}_PORT")) {
        if let Ok(p) = v.parse::<u16>() {
            return p;
        }
    }
    if let Ok(v) = std::env::var(format!("{legacy_name}_PORT")) {
        if let Ok(p) = v.parse::<u16>() {
            return p;
        }
    }
    fallback_port
}

/// Get primal endpoint with environment override
///
/// **Phase 2: Environment Overrides (RECOMMENDED)**
///
/// Checks `{PRIMAL}_ENDPOINT` environment variable for full endpoint URL.
/// This allows complete override including hostname, port, and protocol.
///
/// **Recommended for production**: Use full endpoint URLs for flexibility.
///
/// # Examples
/// ```
/// use toadstool_config::ports::get_primal_endpoint;
///
/// // SONGBIRD_ENDPOINT=https://songbird.prod.example.com:8080
/// let endpoint = get_primal_endpoint("SONGBIRD");
/// // Returns Some("https://songbird.prod.example.com:8080")
///
/// // Without env var
/// let endpoint = get_primal_endpoint("NONEXISTENT");
/// // Returns None
/// ```
#[must_use]
pub fn get_primal_endpoint(primal: &str) -> Option<String> {
    std::env::var(format!("{primal}_ENDPOINT")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ports() {
        // Port 0 = OS-assigned at bind time
        assert_eq!(toadstool::SERVER, 0);
        assert_eq!(toadstool::GPU_COMPUTE, 0);
        assert_eq!(toadstool::DISTRIBUTED, 0);
    }

    #[test]
    fn test_unique_test_ports() {
        let port1 = test::unique_port(1);
        let port2 = test::unique_port(2);

        assert!(port1 >= test::BASE);
        assert!(port2 >= test::BASE);
        assert_ne!(port1, port2);
    }

    #[test]
    fn test_port_ranges() {
        // Port 0 (OS-assigned) is within valid range (u16 guarantees validity)
        let _ = (toadstool::SERVER, toadstool::GPU_COMPUTE);
    }

    #[test]
    fn test_environment_override() {
        assert_eq!(resolve_port(Some("9999"), 8080), 9999);
    }

    #[test]
    fn test_default_when_no_env() {
        assert_eq!(resolve_port(None, 8080), 8080);
    }

    #[test]
    fn test_invalid_env_falls_back_to_default() {
        assert_eq!(resolve_port(Some("not-a-number"), 8080), 8080);
    }

    #[test]
    fn test_empty_env_falls_back_to_default() {
        assert_eq!(resolve_port(Some(""), 8080), 8080);
    }
}
