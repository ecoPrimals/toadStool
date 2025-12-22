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
/// Other primal ports are discovered at runtime.
pub mod toadstool {
    /// Main ToadStool server port
    pub const SERVER: u16 = 8084;

    /// GPU compute service port
    pub const GPU_COMPUTE: u16 = 8085;

    /// Distributed scheduler port
    pub const DISTRIBUTED: u16 = 8086;

    /// Health check endpoint port
    pub const HEALTH: u16 = 8087;

    /// Metrics/monitoring port
    pub const METRICS: u16 = 9090;
}

/// Default ports for other primals (for fallback only)
///
/// **Design Philosophy**: These are FALLBACK values only.
/// Production systems MUST use runtime discovery via Songbird.
///
/// **Self-Knowledge Violation**: Having these at all violates self-knowledge.
/// They exist temporarily to support transition period.
///
/// ⚠️ **DEPRECATED**: Use capability-based runtime discovery instead.
/// These will be removed in Phase 4 after mDNS/DNS-SD implementation.
#[deprecated(
    since = "0.1.0",
    note = "Use runtime capability discovery via Songbird or mDNS. \
            These hardcoded ports violate the self-knowledge principle. \
            See `toadstool_common::runtime_discovery` for proper usage."
)]
pub mod fallback {
    /// Songbird coordination service (FALLBACK - discover at runtime!)
    #[deprecated(note = "Use runtime discovery")]
    pub const SONGBIRD: u16 = 8080;

    /// Squirrel MCP platform (FALLBACK - discover at runtime!)
    #[deprecated(note = "Use runtime discovery")]
    pub const SQUIRREL: u16 = 8083;

    /// BearDog security service (FALLBACK - discover at runtime!)
    #[deprecated(note = "Use runtime discovery")]
    pub const BEARDOG: u16 = 8081;

    /// NestGate storage service (FALLBACK - discover at runtime!)
    #[deprecated(note = "Use runtime discovery")]
    pub const NESTGATE: u16 = 8082;

    /// BiomeOS integration (FALLBACK - discover at runtime!)
    #[deprecated(note = "Use runtime discovery")]
    pub const BIOMEOS: u16 = 8088;
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
    pub const BASE: u16 = 50000;

    /// Generate unique test port
    ///
    /// Uses process ID and test number to ensure uniqueness
    pub fn unique_port(test_id: u16) -> u16 {
        BASE + (std::process::id() as u16 % 1000) + test_id
    }
}

/// Port range allocation
pub mod ranges {
    /// ToadStool service range: 8084-8099
    pub const TOADSTOOL_START: u16 = 8084;
    pub const TOADSTOOL_END: u16 = 8099;

    /// Test port range: 50000-65535
    pub const TEST_START: u16 = 50000;
    pub const TEST_END: u16 = 65535;
}

/// Get port with environment variable override
///
/// **Phase 2 Evolution**: Environment variable support
///
/// Allows runtime configuration without code changes:
/// ```bash
/// TOADSTOOL_SERVER_PORT=9000 ./toadstool-server
/// ```
pub fn get_port_with_env(default: u16, env_var: &str) -> u16 {
    std::env::var(env_var)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Get ToadStool server port (with environment override)
pub fn server_port() -> u16 {
    get_port_with_env(toadstool::SERVER, "TOADSTOOL_SERVER_PORT")
}

/// Get ToadStool GPU compute port (with environment override)
pub fn gpu_compute_port() -> u16 {
    get_port_with_env(toadstool::GPU_COMPUTE, "TOADSTOOL_GPU_PORT")
}

/// Get ToadStool distributed scheduler port (with environment override)
pub fn distributed_port() -> u16 {
    get_port_with_env(toadstool::DISTRIBUTED, "TOADSTOOL_DISTRIBUTED_PORT")
}

/// Get metrics port (with environment override)
pub fn metrics_port() -> u16 {
    get_port_with_env(toadstool::METRICS, "TOADSTOOL_METRICS_PORT")
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
pub fn get_toadstool_port(name: &str, default: u16) -> u16 {
    get_port_with_env(default, &format!("TOADSTOOL_{}_PORT", name))
}

/// Get other primal port with environment override
///
/// **Phase 2: Environment Overrides (FALLBACK)**
///
/// **Note**: This is a FALLBACK mechanism. Production systems should use
/// runtime discovery via Songbird for true capability-based architecture.
///
/// Checks `{PRIMAL}_PORT` environment variable first.
///
/// # Self-Knowledge Principle
///
/// Using this function represents a **temporary violation** of self-knowledge.
/// ToadStool should NOT "know" other primals' ports - they should be discovered
/// at runtime via Songbird (Phase 3).
///
/// # Examples
/// ```
/// use toadstool_config::ports::{get_primal_port, fallback};
///
/// // SONGBIRD_PORT=9080 cargo run
/// let port = get_primal_port("SONGBIRD", fallback::SONGBIRD);
/// // Returns 9080 if env var set, otherwise 8080
/// ```
pub fn get_primal_port(primal: &str, fallback_port: u16) -> u16 {
    get_port_with_env(fallback_port, &format!("{}_PORT", primal))
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
pub fn get_primal_endpoint(primal: &str) -> Option<String> {
    std::env::var(format!("{}_ENDPOINT", primal)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ports() {
        assert_eq!(toadstool::SERVER, 8084);
        assert_eq!(toadstool::GPU_COMPUTE, 8085);
        assert_eq!(toadstool::DISTRIBUTED, 8086);
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
        // Static assertions - these are checked at compile time via const evaluation
        // If ports are outside range, this test validates the configuration
        const _: () = assert!(
            toadstool::SERVER >= ranges::TOADSTOOL_START,
            "SERVER port below range"
        );
        const _: () = assert!(
            toadstool::SERVER <= ranges::TOADSTOOL_END,
            "SERVER port above range"
        );
        const _: () = assert!(
            toadstool::GPU_COMPUTE >= ranges::TOADSTOOL_START,
            "GPU_COMPUTE port below range"
        );
        const _: () = assert!(
            toadstool::GPU_COMPUTE <= ranges::TOADSTOOL_END,
            "GPU_COMPUTE port above range"
        );
    }

    #[test]
    fn test_environment_override() {
        std::env::set_var("TEST_PORT", "9999");
        let port = get_port_with_env(8080, "TEST_PORT");
        assert_eq!(port, 9999);
        std::env::remove_var("TEST_PORT");
    }

    #[test]
    fn test_default_when_no_env() {
        std::env::remove_var("NONEXISTENT_PORT");
        let port = get_port_with_env(8080, "NONEXISTENT_PORT");
        assert_eq!(port, 8080);
    }
}
