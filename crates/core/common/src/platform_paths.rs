// SPDX-License-Identifier: AGPL-3.0-only
//! # Platform-Agnostic Path Resolution
//!
//! Pure Rust, zero-hardcoding path resolution for cross-platform compatibility.
//!
//! ## ecoBin v2.0 Compliance
//!
//! This module follows the ecoBin Architecture Standard v2.0 requirements:
//! - **No hardcoded paths**: Uses XDG, environment detection, and `std::env::temp_dir()`
//! - **Platform-agnostic**: Works on Linux, macOS, Windows, Android, WASM
//! - **Capability-based**: Self-knowledge only, discovers paths at runtime
//!
//! ## Path Resolution Priority
//!
//! 1. **Environment variable** (highest priority)
//! 2. **XDG standard** (Linux/Unix)
//! 3. **Platform standard** (macOS/Windows/Android)
//! 4. **Temp directory fallback** (universal)
//!
//! ## Usage
//!
//! ```
//! use toadstool_common::platform_paths::{PlatformPaths, PathEnv};
//!
//! // Production: capture from environment
//! let env = PathEnv::from_env();
//! let paths = PlatformPaths::new(&env);
//!
//! // Get runtime directory for sockets
//! let runtime_dir = paths.runtime_dir();
//!
//! // Get ToadStool-specific paths
//! let socket_dir = paths.toadstool_socket_dir();
//! ```

use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════
// Environment Snapshot (Testable, No Global State)
// ═══════════════════════════════════════════════════════════════════════════

/// Environment snapshot for path resolution.
///
/// Production code creates via `PathEnv::from_env()`.
/// Tests create with explicit values - no env var mutation needed.
///
/// ## Deep Debt Principles
///
/// - ✅ Testable without environment mutation
/// - ✅ Explicit dependencies (no hidden state)
/// - ✅ Pure functions operate on this snapshot
#[derive(Debug, Clone, Default)]
pub struct PathEnv {
    /// `XDG_RUNTIME_DIR` - Unix socket directory
    pub xdg_runtime_dir: Option<String>,
    /// `XDG_DATA_HOME` - User data directory
    pub xdg_data_home: Option<String>,
    /// `XDG_CACHE_HOME` - Cache directory
    pub xdg_cache_home: Option<String>,
    /// `XDG_CONFIG_HOME` - Config directory
    pub xdg_config_home: Option<String>,
    /// HOME directory
    pub home: Option<String>,
    /// USER name
    pub user: Option<String>,
    /// TMPDIR (Unix) or TMP/TEMP (Windows) - explicit override
    pub tmpdir: Option<String>,
    /// Current platform (detected)
    pub platform: Platform,
}

/// Platform detection for path resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Platform {
    /// Linux (including most distros)
    #[default]
    Linux,
    /// macOS
    MacOS,
    /// Windows
    Windows,
    /// Android (detected via /system/build.prop)
    Android,
    /// WebAssembly target
    Wasm,
    /// Unknown or unsupported platform
    Unknown,
}

impl PathEnv {
    /// Capture current environment (production use)
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            xdg_runtime_dir: std::env::var("XDG_RUNTIME_DIR").ok(),
            xdg_data_home: std::env::var("XDG_DATA_HOME").ok(),
            xdg_cache_home: std::env::var("XDG_CACHE_HOME").ok(),
            xdg_config_home: std::env::var("XDG_CONFIG_HOME").ok(),
            home: std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok(),
            user: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .ok(),
            tmpdir: std::env::var("TMPDIR")
                .or_else(|_| std::env::var("TMP"))
                .or_else(|_| std::env::var("TEMP"))
                .ok(),
            platform: Platform::detect(),
        }
    }

    /// Create for testing with specific runtime dir
    #[cfg(test)]
    #[must_use]
    pub fn with_runtime_dir(dir: &str) -> Self {
        Self {
            xdg_runtime_dir: Some(dir.to_string()),
            ..Default::default()
        }
    }

    /// Create for testing with full control
    #[cfg(test)]
    #[must_use]
    pub fn test_env() -> Self {
        let temp = std::env::temp_dir();
        let temp_str = temp.to_string_lossy().to_string();
        Self {
            xdg_runtime_dir: Some(format!("{temp_str}/test-runtime")),
            xdg_data_home: Some(format!("{temp_str}/test-data")),
            xdg_cache_home: Some(format!("{temp_str}/test-cache")),
            home: Some(temp_str.clone()),
            user: Some("testuser".to_string()),
            tmpdir: Some(temp_str),
            platform: Platform::Linux,
            ..Default::default()
        }
    }
}

impl Platform {
    /// Detect current platform
    #[must_use]
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            // Check for Android
            if std::path::Path::new("/system/build.prop").exists() {
                return Self::Android;
            }
            Self::Linux
        }
        #[cfg(target_os = "macos")]
        {
            Self::MacOS
        }
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::Wasm
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
            target_arch = "wasm32"
        )))]
        {
            Self::Unknown
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Platform Paths (Pure Functions on Environment Snapshot)
// ═══════════════════════════════════════════════════════════════════════════

/// Platform-agnostic path resolver.
///
/// Provides paths for:
/// - **Runtime**: Unix sockets, PID files, transient state
/// - **Data**: Persistent application data
/// - **Cache**: Regenerable cached data
/// - **Temp**: Temporary files (session-scoped)
/// - **Config**: Configuration files
pub struct PlatformPaths<'a> {
    env: &'a PathEnv,
}

impl<'a> PlatformPaths<'a> {
    /// Create path resolver from environment snapshot
    #[must_use]
    pub const fn new(env: &'a PathEnv) -> Self {
        Self { env }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Base Directory Resolution
    // ═══════════════════════════════════════════════════════════════════════

    /// Runtime directory for sockets and transient state.
    ///
    /// Resolution order:
    /// 1. `XDG_RUNTIME_DIR` (if set)
    /// 2. Platform-specific fallback using `temp_dir`
    ///
    /// ## Note
    ///
    /// Does NOT fall back to `/run/user/$UID` - that requires root knowledge.
    /// If `XDG_RUNTIME_DIR` is not set, uses temp directory.
    #[must_use]
    pub fn runtime_dir(&self) -> PathBuf {
        if let Some(ref xdg) = self.env.xdg_runtime_dir {
            return PathBuf::from(xdg);
        }

        // Platform-agnostic fallback: use temp_dir with user-specific subdir
        let temp = self.temp_dir();
        let username = self.env.user.as_deref().unwrap_or("default");

        match self.env.platform {
            Platform::Android => {
                // Android: abstract sockets preferred, but provide filesystem fallback
                temp.join(format!("toadstool-runtime-{username}"))
            }
            Platform::Windows => {
                // Windows: named pipes preferred, filesystem for discovery files
                temp.join(format!("toadstool-{username}"))
            }
            Platform::Wasm => {
                // WASM: in-memory only, but provide path for compatibility
                PathBuf::from("/virtual/toadstool")
            }
            _ => {
                // Linux/macOS: temp-based runtime
                temp.join(format!("toadstool-runtime-{username}"))
            }
        }
    }

    /// Temp directory (`std::env::temp_dir` with optional override).
    ///
    /// Uses `TMPDIR`/`TMP`/`TEMP` if set, otherwise `std::env::temp_dir()`.
    #[must_use]
    pub fn temp_dir(&self) -> PathBuf {
        self.env
            .tmpdir
            .as_ref()
            .map_or_else(std::env::temp_dir, PathBuf::from)
    }

    /// Data directory for persistent application data.
    ///
    /// Resolution order:
    /// 1. `XDG_DATA_HOME` (if set)
    /// 2. `$HOME/.local/share` (Linux/macOS)
    /// 3. `%APPDATA%` equivalent (Windows)
    #[must_use]
    pub fn data_dir(&self) -> PathBuf {
        if let Some(ref xdg) = self.env.xdg_data_home {
            return PathBuf::from(xdg);
        }

        if let Some(ref home) = self.env.home {
            match self.env.platform {
                Platform::MacOS => {
                    return PathBuf::from(home).join("Library/Application Support");
                }
                Platform::Windows => {
                    // On Windows, HOME is USERPROFILE; AppData is separate
                    return PathBuf::from(home).join("AppData/Roaming");
                }
                _ => {
                    return PathBuf::from(home).join(".local/share");
                }
            }
        }

        // Ultimate fallback
        self.temp_dir().join("toadstool-data")
    }

    /// Cache directory for regenerable data.
    ///
    /// Resolution order:
    /// 1. `XDG_CACHE_HOME` (if set)
    /// 2. `$HOME/.cache` (Linux)
    /// 3. Platform-specific (macOS/Windows)
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        if let Some(ref xdg) = self.env.xdg_cache_home {
            return PathBuf::from(xdg);
        }

        if let Some(ref home) = self.env.home {
            match self.env.platform {
                Platform::MacOS => {
                    return PathBuf::from(home).join("Library/Caches");
                }
                Platform::Windows => {
                    return PathBuf::from(home).join("AppData/Local/Temp");
                }
                _ => {
                    return PathBuf::from(home).join(".cache");
                }
            }
        }

        self.temp_dir().join("toadstool-cache")
    }

    /// Config directory for configuration files.
    #[must_use]
    pub fn config_dir(&self) -> PathBuf {
        if let Some(ref xdg) = self.env.xdg_config_home {
            return PathBuf::from(xdg);
        }

        if let Some(ref home) = self.env.home {
            match self.env.platform {
                Platform::MacOS => {
                    return PathBuf::from(home).join("Library/Preferences");
                }
                Platform::Windows => {
                    return PathBuf::from(home).join("AppData/Roaming");
                }
                _ => {
                    return PathBuf::from(home).join(".config");
                }
            }
        }

        self.temp_dir().join("toadstool-config")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ToadStool-Specific Paths
    // ═══════════════════════════════════════════════════════════════════════

    /// ToadStool socket directory (biomeOS standard: `{runtime}/biomeos/`)
    #[must_use]
    pub fn toadstool_socket_dir(&self) -> PathBuf {
        self.runtime_dir().join("biomeos")
    }

    /// ToadStool main socket path
    #[must_use]
    pub fn toadstool_socket(&self) -> PathBuf {
        self.toadstool_socket_dir().join("toadstool.sock")
    }

    /// ToadStool JSON-RPC socket path
    #[must_use]
    pub fn toadstool_jsonrpc_socket(&self) -> PathBuf {
        self.toadstool_socket_dir().join("toadstool.jsonrpc.sock")
    }

    /// ToadStool data directory
    #[must_use]
    pub fn toadstool_data_dir(&self) -> PathBuf {
        self.data_dir().join("toadstool")
    }

    /// ToadStool cache directory
    #[must_use]
    pub fn toadstool_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("toadstool")
    }

    /// ToadStool log directory (in data dir for persistence)
    #[must_use]
    pub fn toadstool_log_dir(&self) -> PathBuf {
        self.toadstool_data_dir().join("logs")
    }

    /// ToadStool temp directory (session-scoped)
    #[must_use]
    pub fn toadstool_temp_dir(&self) -> PathBuf {
        self.temp_dir().join("toadstool")
    }

    /// Display backend socket
    #[must_use]
    pub fn display_socket(&self) -> PathBuf {
        self.toadstool_socket_dir().join("display.sock")
    }

    /// Sandbox base directory
    #[must_use]
    pub fn sandbox_dir(&self) -> PathBuf {
        self.toadstool_data_dir().join("sandbox")
    }

    /// Sandbox temp directory
    #[must_use]
    pub fn sandbox_temp_dir(&self) -> PathBuf {
        self.toadstool_temp_dir().join("sandbox")
    }

    /// Discovery port file (for TCP fallback)
    #[must_use]
    pub fn ipc_port_file(&self) -> PathBuf {
        self.toadstool_socket_dir().join("toadstool-ipc-port")
    }

    /// JSON-RPC port file (for TCP fallback)
    #[must_use]
    pub fn jsonrpc_port_file(&self) -> PathBuf {
        self.toadstool_socket_dir().join("toadstool-jsonrpc-port")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // biomeOS Ecosystem Paths
    // ═══════════════════════════════════════════════════════════════════════

    /// biomeOS runtime directory
    #[must_use]
    pub fn biomeos_runtime_dir(&self) -> PathBuf {
        self.runtime_dir().join("biomeos")
    }

    /// Primal socket path by capability name
    #[must_use]
    pub fn primal_socket(&self, primal_name: &str) -> PathBuf {
        self.biomeos_runtime_dir()
            .join(format!("{primal_name}.sock"))
    }

    /// ecoPrimals discovery directory
    #[must_use]
    pub fn discovery_dir(&self) -> PathBuf {
        self.runtime_dir().join("ecoPrimals/discovery")
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Convenience Functions (Use Default Environment)
// ═══════════════════════════════════════════════════════════════════════════

/// Get runtime directory using current environment.
///
/// Prefer using `PlatformPaths` for testability.
#[must_use]
pub fn runtime_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).runtime_dir()
}

/// Get temp directory using current environment.
#[must_use]
pub fn temp_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).temp_dir()
}

/// Get ToadStool socket directory using current environment.
#[must_use]
pub fn toadstool_socket_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).toadstool_socket_dir()
}

/// Get ToadStool main socket path using current environment.
#[must_use]
pub fn toadstool_socket() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).toadstool_socket()
}

/// Get ToadStool temp directory using current environment.
#[must_use]
pub fn toadstool_temp_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).toadstool_temp_dir()
}

/// Get biomeOS runtime directory using current environment.
#[must_use]
pub fn biomeos_runtime_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).biomeos_runtime_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_dir_with_xdg() {
        let env = PathEnv {
            xdg_runtime_dir: Some("/run/user/1000".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(paths.runtime_dir(), PathBuf::from("/run/user/1000"));
    }

    #[test]
    fn test_runtime_dir_fallback() {
        let env = PathEnv {
            xdg_runtime_dir: None,
            user: Some("testuser".to_string()),
            tmpdir: Some("/tmp".to_string()),
            platform: Platform::Linux,
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        let runtime = paths.runtime_dir();
        assert!(
            runtime
                .to_string_lossy()
                .contains("toadstool-runtime-testuser")
        );
    }

    #[test]
    fn test_toadstool_socket_dir() {
        let env = PathEnv {
            xdg_runtime_dir: Some("/run/user/1000".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(
            paths.toadstool_socket_dir(),
            PathBuf::from("/run/user/1000/biomeos")
        );
    }

    #[test]
    fn test_toadstool_socket() {
        let env = PathEnv {
            xdg_runtime_dir: Some("/run/user/1000".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(
            paths.toadstool_socket(),
            PathBuf::from("/run/user/1000/biomeos/toadstool.sock")
        );
    }

    #[test]
    fn test_data_dir_with_xdg() {
        let env = PathEnv {
            xdg_data_home: Some("/home/user/.local/share".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(paths.data_dir(), PathBuf::from("/home/user/.local/share"));
    }

    #[test]
    fn test_primal_socket() {
        let env = PathEnv {
            xdg_runtime_dir: Some("/run/user/1000".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(
            paths.primal_socket("beardog"),
            PathBuf::from("/run/user/1000/biomeos/beardog.sock")
        );
    }

    #[test]
    fn test_temp_dir_override() {
        let env = PathEnv {
            tmpdir: Some("/custom/tmp".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(paths.temp_dir(), PathBuf::from("/custom/tmp"));
    }

    #[test]
    fn test_platform_detection() {
        let platform = Platform::detect();
        // Should detect something (not Unknown on supported platforms)
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        assert_ne!(platform, Platform::Unknown);
    }

    #[test]
    fn test_runtime_dir_android_platform() {
        let env = PathEnv {
            xdg_runtime_dir: None,
            user: Some("android_user".to_string()),
            tmpdir: Some("/data/local/tmp".to_string()),
            platform: Platform::Android,
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        let runtime = paths.runtime_dir();
        assert!(
            runtime
                .to_string_lossy()
                .contains("toadstool-runtime-android_user")
        );
    }

    #[test]
    fn test_runtime_dir_windows_platform() {
        let env = PathEnv {
            xdg_runtime_dir: None,
            user: Some("winuser".to_string()),
            tmpdir: Some("C:\\Temp".to_string()),
            platform: Platform::Windows,
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        let runtime = paths.runtime_dir();
        assert!(runtime.to_string_lossy().contains("toadstool-winuser"));
    }

    #[test]
    fn test_runtime_dir_wasm_platform() {
        let env = PathEnv {
            xdg_runtime_dir: None,
            platform: Platform::Wasm,
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        let runtime = paths.runtime_dir();
        assert_eq!(runtime, PathBuf::from("/virtual/toadstool"));
    }

    #[test]
    fn test_data_dir_linux_with_home() {
        let env = PathEnv {
            xdg_data_home: None,
            home: Some("/home/user".to_string()),
            platform: Platform::Linux,
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(paths.data_dir(), PathBuf::from("/home/user/.local/share"));
    }

    #[test]
    fn test_cache_dir_with_xdg() {
        let env = PathEnv {
            xdg_cache_home: Some("/home/user/.cache".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(paths.cache_dir(), PathBuf::from("/home/user/.cache"));
    }

    #[test]
    fn test_config_dir_with_xdg() {
        let env = PathEnv {
            xdg_config_home: Some("/home/user/.config".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(paths.config_dir(), PathBuf::from("/home/user/.config"));
    }

    #[test]
    fn test_path_env_from_env() {
        let env = PathEnv::from_env();
        // Should not panic; may or may not have values depending on test environment
        let _ = format!("{env:?}");
    }

    #[test]
    fn test_path_env_test_env() {
        let env = PathEnv::test_env();
        assert!(env.xdg_runtime_dir.is_some());
        assert_eq!(env.user.as_deref(), Some("testuser"));
    }

    #[test]
    fn test_display_socket() {
        let env = PathEnv {
            xdg_runtime_dir: Some("/run/user/1000".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(
            paths.display_socket(),
            PathBuf::from("/run/user/1000/biomeos/display.sock")
        );
    }

    #[test]
    fn test_ipc_port_file() {
        let env = PathEnv {
            xdg_runtime_dir: Some("/run/user/1000".to_string()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        let port_file = paths.ipc_port_file();
        assert!(port_file.to_string_lossy().contains("toadstool-ipc-port"));
    }
}
