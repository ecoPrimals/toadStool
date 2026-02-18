//! Environment variable loader with zero-copy prefix and type-safe parsing.

use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use tracing::debug;

/// Environment variable configuration loader with type-safe parsing.
///
/// # Zero-Copy Optimization
///
/// The `prefix` field uses `Cow<'static, str>` to avoid allocations for the
/// common case of using the default "TOADSTOOL" prefix. This is a zero-cost
/// abstraction that only allocates when a custom prefix is provided.
#[derive(Debug, Clone)]
pub struct EnvConfigLoader {
    /// Environment prefix for `ToadStool` variables (zero-copy for defaults)
    prefix: Cow<'static, str>,
    /// Cache of loaded environment variables
    cache: HashMap<String, String>,
}

impl EnvConfigLoader {
    /// Create a new loader using the default `TOADSTOOL_` prefix (zero allocation).
    #[must_use]
    pub fn new() -> Self {
        Self {
            prefix: Cow::Borrowed("TOADSTOOL"),
            cache: HashMap::new(),
        }
    }

    /// Create a loader with a custom prefix (allocates once).
    #[must_use]
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: Cow::Owned(prefix.to_string()),
            cache: HashMap::new(),
        }
    }

    /// Load matching environment variables into the internal cache.
    pub fn load_cache(&mut self) {
        for (key, value) in env::vars() {
            if key.starts_with(self.prefix.as_ref()) {
                self.cache.insert(key, value);
            }
        }
        debug!("Loaded {} environment variables", self.cache.len());
    }

    /// Get environment variable as `String` with fallback.
    ///
    /// Empty prefix is handled correctly — no leading underscore is added.
    #[must_use]
    pub fn get_string(&self, key: &str, default: &str) -> String {
        let env_key = if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}_{}", self.prefix, key)
        };
        env::var(&env_key).unwrap_or_else(|_| default.to_string())
    }

    /// Get environment variable as `bool` with fallback.
    ///
    /// Accepts: `true/false`, `1/0`, `yes/no`, `on/off` (case-insensitive).
    #[must_use]
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            })
            .unwrap_or(default)
    }

    /// Get environment variable as `u16` with fallback.
    #[must_use]
    pub fn get_u16(&self, key: &str, default: u16) -> u16 {
        let env_key = if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}_{}", self.prefix, key)
        };
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as `u32` with fallback.
    #[must_use]
    pub fn get_u32(&self, key: &str, default: u32) -> u32 {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as `u64` with fallback.
    #[must_use]
    pub fn get_u64(&self, key: &str, default: u64) -> u64 {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as `f64` with fallback.
    #[must_use]
    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as `Duration` (value in whole seconds) with fallback.
    #[must_use]
    pub fn get_duration(&self, key: &str, default: Duration) -> Duration {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| {
                v.parse::<u64>()
                    .map(Duration::from_secs)
                    .map_err(|_| env::VarError::NotPresent)
            })
            .unwrap_or(default)
    }

    /// Get environment variable as `SocketAddr` with fallback.
    #[must_use]
    pub fn get_socket_addr(&self, key: &str, default: SocketAddr) -> SocketAddr {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as `PathBuf` with fallback.
    #[must_use]
    pub fn get_path(&self, key: &str, default: &str) -> PathBuf {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key).map_or_else(|_| PathBuf::from(default), PathBuf::from)
    }

    /// Get all environment variables whose keys start with `{self.prefix}_{prefix}`.
    #[must_use]
    pub fn get_prefixed(&self, prefix: &str) -> HashMap<String, String> {
        let full_prefix = format!("{}_{}", self.prefix, prefix);
        env::vars()
            .filter(|(key, _)| key.starts_with(&full_prefix))
            .collect()
    }
}

impl Default for EnvConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
