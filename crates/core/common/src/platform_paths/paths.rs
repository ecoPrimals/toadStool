// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use crate::constants::PRIMAL_NAME;

use super::env::{PathEnv, Platform};

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

        let temp = self.temp_dir();
        let username = self.env.user.as_deref().unwrap_or("default");

        match self.env.platform {
            Platform::Windows => temp.join(format!("toadstool-{username}")),
            Platform::Wasm => PathBuf::from("/virtual/toadstool"),
            _ => temp.join(format!("toadstool-runtime-{username}")),
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
                    return PathBuf::from(home).join("AppData/Roaming");
                }
                _ => {
                    return PathBuf::from(home).join(".local/share");
                }
            }
        }

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

    /// ToadStool socket directory (biomeOS standard: `{runtime}/biomeos/`)
    #[must_use]
    pub fn toadstool_socket_dir(&self) -> PathBuf {
        self.runtime_dir().join("biomeos")
    }

    /// ToadStool main socket path (domain-based per Self-Knowledge v1.1).
    #[must_use]
    pub fn toadstool_socket(&self) -> PathBuf {
        self.toadstool_socket_dir().join("compute.sock")
    }

    /// ToadStool JSON-RPC socket path (same as main socket since v1.1).
    #[must_use]
    pub fn toadstool_jsonrpc_socket(&self) -> PathBuf {
        self.toadstool_socket_dir().join("compute.sock")
    }

    /// ToadStool data directory
    #[must_use]
    pub fn toadstool_data_dir(&self) -> PathBuf {
        self.data_dir().join(PRIMAL_NAME)
    }

    /// ToadStool cache directory
    #[must_use]
    pub fn toadstool_cache_dir(&self) -> PathBuf {
        self.cache_dir().join(PRIMAL_NAME)
    }

    /// ToadStool log directory (in data dir for persistence)
    #[must_use]
    pub fn toadstool_log_dir(&self) -> PathBuf {
        self.toadstool_data_dir().join("logs")
    }

    /// ToadStool temp directory (session-scoped)
    #[must_use]
    pub fn toadstool_temp_dir(&self) -> PathBuf {
        self.temp_dir().join(PRIMAL_NAME)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::constants::PRIMAL_NAME;

    use super::super::env::{PathEnv, Platform};
    use super::PlatformPaths;

    #[test]
    fn path_resolution_prefers_xdg_when_all_set() {
        let env = PathEnv {
            xdg_runtime_dir: Some("/run/xdg".into()),
            xdg_data_home: Some("/data/xdg".into()),
            xdg_cache_home: Some("/cache/xdg".into()),
            xdg_config_home: Some("/config/xdg".into()),
            tmpdir: Some("/tmp/override".into()),
            platform: Platform::Linux,
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(paths.runtime_dir(), PathBuf::from("/run/xdg"));
        assert_eq!(paths.data_dir(), PathBuf::from("/data/xdg"));
        assert_eq!(paths.cache_dir(), PathBuf::from("/cache/xdg"));
        assert_eq!(paths.config_dir(), PathBuf::from("/config/xdg"));
        assert_eq!(paths.temp_dir(), PathBuf::from("/tmp/override"));
    }

    #[test]
    fn missing_xdg_uses_home_for_linux_data_cache_config() {
        let env = PathEnv {
            home: Some("/home/u".into()),
            platform: Platform::Linux,
            tmpdir: Some("/var/tmp".into()),
            ..Default::default()
        };
        let paths = PlatformPaths::new(&env);
        assert_eq!(paths.data_dir(), PathBuf::from("/home/u/.local/share"));
        assert_eq!(paths.cache_dir(), PathBuf::from("/home/u/.cache"));
        assert_eq!(paths.config_dir(), PathBuf::from("/home/u/.config"));
    }

    #[test]
    fn empty_home_and_no_xdg_falls_back_to_temp_subdirs() {
        let env = PathEnv {
            home: None,
            xdg_data_home: None,
            xdg_cache_home: None,
            xdg_config_home: None,
            xdg_runtime_dir: None,
            tmpdir: Some("/fixed/tmp".into()),
            user: Some("nobody".into()),
            platform: Platform::Linux,
        };
        let paths = PlatformPaths::new(&env);
        let base = PathBuf::from("/fixed/tmp");
        assert_eq!(paths.temp_dir(), base);
        assert_eq!(paths.data_dir(), base.join("toadstool-data"));
        assert_eq!(paths.cache_dir(), base.join("toadstool-cache"));
        assert_eq!(paths.config_dir(), base.join("toadstool-config"));
        assert_eq!(
            paths.toadstool_data_dir(),
            base.join("toadstool-data").join(PRIMAL_NAME)
        );
    }
}
