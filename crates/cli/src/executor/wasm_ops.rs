// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// Verify SHA-256 checksum of data against an expected hex string.
///
/// Returns `Ok(())` when no checksum is supplied or when the data matches.
pub fn verify_sha256(data: &[u8], expected: &Option<String>) -> Result<()> {
    if let Some(expected) = expected {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual = format!("{:x}", hasher.finalize());
        if &actual != expected {
            return Err(crate::CliError::Other(format!(
                "WASM module checksum mismatch: expected {expected}, got {actual}"
            )));
        }
    }
    Ok(())
}

/// WASM operation implementations
impl BiomeExecutor {
    pub(super) async fn load_wasm_with_verification(
        &self,
        source: &str,
        expected_checksum: &Option<String>,
    ) -> Result<Vec<u8>> {
        let module_data = if source.starts_with("http://") || source.starts_with("https://") {
            return Err(crate::CliError::Other(
                "HTTP WASM loading not yet implemented".to_string(),
            ));
        } else {
            fs::read(source).await?
        };

        verify_sha256(&module_data, expected_checksum)?;
        if expected_checksum.is_some() {
            info!("✅ WASM module checksum verified");
        }

        Ok(module_data)
    }

    #[cfg(test)]
    #[expect(dead_code)]
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
            return Err(crate::CliError::Other(
                "WASM execution requires the 'wasm' feature. Build with: cargo build --features wasm".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_sha256_none_checksum() {
        let data = b"hello world";
        assert!(verify_sha256(data, &None).is_ok());
    }

    #[test]
    fn test_verify_sha256_correct_checksum() {
        let data = b"hello world";
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_sha256(data, &Some(expected.to_string())).is_ok());
    }

    #[test]
    fn test_verify_sha256_wrong_checksum() {
        let data = b"hello world";
        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_sha256(data, &Some(wrong.to_string()));
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("checksum mismatch"));
    }

    #[test]
    fn test_verify_sha256_empty_data() {
        let data = b"";
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(verify_sha256(data, &Some(expected.to_string())).is_ok());
    }

    #[tokio::test]
    async fn test_load_wasm_http_not_implemented() {
        let executor = super::BiomeExecutor::new().await.expect("executor");
        let result = executor
            .load_wasm_with_verification("http://example.com/module.wasm", &None)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP"));
    }

    #[tokio::test]
    async fn test_load_wasm_https_not_implemented() {
        let executor = super::BiomeExecutor::new().await.expect("executor");
        let result = executor
            .load_wasm_with_verification("https://example.com/module.wasm", &None)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP"));
    }

    #[tokio::test]
    async fn test_load_wasm_nonexistent_file() {
        let executor = super::BiomeExecutor::new().await.expect("executor");
        let result = executor
            .load_wasm_with_verification("/nonexistent/path/module.wasm", &None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_wasm_with_checksum_verification() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(b"hello world").expect("write");
        tmp.flush().expect("flush");
        let path = tmp.path().to_string_lossy().to_string();

        let executor = super::BiomeExecutor::new().await.expect("executor");
        let expected =
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string();
        let result = executor
            .load_wasm_with_verification(&path, &Some(expected))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"hello world");
    }

    #[tokio::test]
    async fn test_load_wasm_checksum_mismatch() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(b"wrong data").expect("write");
        tmp.flush().expect("flush");
        let path = tmp.path().to_string_lossy().to_string();

        let executor = super::BiomeExecutor::new().await.expect("executor");
        let wrong_checksum =
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string();
        let result = executor
            .load_wasm_with_verification(&path, &Some(wrong_checksum))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("checksum"));
    }
}
