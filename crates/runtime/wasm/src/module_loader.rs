// SPDX-License-Identifier: AGPL-3.0-or-later
//! Module loading for wasmi
//!
//! Handles loading WASM modules from various sources (file, bytes, URL)
//! with validation and caching support.

use sha2::{Digest, Sha256};
use std::time::Duration;
use tracing::debug;
use wasmi::{Engine, Module};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::workload::WasmModuleSource;

use crate::config::WasmRuntimeConfig;

fn encode_hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len().saturating_mul(2));
    for &b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Module loader for WASM modules
pub struct ModuleLoader {
    engine: Engine,
    config: WasmRuntimeConfig,
}

impl ModuleLoader {
    /// Create a new module loader
    pub const fn new(engine: Engine, config: WasmRuntimeConfig) -> Self {
        Self { engine, config }
    }

    /// Generate cache key for a module source
    pub fn generate_cache_key(&self, module_source: &WasmModuleSource) -> String {
        let mut hasher = Sha256::new();

        // Include source content in hash
        match module_source {
            WasmModuleSource::Bytes { data, .. } => {
                hasher.update(data);
            }
            WasmModuleSource::File { path, .. } => {
                hasher.update(path.to_string_lossy().as_bytes());
                // Include file modification time if available
                if let Ok(metadata) = std::fs::metadata(path)
                    && let Ok(modified) = metadata.modified()
                    && let Ok(duration) = modified.duration_since(std::time::SystemTime::UNIX_EPOCH)
                {
                    hasher.update(duration.as_secs().to_le_bytes());
                }
            }
            WasmModuleSource::Url { url, .. } => {
                hasher.update(url.as_bytes());
            }
        }

        // Include compilation configuration in hash
        hasher.update(self.config.max_memory_mb.to_le_bytes());
        hasher.update(self.config.max_pages.to_le_bytes());
        hasher.update([self.config.security_level as u8]);

        if let Some(fuel_limit) = self.config.fuel_limit {
            hasher.update(fuel_limit.to_le_bytes());
        }

        let hash = hasher.finalize();
        format!("wasm_{}", encode_hex_lower(&hash[..16]))
    }

    /// Load WASM module from various sources
    pub async fn load_module(&self, module_source: &WasmModuleSource) -> ToadStoolResult<Module> {
        let timeout_duration = Duration::from_millis(self.config.module_load_timeout_ms);

        let load_future = async {
            match module_source {
                WasmModuleSource::File { path } => {
                    debug!("Loading WASM module from file: {}", path.display());
                    let bytes = std::fs::read(path).map_err(|e| {
                        ToadStoolError::io(format!(
                            "Failed to read WASM file {}: {}",
                            path.display(),
                            e
                        ))
                    })?;
                    self.load_from_bytes(&bytes)
                }
                WasmModuleSource::Url { url: _ } => {
                    // URL loading feature-gated (removed for pure Rust)
                    Err(ToadStoolError::not_supported(
                        "URL module loading not enabled (pure Rust build)".to_string(),
                    ))
                }
                WasmModuleSource::Bytes { data } => {
                    debug!("Loading WASM module from bytes ({} bytes)", data.len());
                    self.load_from_bytes(data)
                }
            }
        };

        tokio::time::timeout(timeout_duration, load_future)
            .await
            .map_err(|_| {
                ToadStoolError::timeout(format!(
                    "Module load timeout: {}ms",
                    self.config.module_load_timeout_ms
                ))
            })?
    }

    /// Load module from bytes with validation
    fn load_from_bytes(&self, bytes: &[u8]) -> ToadStoolResult<Module> {
        // Validate module size
        let max_size_bytes = self.config.max_memory_mb as usize * 1024 * 1024;
        if bytes.len() > max_size_bytes {
            return Err(ToadStoolError::resource(format!(
                "WASM module size {} bytes exceeds limit of {} MB",
                bytes.len(),
                self.config.max_memory_mb
            )));
        }

        // Parse and validate module
        // wasmi 1.0: Module::new() instead of Module::from_binary()
        Module::new(&self.engine, bytes)
            .map_err(|e| ToadStoolError::validation(format!("Invalid WASM module: {e}")))
    }

    /// Get engine reference
    pub const fn engine(&self) -> &Engine {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cache_key_generation() {
        let engine = Engine::default();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);

        let source = WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0, 97, 115, 109]), // WASM magic bytes
        };

        let key = loader.generate_cache_key(&source);
        assert!(key.starts_with("wasm_"));
        assert_eq!(key.len(), 5 + 32); // "wasm_" + 32 hex chars
    }

    #[test]
    fn test_cache_key_different_sources_different_keys() {
        let engine = Engine::default();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);

        let source1 = WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0, 97, 115, 109]),
        };
        let source2 = WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0, 97, 115, 109, 1, 0, 0, 0]),
        };
        let key1 = loader.generate_cache_key(&source1);
        let key2 = loader.generate_cache_key(&source2);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_key_file_source() {
        let engine = Engine::default();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);

        let source = WasmModuleSource::File {
            path: PathBuf::from("/tmp/test.wasm"),
        };
        let key = loader.generate_cache_key(&source);
        assert!(key.starts_with("wasm_"));
    }

    #[test]
    fn test_loader_engine_access() {
        let engine = Engine::default();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);
        let _ = loader.engine();
    }

    #[tokio::test]
    async fn test_load_module_from_bytes_valid_wasm() {
        let engine = Engine::default();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);

        let wasm = wat::parse_str("(module)").unwrap();
        let source = WasmModuleSource::Bytes {
            data: bytes::Bytes::from(wasm),
        };
        let result = loader.load_module(&source).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_module_invalid_bytes_fails() {
        let engine = Engine::default();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);

        let source = WasmModuleSource::Bytes {
            data: bytes::Bytes::from(vec![0, 0, 0, 0]),
        };
        let result = loader.load_module(&source).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_module_url_not_supported() {
        let engine = Engine::default();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);

        let source = WasmModuleSource::Url {
            url: "https://example.com/module.wasm".to_string(),
        };
        let result = loader.load_module(&source).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .to_lowercase()
                .contains("url")
        );
    }
}
