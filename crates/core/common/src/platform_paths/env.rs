// SPDX-License-Identifier: AGPL-3.0-or-later

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
        use crate::interned_strings::socket_env;

        Self {
            xdg_runtime_dir: std::env::var(socket_env::XDG_RUNTIME_DIR).ok(),
            xdg_data_home: std::env::var(socket_env::XDG_DATA_HOME).ok(),
            xdg_cache_home: std::env::var(socket_env::XDG_CACHE_HOME).ok(),
            xdg_config_home: std::env::var(socket_env::XDG_CONFIG_HOME).ok(),
            home: std::env::var(socket_env::HOME)
                .or_else(|_| std::env::var(socket_env::USERPROFILE))
                .ok(),
            user: std::env::var(socket_env::USER)
                .or_else(|_| std::env::var(socket_env::USERNAME))
                .ok(),
            tmpdir: std::env::var(socket_env::TMPDIR)
                .or_else(|_| std::env::var(socket_env::TMP))
                .or_else(|_| std::env::var(socket_env::TEMP))
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
