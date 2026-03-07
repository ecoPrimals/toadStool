// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test utilities for wasmi runtime testing
//!
//! Pure Rust test utilities - NO MOCKS, real implementations!
//!
//! ## Deep Debt Principles Applied:
//! - ✅ Modern async patterns (`tokio::test`)
//! - ✅ No hardcoding (capability-based helpers)
//! - ✅ No mocks (real WASM modules)
//! - ✅ Fast AND safe (zero unsafe)

use toadstool::error::ToadStoolResult;
use wasmi::Module;

/// Create a simple WASM module for testing
///
/// Returns a module with basic arithmetic functions.
/// Pure Rust - no hardcoded paths, runtime-generated WAT!
pub fn create_simple_wasm_module() -> ToadStoolResult<Vec<u8>> {
    // Generate WAT at runtime (no hardcoded files!)
    let wat = r#"
        (module
            (func (export "add") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add
            )
            (func (export "multiply") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.mul
            )
            (func (export "_start")
                ;; WASI entry point (does nothing)
                nop
            )
        )
    "#;

    wat::parse_str(wat)
        .map_err(|e| toadstool::error::ToadStoolError::validation(format!("WAT parse error: {e}")))
}

/// Create a compute-intensive WASM module for fuel testing
///
/// Returns a module that consumes significant fuel.
/// Tests capability: fuel metering accuracy!
pub fn create_compute_intensive_wasm() -> ToadStoolResult<Vec<u8>> {
    let wat = r#"
        (module
            (func (export "fibonacci") (param i32) (result i32)
                (local i32 i32 i32)
                ;; Compute fibonacci recursively (fuel-intensive!)
                local.get 0
                i32.const 2
                i32.lt_s
                if (result i32)
                    local.get 0
                else
                    local.get 0
                    i32.const 1
                    i32.sub
                    call 0  ;; recursive call
                    local.set 1
                    
                    local.get 0
                    i32.const 2
                    i32.sub
                    call 0  ;; recursive call
                    local.set 2
                    
                    local.get 1
                    local.get 2
                    i32.add
                end
            )
            (func (export "_start")
                i32.const 10
                call 0
                drop
            )
        )
    "#;

    wat::parse_str(wat)
        .map_err(|e| toadstool::error::ToadStoolError::validation(format!("WAT parse error: {e}")))
}

/// Create a memory-intensive WASM module
///
/// Returns a module that allocates significant memory.
/// Tests capability: memory limit enforcement!
pub fn create_memory_intensive_wasm() -> ToadStoolResult<Vec<u8>> {
    let wat = r#"
        (module
            (memory (export "memory") 10)  ;; 10 pages = 640KB
            (func (export "allocate") (result i32)
                ;; Fill memory with data
                (local i32)
                (local.set 0 (i32.const 0))
                (block
                    (loop
                        (i32.store 
                            (local.get 0)
                            (i32.const 42)
                        )
                        (local.set 0 
                            (i32.add (local.get 0) (i32.const 4))
                        )
                        (br_if 0 
                            (i32.lt_u (local.get 0) (i32.const 65536))
                        )
                    )
                )
                local.get 0
            )
            (func (export "_start")
                call 0
                drop
            )
        )
    "#;

    wat::parse_str(wat)
        .map_err(|e| toadstool::error::ToadStoolError::validation(format!("WAT parse error: {e}")))
}

/// Create an invalid WASM module for error testing
///
/// Returns invalid WASM bytecode.
/// Tests capability: graceful error handling!
#[must_use]
pub fn create_invalid_wasm() -> Vec<u8> {
    // Invalid WASM magic number
    vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0xFF]
}

/// Create a WASM module with WASI capabilities
///
/// Returns a module that uses WASI for stdio.
/// Tests capability: WASI integration!
pub fn create_wasi_hello_world() -> ToadStoolResult<Vec<u8>> {
    let wat = r#"
        (module
            (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
            
            (memory (export "memory") 1)
            (data (i32.const 0) "Hello from Pure Rust WASM!\n")
            
            (func (export "_start")
                ;; Write "Hello" to stdout
                (i32.store (i32.const 100) (i32.const 0))   ;; iov.buf = 0
                (i32.store (i32.const 104) (i32.const 27))  ;; iov.len = 27
                
                (call $fd_write
                    (i32.const 1)    ;; stdout
                    (i32.const 100)  ;; iovs
                    (i32.const 1)    ;; iovs_len
                    (i32.const 200)  ;; nwritten
                )
                drop
            )
        )
    "#;

    wat::parse_str(wat)
        .map_err(|e| toadstool::error::ToadStoolError::validation(format!("WAT parse error: {e}")))
}

/// Verify a WASM module is valid
///
/// Capability-based validation - discovers what the module can do!
pub fn verify_wasm_module(engine: &wasmi::Engine, wasm: &[u8]) -> ToadStoolResult<Module> {
    Module::new(engine, wasm)
        .map_err(|e| toadstool::error::ToadStoolError::validation(format!("Invalid WASM: {e}")))
}

/// Helper to create test engine with specific configuration
///
/// Pure Rust configuration - no hardcoded values!
#[must_use]
pub fn create_test_engine() -> wasmi::Engine {
    let mut config = wasmi::Config::default();
    // Modern async-friendly configuration
    config.wasm_multi_value(true);
    config.wasm_mutable_global(true); // Note: singular, not plural!

    wasmi::Engine::new(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_simple_wasm() {
        let wasm = create_simple_wasm_module().unwrap();
        assert!(!wasm.is_empty());

        // Verify it's valid WASM (capability: validation!)
        let engine = create_test_engine();
        let _module = verify_wasm_module(&engine, &wasm).unwrap();

        // Module created successfully - no hardcoded expectations!
    }

    #[test]
    fn test_create_compute_intensive_wasm() {
        let wasm = create_compute_intensive_wasm().unwrap();
        assert!(!wasm.is_empty());

        let engine = create_test_engine();
        verify_wasm_module(&engine, &wasm).unwrap();
    }

    #[test]
    fn test_create_memory_intensive_wasm() {
        let wasm = create_memory_intensive_wasm().unwrap();
        assert!(!wasm.is_empty());

        let engine = create_test_engine();
        verify_wasm_module(&engine, &wasm).unwrap();
    }

    #[test]
    fn test_invalid_wasm_fails_validation() {
        let wasm = create_invalid_wasm();
        let engine = create_test_engine();

        // Should fail validation (capability: error detection!)
        let result = verify_wasm_module(&engine, &wasm);
        assert!(result.is_err());
    }

    #[test]
    fn test_wasi_hello_world() {
        let wasm = create_wasi_hello_world().unwrap();
        assert!(!wasm.is_empty());

        let engine = create_test_engine();
        verify_wasm_module(&engine, &wasm).unwrap();
    }
}
