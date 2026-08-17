// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    async_fn_in_trait,
    reason = "all implementors are Send + Sync; no dyn dispatch"
)]
#![allow(
    clippy::must_use_candidate,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::missing_errors_doc,
    reason = "WASM runtime API ergonomics; numeric casts are bounds-checked"
)]

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

// Pure modules — types, config, metrics (WASM-compatible)
pub mod cache_metrics;
pub mod config;
pub mod metrics;
#[cfg(feature = "runtime")]
pub mod wasi_context;

// Runtime-dependent modules (tokio::spawn, tokio::sync across await)
#[cfg(feature = "runtime")]
pub mod cache_wasmi;
#[cfg(feature = "runtime")]
pub mod engine_wasmi;
#[cfg(feature = "runtime")]
pub mod execution_wasmi;
#[cfg(feature = "runtime")]
pub mod module_loader;

// Component model: core types always available, registry/linker need runtime
pub mod component_model;

// Re-export public API
pub use cache_metrics::CacheMetrics;
pub use config::{SecurityLevel, WasmRuntimeConfig, WasmRuntimeConfigBuilder};

#[cfg(feature = "runtime")]
pub use cache_wasmi::ModuleCache;
#[cfg(feature = "runtime")]
pub use engine_wasmi::WasmRuntimeEngine;
#[cfg(feature = "runtime")]
pub use execution_wasmi::ModuleExecutor;
#[cfg(feature = "runtime")]
pub use module_loader::ModuleLoader;
#[cfg(feature = "runtime")]
pub use wasi_context::{WasiConfig, create_wasi_context};

pub use component_model::{
    ComponentInstance, ComponentInterface, ComponentModelConfig, ComponentModelSupport,
    ComponentResourceUsage, ComponentState, ComponentStats, ComponentValue, InterfaceFunction,
    InterfaceType,
};

#[cfg(feature = "runtime")]
pub use component_model::{ComponentLinker, ComponentRegistry};
