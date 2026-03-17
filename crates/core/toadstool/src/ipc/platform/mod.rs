// SPDX-License-Identifier: AGPL-3.0-only
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
use std::path::PathBuf;

pub mod unix;

#[cfg(target_os = "linux")]
pub mod abstract_socket;

pub mod tcp;

// Re-exports for convenience
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
    Unix { path: PathBuf },

    /// Abstract Unix socket (Linux, Android)
    ///
    /// Uses Linux-specific abstract namespace (name starts with null byte).
    /// Doesn't create filesystem entry, better for Android (SELinux-friendly).
    #[cfg(target_os = "linux")]
    Abstract { name: String },

    /// TCP socket (universal fallback)
    Tcp { host: String, port: u16 },
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

        // Default: Unix socket (Linux desktop, macOS)
        let runtime_dir = get_runtime_dir();
        Self::Unix {
            path: PathBuf::from(format!("{runtime_dir}/biomeos/toadstool.sock")),
        }
    }

    /// Check if endpoint is Unix socket
    pub const fn is_unix(&self) -> bool {
        matches!(self, Self::Unix { .. })
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
    if std::env::var("ANDROID_ROOT").is_ok() {
        return true;
    }

    false
}

/// Get XDG_RUNTIME_DIR or fallback
///
/// **Deep Debt**: Pure Rust UID detection (evolved from toadstool_common)
fn get_runtime_dir() -> String {
    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        // Pure Rust UID fallback (no unsafe libc!)
        toadstool_common::uid_detector::get_user_id().map_or_else(
            |_| "/tmp/biomeos-runtime".to_string(),
            |uid| format!("/run/user/{uid}"),
        )
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
        #[cfg(target_os = "linux")]
        {
            // Could be Unix or Abstract depending on Android detection
            assert!(endpoint.is_unix() || endpoint.is_abstract());
        }

        #[cfg(not(target_os = "linux"))]
        {
            assert!(endpoint.is_unix());
        }
    }

    #[test]
    fn test_endpoint_serialization() {
        let endpoint = Endpoint::Unix {
            path: PathBuf::from("/tmp/test.sock"),
        };

        let json = serde_json::to_string(&endpoint).unwrap();
        let deserialized: Endpoint = serde_json::from_str(&json).unwrap();

        assert_eq!(endpoint, deserialized);
    }
}
