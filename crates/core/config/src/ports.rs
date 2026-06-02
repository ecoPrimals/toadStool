// SPDX-License-Identifier: AGPL-3.0-or-later
//! Centralized Port Configuration
//!
//! **Capability-Based Discovery Evolution**
//!
//! Centralized port constants with environment override support.
//! - Phase 1: Centralize (this file) ✅
//! - Phase 2: Environment variable overrides ✅ (S203p — `socket_env` constants)
//! - Phase 3: Runtime discovery via coordination service
//! - Phase 4: Full mDNS + capability-based discovery

use toadstool_common::interned_strings::socket_env;

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

    /// Daemon/BYOB HTTP API port (0 = OS-assigned; override with `TOADSTOOL_DAEMON_API_PORT`)
    pub const DAEMON_API: u16 = 0;
}

/// Capability-based fallback ports for cold-start bootstrap.
///
/// # When these apply
///
/// Values here are **only** defaults when no discovery is available and no
/// `TOADSTOOL_{CAPABILITY}_PORT` (or related) environment override is set.
///
/// **Production deployments must not rely on these literals** — use
/// `RuntimeDiscovery::discover_capability()`, explicit endpoints, or config.
///
/// Capabilities map to port ranges rather than primal-specific assignments.
/// Each capability resolves via: env var → config file → this fallback.
pub mod capability_fallback {
    use toadstool_common::constants::discovery_ports::{
        DEFAULT_COORDINATION_PORT, DEFAULT_SECURITY_PORT, DEFAULT_STORAGE_PORT,
        DISPLAY_IPC_FALLBACK,
    };

    /// Coordination capability (orchestration, scheduling)
    pub const COORDINATION: u16 = DEFAULT_COORDINATION_PORT;

    /// Security capability (auth, policy, zero-trust)
    pub const SECURITY: u16 = DEFAULT_SECURITY_PORT;

    /// Storage capability (artifacts, pipelines)
    pub const STORAGE: u16 = DEFAULT_STORAGE_PORT;

    /// Platform / intelligence capability (MCP, model hosting)
    pub const PLATFORM: u16 = 8083;

    /// Ecosystem integration capability (biome management)
    pub const ECOSYSTEM: u16 = 8088;

    /// Ecosystem primary port (UI/API gateway)
    pub const ECOSYSTEM_PRIMARY: u16 = 8005;

    /// Shader compiler capability (WGSL/SPIR-V → native binary)
    pub const SHADER_COMPILER: u16 = 8090;

    /// Display IPC capability (local display server communication)
    pub const DISPLAY_IPC: u16 = DISPLAY_IPC_FALLBACK;
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
/// // Returns 9000 if env var set, otherwise the default (e.g. 0 for OS-assigned)
/// ```
#[must_use]
pub fn get_toadstool_port(name: &str, default: u16) -> u16 {
    get_port_with_env(default, &format!("TOADSTOOL_{name}_PORT"))
}

/// Resolve port for a capability via `TOADSTOOL_{CAPABILITY}_PORT` with fallback default.
///
/// Does not read `{CAPABILITY}_PORT`; use [`resolve_capability_port`] when that is needed.
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

/// Resolve a capability port using capability-scoped environment variables.
///
/// Lookup order (wateringHole / backward-compatible):
/// 1. `TOADSTOOL_{CAPABILITY}_PORT`
/// 2. `{CAPABILITY}_PORT`
/// 3. Legacy primal `{PRIMAL}_PORT` where applicable (`SONGBIRD_PORT` → coordination, …)
/// 4. For platform / intelligence: `TOADSTOOL_INTELLIGENCE_PORT`, `INTELLIGENCE_PORT`, then
///    `SQUIRREL_PORT`
/// 5. Numeric [`capability_fallback`] default
#[must_use]
pub fn resolve_capability_port(capability: &str, fallback_port: u16) -> u16 {
    fn parse_port(key: &str) -> Option<u16> {
        std::env::var(key).ok()?.parse().ok()
    }

    #[expect(deprecated, reason = "legacy primal port env names in capability resolution chain")]
    let ordered_keys: &[&str] = match capability {
        "COORDINATION" => &[
            socket_env::TOADSTOOL_COORDINATION_PORT,
            socket_env::COORDINATION_PORT,
            socket_env::SONGBIRD_PORT,
        ],
        "SECURITY" => &[
            socket_env::TOADSTOOL_SECURITY_PORT,
            socket_env::SECURITY_PORT,
            socket_env::BEARDOG_PORT,
        ],
        "STORAGE" => &[
            socket_env::TOADSTOOL_STORAGE_PORT,
            socket_env::STORAGE_PORT,
            socket_env::NESTGATE_PORT,
        ],
        "PLATFORM" => &[
            socket_env::TOADSTOOL_INTELLIGENCE_PORT,
            socket_env::TOADSTOOL_PLATFORM_PORT,
            socket_env::INTELLIGENCE_PORT,
            socket_env::PLATFORM_PORT,
            socket_env::SQUIRREL_PORT,
        ],
        _ => &[],
    };

    if !ordered_keys.is_empty() {
        for key in ordered_keys {
            if let Some(p) = parse_port(key) {
                return p;
            }
        }
        return fallback_port;
    }

    if let Some(p) = parse_port(&format!("TOADSTOOL_{capability}_PORT")) {
        return p;
    }
    if let Some(p) = parse_port(&format!("{capability}_PORT")) {
        return p;
    }
    fallback_port
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
