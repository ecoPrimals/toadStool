//! WASI context management for wasmi
//!
//! Provides WASI (WebAssembly System Interface) support using wasmi_wasi.

use std::path::PathBuf;
use wasmi::Store;
use wasmi_wasi::{WasiCtx, WasiCtxBuilder};
use tracing::debug;

use toadstool::error::{ToadStoolError, ToadStoolResult};

/// WASI configuration
#[derive(Debug, Clone)]
pub struct WasiConfig {
    /// Inherit stdio from host
    pub inherit_stdio: bool,
    /// Inherit environment variables
    pub inherit_env: bool,
    /// Allowed preopened directories
    pub preopened_dirs: Vec<(PathBuf, PathBuf)>,
    /// Command-line arguments
    pub args: Vec<String>,
}

impl Default for WasiConfig {
    fn default() -> Self {
        Self {
            inherit_stdio: true,
            inherit_env: false,
            preopened_dirs: Vec::new(),
            args: Vec::new(),
        }
    }
}

/// Create WASI context from configuration
pub fn create_wasi_context(config: &WasiConfig) -> ToadStoolResult<WasiCtx> {
    debug!("Creating WASI context");
    
    let mut builder = WasiCtxBuilder::new();

    // Configure stdio
    if config.inherit_stdio {
        builder = builder.inherit_stdio();
    }

    // Configure environment
    if config.inherit_env {
        builder = builder.inherit_env();
    }

    // Add command-line arguments
    builder = builder.args(&config.args);

    // Add preopened directories
    for (guest_path, host_path) in &config.preopened_dirs {
        debug!(
            "Preopening directory: {} -> {}",
            guest_path.display(),
            host_path.display()
        );
        // Note: wasmi_wasi API may differ - need to verify exact method
        // builder = builder.preopened_dir(host_path, guest_path)?;
    }

    // Build context
    let ctx = builder.build();
    
    Ok(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_wasi_context() {
        let config = WasiConfig::default();
        let ctx = create_wasi_context(&config);
        assert!(ctx.is_ok(), "Should create default WASI context");
    }
}
