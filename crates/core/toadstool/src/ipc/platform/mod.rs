// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform abstraction layer for universal IPC
//!
//! **Deep Debt Principles**:
//! - ✅ Agnostic design (not hardcoded to one transport)
//! - ✅ Capability-based (auto-detect what's available)
//! - ✅ Safe Rust (no unsafe, pure Rust)
//! - ✅ Idiomatic (modern async/await)
//!
//! ## Supported Platforms
//!
//! | Platform | Transport | Status |
//! |----------|-----------|--------|
//! | Linux (desktop) | Unix + Abstract | ✅ Implemented |
//! | Android | Abstract | ✅ Implemented (this phase) |
//! | macOS | Unix | ✅ Implemented |
//! | Windows | TCP | ⏳ Future |
//! | Cross-device | TCP | ⏳ Future |

use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::path::PathBuf;

#[cfg(any(unix, target_os = "linux"))]
use toadstool_common::interned_strings::socket_env;

#[cfg(unix)]
pub mod unix;

#[cfg(target_os = "linux")]
pub mod abstract_socket;

pub mod tcp;

// Re-exports for convenience
#[cfg(unix)]
pub use unix::{bind as bind_unix, connect as connect_unix};

#[cfg(target_os = "linux")]
pub use abstract_socket::{bind as bind_abstract, connect as connect_abstract};

pub use tcp::{bind as bind_tcp, connect as connect_tcp};

/// Platform-agnostic socket endpoint
///
/// **Deep Debt**: Agnostic enum, not hardcoded types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Endpoint {
    /// Filesystem Unix socket (Linux, macOS)
    #[cfg(unix)]
    Unix {
        /// Socket path on filesystem.
        path: PathBuf,
    },

    /// Abstract Unix socket (Linux, Android)
    ///
    /// Uses Linux-specific abstract namespace (name starts with null byte).
    /// Doesn't create filesystem entry, better for Android (SELinux-friendly).
    #[cfg(target_os = "linux")]
    Abstract {
        /// Abstract socket name (e.g. `@biomeos_toadstool`).
        name: String,
    },

    /// TCP socket (universal fallback)
    Tcp {
        /// Host or IP address.
        host: String,
        /// Port number.
        port: u16,
    },
}

impl Endpoint {
    /// Get biomeOS-compliant endpoint for ToadStool
    ///
    /// **Deep Debt**: Runtime detection, not hardcoded
    pub fn for_toadstool() -> Self {
        #[cfg(target_os = "linux")]
        {
            // Android detection: Check for /system/bin or Termux markers
            if is_android() {
                // Android prefers abstract sockets (SELinux-friendly)
                return Self::Abstract {
                    name: "@biomeos_toadstool".to_string(),
                };
            }
        }

        #[cfg(unix)]
        {
            // Default: domain-based Unix socket per Self-Knowledge v1.1
            let runtime_dir = get_runtime_dir();
            Self::Unix {
                path: PathBuf::from(format!("{runtime_dir}/biomeos/compute.sock")),
            }
        }
        #[cfg(not(unix))]
        {
            // Windows and other non-Unix platforms: TCP fallback
            Self::Tcp {
                host: toadstool_common::constants::network::LOCALHOST_IPV4.to_string(),
                port: tcp::DEFAULT_PORT,
            }
        }
    }

    /// Check if endpoint is Unix socket
    #[cfg(unix)]
    pub const fn is_unix(&self) -> bool {
        matches!(self, Self::Unix { .. })
    }

    #[cfg(not(unix))]
    /// Unix sockets are unavailable on this platform.
    pub const fn is_unix(&self) -> bool {
        false
    }

    /// Check if endpoint is Abstract socket
    #[cfg(target_os = "linux")]
    pub const fn is_abstract(&self) -> bool {
        matches!(self, Self::Abstract { .. })
    }

    /// Check if endpoint is TCP
    pub const fn is_tcp(&self) -> bool {
        matches!(self, Self::Tcp { .. })
    }

    /// Get display string for endpoint
    ///
    /// **Deep Debt**: Human-readable, safe for logging
    pub fn display(&self) -> String {
        match self {
            #[cfg(unix)]
            Self::Unix { path } => format!("unix:{}", path.display()),
            #[cfg(target_os = "linux")]
            Self::Abstract { name } => format!("abstract:{name}"),
            Self::Tcp { host, port } => format!("tcp://{host}:{port}"),
        }
    }

    /// Get transport tier for fallback logic
    ///
    /// Tier 1: Preferred (Unix, Abstract)\
    /// Tier 2: Fallback (TCP)
    pub const fn tier(&self) -> TransportTier {
        match self {
            #[cfg(unix)]
            Self::Unix { .. } => TransportTier::Tier1,
            #[cfg(target_os = "linux")]
            Self::Abstract { .. } => TransportTier::Tier1,
            Self::Tcp { .. } => TransportTier::Tier2,
        }
    }
}

/// Transport tier for fallback logic
///
/// **Deep Debt**: Capability-based selection, not hardcoded
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportTier {
    /// Tier 1: Preferred local transports (Unix, Abstract)
    Tier1,
    /// Tier 2: Fallback for cross-device or unavailable Tier 1 (TCP)
    Tier2,
}

// ============================================================================
// Transport-injected connection (sourDough standard)
// ============================================================================

/// Connect to a peer using a launcher-injected `TransportEndpoint`.
///
/// Routes through the appropriate platform transport based on the endpoint type.
/// This is the canonical way to establish outbound IPC connections per the
/// sourDough transport standard.
///
/// # Errors
///
/// Returns error if the connection fails.
pub async fn connect_transport(
    endpoint: &toadstool_common::transport_endpoint::TransportEndpoint,
) -> crate::ToadStoolResult<ConnectedTransport> {
    use toadstool_common::transport_endpoint::TransportEndpoint;
    match endpoint {
        #[cfg(unix)]
        TransportEndpoint::Uds { path } => {
            let stream = connect_unix(path).await?;
            Ok(ConnectedTransport::Unix(stream))
        }
        #[cfg(not(unix))]
        TransportEndpoint::Uds { .. } => Err(crate::ToadStoolError::not_supported(
            "Unix domain socket transport is unavailable on this platform",
        )),
        TransportEndpoint::Tcp { host, port } => {
            let stream = connect_tcp(host, *port).await?;
            Ok(ConnectedTransport::Tcp(stream))
        }
        TransportEndpoint::MeshRelay {
            peer_id,
            capability,
        } => Err(crate::ToadStoolError::not_supported(format!(
            "mesh_relay transport not yet implemented (peer={peer_id}, cap={capability})"
        ))),
    }
}

/// A connected transport stream (result of `connect_transport`).
pub enum ConnectedTransport {
    /// Connected Unix domain socket.
    #[cfg(unix)]
    Unix(tokio::net::UnixStream),
    /// Connected TCP stream.
    Tcp(tokio::net::TcpStream),
}

// ============================================================================
// Platform Detection Helpers (Pure Rust, No Unsafe!)
// ============================================================================

/// Detect if running on Android
///
/// **Deep Debt**: Runtime capability detection, safe Rust
#[cfg(target_os = "linux")]
fn is_android() -> bool {
    // Check for Android system paths
    if std::path::Path::new("/system/bin/app_process").exists() {
        return true;
    }

    // Check for Termux (common on Android)
    if std::path::Path::new("/data/data/com.termux").exists() {
        return true;
    }

    // Check ANDROID_ROOT environment variable
    if std::env::var(socket_env::ANDROID_ROOT).is_ok() {
        return true;
    }

    false
}

/// Get `XDG_RUNTIME_DIR` or fallback.
#[cfg(unix)]
fn get_runtime_dir() -> String {
    if let Ok(dir) = std::env::var(socket_env::XDG_RUNTIME_DIR) {
        return dir;
    }
    #[cfg(unix)]
    if let Ok(uid) = toadstool_common::uid_detector::get_user_id() {
        return format!("/run/user/{uid}");
    }
    std::env::var(socket_env::BIOMEOS_RUNTIME_DIR).unwrap_or_else(|_| {
        std::env::temp_dir()
            .join("biomeos-runtime")
            .to_string_lossy()
            .into_owned()
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn test_endpoint_creation() {
        let endpoint = Endpoint::Unix {
            path: PathBuf::from("/tmp/test.sock"),
        };
        assert!(endpoint.is_unix());
        assert_eq!(endpoint.tier(), TransportTier::Tier1);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_abstract_endpoint() {
        let endpoint = Endpoint::Abstract {
            name: "@biomeos_test".to_string(),
        };
        assert!(endpoint.is_abstract());
        assert_eq!(endpoint.tier(), TransportTier::Tier1);
    }

    #[test]
    fn test_tcp_endpoint() {
        let endpoint = Endpoint::Tcp {
            host: "localhost".to_string(),
            port: 8080,
        };
        assert!(endpoint.is_tcp());
        assert_eq!(endpoint.tier(), TransportTier::Tier2);
    }

    #[test]
    fn test_tier_ordering() {
        assert!(TransportTier::Tier1 < TransportTier::Tier2);
    }

    #[test]
    fn test_default_endpoint() {
        let endpoint = Endpoint::for_toadstool();

        // Should create appropriate endpoint for platform
        #[cfg(all(target_os = "linux", unix))]
        {
            // Could be Unix or Abstract depending on Android detection
            assert!(endpoint.is_unix() || endpoint.is_abstract());
        }

        #[cfg(not(all(target_os = "linux", unix)))]
        {
            assert!(endpoint.is_tcp());
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_endpoint_serialization() {
        let endpoint = Endpoint::Unix {
            path: PathBuf::from("/tmp/test.sock"),
        };

        let json = serde_json::to_string(&endpoint).unwrap();
        let deserialized: Endpoint = serde_json::from_str(&json).unwrap();

        assert_eq!(endpoint, deserialized);
    }
}
