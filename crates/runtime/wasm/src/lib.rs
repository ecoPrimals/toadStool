//! ToadStool - Universal Compute Platform  
//! Copyright (C) 2025 ToadStool Development Team
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU Affero General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
//! GNU Affero General Public License for more details.
//!
//! You should have received a copy of the GNU Affero General Public License
//! along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # `ToadStool` WebAssembly Runtime Engine
//!
//! 100% Pure Rust WebAssembly interpreter using wasmi 1.0.
//!
//! ## Evolution (Jan 17, 2026)
//!
//! **OLD**: wasmtime (JIT with C dependencies)
//! **NEW**: wasmi 1.0 (Pure Rust interpreter)
//!
//! ## Benefits
//!
//! - ✅ ZERO C dependencies (100% Pure Rust!)
//! - ✅ Trivial ARM cross-compilation
//! - ✅ Instant startup (no JIT warmup)
//! - ✅ Lower memory usage
//! - ✅ Better security (no JIT exploits)
//! - ✅ Simpler caching (Module is Clone!)
//!
//! ## Performance
//!
//! Wasmi is ~10x slower than wasmtime JIT, but ToadStool's typical workloads
//! are short-lived (seconds-minutes), making the interpreter perfect!
//!
//! For truly long-running WASM: Phase 2 will orchestrate wasmtime as subprocess.

// Module declarations
pub mod cache_wasmi;
pub mod cache_metrics;
pub mod config;
pub mod engine_wasmi;
pub mod metrics;
pub mod module_loader;
pub mod wasi_context;

// Component model: Phase 2 (orchestrate wasmtime subprocess)
#[cfg(feature = "component-model")]
pub mod component_model;

// Re-export public API
pub use cache_wasmi::ModuleCache;
pub use cache_metrics::CacheMetrics;
pub use config::{SecurityLevel, WasmRuntimeConfig, WasmRuntimeConfigBuilder};
pub use engine_wasmi::WasmRuntimeEngine;
pub use module_loader::ModuleLoader;
pub use wasi_context::{WasiConfig, create_wasi_context};

#[cfg(feature = "component-model")]
pub use component_model::*;

// Helper function for error conversion
use toadstool::error::ToadStoolError;

/// Helper function to convert wasmi errors to `ToadStoolError`
#[allow(dead_code)]
pub(crate) fn wasmi_error(err: wasmi::Error) -> ToadStoolError {
    ToadStoolError::runtime(err.to_string())
}
