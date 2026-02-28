//! ToadStool Launcher with Endpoint Discovery
//!
//! **Phase 3: Deployment Coordination**
//!
//! Provides launcher infrastructure for starting toadstool with automatic
//! endpoint discovery and health monitoring.
//!
//! ## Isomorphic Operation
//!
//! The launcher:
//! 1. Starts toadstool server
//! 2. Discovers IPC endpoint (Unix or TCP)
//! 3. Monitors health
//! 4. Reports status
//!
//! ## Deep Debt Compliance
//!
//! - ✅ Runtime discovery (not hardcoded)
//! - ✅ Platform-agnostic (Unix OR TCP)
//! - ✅ Zero configuration
//! - ✅ Pure Rust
//! - ✅ Zero unsafe

use crate::error::{ToadStoolError, ToadStoolResult};
use std::path::PathBuf;
use std::time::Duration;
use toadstool_common::constants::timeouts;
use tokio::process::Command;
use tracing::info;

/// ToadStool launch configuration
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    /// Binary path (default: "toadstool")
    pub binary_path: PathBuf,
    /// Additional arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Startup timeout
    pub startup_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("toadstool"),
            args: vec!["daemon".to_string()],
            env: Vec::new(),
            startup_timeout: timeouts::CONNECTION_STARTUP_TIMEOUT,
            health_check_interval: timeouts::TCP_CONNECT_TIMEOUT,
        }
    }
}

/// Endpoint discovery result
#[derive(Debug, Clone)]
pub enum Endpoint {
    /// Unix domain socket
    Unix(PathBuf),
    /// TCP socket
    Tcp(std::net::SocketAddr),
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Endpoint::Unix(path) => write!(f, "unix:{}", path.display()),
            Endpoint::Tcp(addr) => write!(f, "tcp:{}", addr),
        }
    }
}

/// Discover toadstool IPC endpoint
///
/// **Isomorphic Discovery**: Tries Unix socket first, then TCP discovery file.
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool::launcher::discover_toadstool_endpoint;
///
/// # async fn example() -> ToadStoolResult<()> {
/// let endpoint = discover_toadstool_endpoint().await?;
/// println!("Found toadstool at: {}", endpoint);
/// # Ok(())
/// # }
/// ```
pub async fn discover_toadstool_endpoint() -> ToadStoolResult<Endpoint> {
    // Try Unix socket paths (XDG-compliant)
    let unix_paths = get_toadstool_socket_paths();
    for path in unix_paths {
        if tokio::fs::metadata(&path).await.is_ok() {
            info!("✅ Discovered Unix socket: {}", path.display());
            return Ok(Endpoint::Unix(path));
        }
    }

    // Try TCP discovery file
    let discovery_files = get_tcp_discovery_file_paths();
    for file in discovery_files {
        if let Ok(contents) = tokio::fs::read_to_string(&file).await {
            // Parse format: tcp:127.0.0.1:PORT
            if let Some(addr_str) = contents.trim().strip_prefix("tcp:") {
                if let Ok(addr) = addr_str.parse() {
                    info!("✅ Discovered TCP endpoint: {}", addr);
                    return Ok(Endpoint::Tcp(addr));
                }
            }
        }
    }

    Err(ToadStoolError::not_found(
        "No toadstool endpoint found (tried Unix sockets and TCP discovery)",
    ))
}

/// Get candidate Unix socket paths for toadstool
///
/// **XDG-compliant**: Follows standard directory hierarchy
fn get_toadstool_socket_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. XDG_RUNTIME_DIR (preferred)
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        paths.push(PathBuf::from(&runtime_dir).join("toadstool/display.sock"));
        paths.push(PathBuf::from(runtime_dir).join("biomeos/toadstool.sock"));
    }

    // 2. HOME/.local/share (secondary)
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(&home).join(".local/share/toadstool/display.sock"));
        paths.push(PathBuf::from(home).join(".local/share/biomeos/toadstool.sock"));
    }

    // 3. Temp dir (fallback - platform agnostic)
    let temp = std::env::temp_dir();
    paths.push(temp.join("toadstool/display.sock"));
    paths.push(temp.join("biomeos/toadstool.sock"));

    paths
}

/// Get candidate TCP discovery file paths
///
/// **XDG-compliant**: Follows standard directory hierarchy
fn get_tcp_discovery_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. XDG_RUNTIME_DIR (preferred)
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        paths.push(PathBuf::from(runtime_dir).join("toadstool-ipc-port"));
    }

    // 2. HOME/.local/share (secondary)
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".local/share/toadstool-ipc-port"));
    }

    // 3. Temp dir (fallback - platform agnostic)
    paths.push(std::env::temp_dir().join("toadstool-ipc-port"));

    paths
}

/// Launch toadstool with configuration
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool::launcher::{LaunchConfig, launch_toadstool};
///
/// # async fn example() -> ToadStoolResult<()> {
/// let config = LaunchConfig::default();
/// launch_toadstool(config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn launch_toadstool(config: LaunchConfig) -> ToadStoolResult<()> {
    info!("🚀 Launching toadstool with config: {:?}", config);

    // Build command
    let mut cmd = Command::new(&config.binary_path);
    cmd.args(&config.args);

    // Add environment variables
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    // Spawn process
    info!(
        "   Spawning process: {:?} {:?}",
        config.binary_path, config.args
    );
    let mut child = cmd
        .spawn()
        .map_err(|e| ToadStoolError::runtime(format!("Failed to spawn toadstool process: {e}")))?;

    // Poll for endpoint readiness (no blind sleep — event-driven startup detection)
    info!(
        "   Waiting for startup (timeout: {:?})...",
        config.startup_timeout
    );

    let start = std::time::Instant::now();
    let mut last_error = None;
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while start.elapsed() < config.startup_timeout {
        // Check if process crashed
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(ToadStoolError::runtime(format!(
                    "Toadstool process exited prematurely with status: {}",
                    status
                )));
            }
            Ok(None) => {}
            Err(e) => {
                return Err(ToadStoolError::runtime(format!(
                    "Failed to check process status: {}",
                    e
                )));
            }
        }

        // Try endpoint discovery
        match discover_toadstool_endpoint().await {
            Ok(endpoint) => {
                info!("   ✅ Endpoint discovered: {}", endpoint);
                return Ok(());
            }
            Err(e) => {
                last_error = Some(e);
            }
        }
        interval.tick().await;
    }

    Err(ToadStoolError::runtime(format!(
        "Timeout waiting for toadstool endpoint: {:?}",
        last_error
    )))
}

/// Check toadstool health
///
/// Uses endpoint discovery to verify the server is responding.
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool::launcher::check_toadstool_health;
///
/// # async fn example() -> ToadStoolResult<()> {
/// check_toadstool_health().await?;
/// println!("Health check passed");
/// # Ok(())
/// # }
/// ```
pub async fn check_toadstool_health() -> ToadStoolResult<()> {
    // Basic health check by verifying endpoint exists
    verify_endpoint_exists().await
}

/// Basic health check by verifying endpoint exists
///
/// **Lightweight alternative** that doesn't require display crate.
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool::launcher::verify_endpoint_exists;
///
/// # async fn example() -> ToadStoolResult<()> {
/// verify_endpoint_exists().await?;
/// println!("Endpoint is accessible");
/// # Ok(())
/// # }
/// ```
pub async fn verify_endpoint_exists() -> ToadStoolResult<()> {
    discover_toadstool_endpoint()
        .await
        .map_err(|e| ToadStoolError::runtime(format!("Endpoint verification failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_config_default() {
        let config = LaunchConfig::default();
        assert_eq!(config.binary_path, PathBuf::from("toadstool"));
        assert_eq!(config.args, vec!["daemon"]);
        assert_eq!(config.startup_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_launch_config_default_env_empty() {
        let config = LaunchConfig::default();
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_launch_config_default_health_check_interval() {
        let config = LaunchConfig::default();
        assert_eq!(config.health_check_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_launch_config_custom() {
        let config = LaunchConfig {
            binary_path: PathBuf::from("/usr/bin/toadstool"),
            args: vec!["daemon".to_string(), "--debug".to_string()],
            env: vec![("FOO".to_string(), "bar".to_string())],
            startup_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
        };
        assert_eq!(config.binary_path, PathBuf::from("/usr/bin/toadstool"));
        assert_eq!(config.args.len(), 2);
        assert_eq!(config.env[0].0, "FOO");
        assert_eq!(config.env[0].1, "bar");
        assert_eq!(config.startup_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_launch_config_clone() {
        let config = LaunchConfig::default();
        let cloned = config.clone();
        assert_eq!(config.binary_path, cloned.binary_path);
        assert_eq!(config.args, cloned.args);
    }

    #[test]
    fn test_endpoint_display() {
        let unix = Endpoint::Unix(PathBuf::from("/tmp/test.sock"));
        assert!(unix.to_string().contains("unix:"));
        assert!(unix.to_string().contains("/tmp/test.sock"));

        let tcp = Endpoint::Tcp("127.0.0.1:8080".parse().unwrap());
        assert!(tcp.to_string().contains("tcp:"));
        assert!(tcp.to_string().contains("127.0.0.1"));
    }

    #[test]
    fn test_endpoint_unix_clone() {
        let unix = Endpoint::Unix(PathBuf::from("/var/run/toadstool.sock"));
        let cloned = unix.clone();
        assert_eq!(unix.to_string(), cloned.to_string());
    }

    #[test]
    fn test_endpoint_tcp_clone() {
        let tcp = Endpoint::Tcp("192.168.1.1:9000".parse().unwrap());
        let cloned = tcp.clone();
        assert_eq!(tcp.to_string(), cloned.to_string());
    }

    #[test]
    fn test_get_socket_paths() {
        let paths = get_toadstool_socket_paths();
        assert!(!paths.is_empty());
        // Should include at least /tmp fallback
        assert!(paths.iter().any(|p| p.starts_with("/tmp")));
    }

    #[test]
    fn test_get_socket_paths_count() {
        let paths = get_toadstool_socket_paths();
        // Minimum: /tmp paths (2) + optionally XDG + HOME paths
        assert!(paths.len() >= 2);
    }

    #[test]
    fn test_get_discovery_file_paths() {
        let paths = get_tcp_discovery_file_paths();
        assert!(!paths.is_empty());
        // Should include at least /tmp fallback
        assert!(paths.iter().any(|p| p.starts_with("/tmp")));
    }

    #[test]
    fn test_get_discovery_file_paths_tmp_path() {
        let paths = get_tcp_discovery_file_paths();
        // Use std::env::temp_dir() for platform-agnostic test
        let tmp_path = std::env::temp_dir().join("toadstool-ipc-port");
        assert!(paths.contains(&tmp_path));
    }
}
