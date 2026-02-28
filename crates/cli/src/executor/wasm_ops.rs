//! WASM Loading and Execution Operations
//!
//! This module contains WASM-specific operations:
//! - `load_wasm_with_verification()` - Load and verify WASM modules
//! - `execute_wasm_module()` - Execute WASM with runtime
//!
//! **Deep Debt Principles**:
//! - ✅ Verification before execution (security)
//! - ✅ Modern async patterns
//! - ✅ Proper error handling

use super::*;
use sha2::{Digest, Sha256};

/// WASM operation implementations
impl BiomeExecutor {
    pub(super) async fn load_wasm_with_verification(
        &self,
        source: &str,
        expected_checksum: &Option<String>,
    ) -> Result<Vec<u8>> {
        // Load WASM module from source
        let module_data = if source.starts_with("http://") || source.starts_with("https://") {
            // Download from URL
            return Err(crate::CliError::Other(
                "HTTP WASM loading not yet implemented".to_string(),
            ));
        } else {
            // Load from local file
            fs::read(source).await?
        };

        // Verify checksum if provided
        if let Some(expected) = expected_checksum {
            let mut hasher = Sha256::new();
            hasher.update(&module_data);
            let actual_checksum = format!("{:x}", hasher.finalize());

            if &actual_checksum != expected {
                return Err(crate::CliError::Other(format!(
                    "WASM module checksum mismatch: expected {expected}, got {actual_checksum}"
                )));
            }
            info!("✅ WASM module checksum verified");
        }

        Ok(module_data)
    }

    #[allow(dead_code)] // Reserved: WASM execution; used when feature "wasm" enabled
    pub(super) async fn execute_wasm_module(
        &self,
        module_data: &[u8],
        args: Vec<String>,
    ) -> Result<()> {
        #[cfg(feature = "wasm")]
        {
            use bytes::Bytes;
            use toadstool::workload::WasmModuleSource;
            use toadstool_runtime_wasm::{ModuleExecutor, WasmRuntimeConfig};
            use wasmi::Engine;

            let engine = Engine::default();
            let config = WasmRuntimeConfig::default();
            let executor = ModuleExecutor::new(engine, config);

            let source = WasmModuleSource::Bytes {
                data: Bytes::copy_from_slice(module_data),
            };

            // WASI modules typically export _start; Core WASM may use different entry points
            let entry_point = "_start";

            let _output = executor
                .load_and_execute(&source, entry_point, args)
                .await
                .map_err(|e| crate::CliError::Other(format!("WASM execution failed: {e}")))?;

            info!("✅ WASM module execution completed");
            Ok(())
        }

        #[cfg(not(feature = "wasm"))]
        {
            let _ = (module_data, args);
            bail!(
                "WASM execution requires the 'wasm' feature. Build with: cargo build --features wasm"
            );
        }
    }
}
