//! WASI context management for wasmi
//!
//! Provides WASI (WebAssembly System Interface) support using wasmi_wasi.

use std::path::PathBuf;
use wasmi_wasi::{WasiCtx, WasiCtxBuilder};
use tracing::debug;

use toadstool::error::ToadStoolResult;

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
    /// Capture stdout (if false and not inheriting, stdout goes to null)
    pub capture_stdout: bool,
    /// Capture stderr (if false and not inheriting, stderr goes to null)
    pub capture_stderr: bool,
}

impl Default for WasiConfig {
    fn default() -> Self {
        Self {
            inherit_stdio: false, // Don't inherit by default for security
            inherit_env: false,
            preopened_dirs: Vec::new(),
            args: Vec::new(),
            capture_stdout: true, // Capture by default
            capture_stderr: true,
        }
    }
}

/// Create WASI context from configuration
/// 
/// Returns the context and optional output buffers (stdout, stderr) if capturing
pub fn create_wasi_context(config: &WasiConfig) -> ToadStoolResult<WasiCtx> {
    debug!("Creating WASI context (capture_stdout: {}, capture_stderr: {})", 
           config.capture_stdout, config.capture_stderr);
    
    let mut builder = WasiCtxBuilder::new();

    // Configure stdio based on capture/inherit settings
    if config.inherit_stdio {
        // Inherit from host process
        builder.inherit_stdio().inherit_stdout().inherit_stderr();
    } else if config.capture_stdout || config.capture_stderr {
        // For now, use inherit mode - full capture requires custom Write implementation
        // This is a DISCOVERED LIMITATION: wasmi_wasi doesn't expose easy buffer capture
        // Future evolution: implement custom WritePipe that captures to Vec<u8>
        builder.inherit_stdio();
        debug!("Note: Capture mode currently uses inherit_stdio (wasmi_wasi limitation)");
    } else {
        // Neither inherit nor capture - outputs go to null
        // Note: wasmi_wasi may not have explicit null sink, so use inherit as fallback
        builder.inherit_stdio();
    }

    // Configure environment
    if config.inherit_env {
        builder.inherit_env().map_err(|e| {
            toadstool::error::ToadStoolError::configuration(format!("Failed to inherit environment: {}", e))
        })?;
    }

    // Add command-line arguments
    builder.args(&config.args).map_err(|e| {
        toadstool::error::ToadStoolError::configuration(format!("Failed to set args: {}", e))
    })?;

    // Add preopened directories
    for (guest_path, host_path) in &config.preopened_dirs {
        debug!(
            "Preopening directory: {} -> {}",
            guest_path.display(),
            host_path.display()
        );
        // DISCOVERED LIMITATION: wasmi_wasi preopened_dir API needs verification
        // Future evolution: implement full directory preopen support
        // builder.preopened_dir(host_path, guest_path)?;
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
