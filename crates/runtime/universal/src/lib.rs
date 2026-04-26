// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    async_fn_in_trait,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::wildcard_imports,
    clippy::return_self_not_must_use,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::struct_excessive_bools,
    clippy::single_match_else,
    clippy::redundant_closure_for_method_calls,
    clippy::manual_let_else,
    clippy::map_unwrap_or,
    clippy::unreadable_literal,
    clippy::unnecessary_wraps,
    clippy::no_effect_underscore_binding,
    reason = "universal runtime: async_fn_in_trait for execution traits; numeric casts and pedantic lints suppressed crate-wide"
)]

//! ToadStool Universal Compute Runtime
//!
//! A unified runtime that treats CPU, GPU, and neuromorphic processors as
//! different orders of the same parallel compute architecture.
//!
//! In-process GPU acceleration uses **wgpu** (Vulkan, Metal, Direct3D 12). OpenCL
//! is not implemented here; external OpenCL dispatch goes through **barraCuda** /
//! **coralReef** via IPC.
//!
//! # Philosophy
//!
//! **Not this** (Traditional):
//! ```text
//! CPU Code ≠ GPU Code ≠ NPU Code
//! Different APIs, different mental models
//! ```
//!
//! **This** (ToadStool):
//! ```text
//! Application → Universal Runtime → Optimal Compute Unit
//! Same API, automatic selection, capability-based
//! ```
//!
//! # Design Principles
//!
//! 1. **No Hardcoding** - Discover capabilities at runtime
//! 2. **Self-Knowledge** - Each unit knows only itself
//! 3. **Capability-Based** - Select based on what units can do
//! 4. **Pure Rust** - No mocks in production, complete implementations
//! 5. **Type-Safe** - Compiler-verified correctness
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool_runtime_universal::{ComputeError, UniversalRuntime};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ComputeError> {
//!     // Discover all available compute units
//!     let runtime = UniversalRuntime::discover().await?;
//!     
//!     println!("Available compute units: {}", runtime.num_units());
//!     for unit in runtime.units() {
//!         println!("  • {}", unit.name());
//!     }
//!     
//!     // Runtime selects optimal unit automatically
//!     let input = vec![1.0f32; 10_000];
//!     let result = runtime.execute_map_f32(input, |x| x * 2.0 + 1.0).await?;
//!     
//!     println!("Processed {} values", result.len());
//!     
//!     Ok(())
//! }
//! ```

pub mod backends;
pub mod capabilities;
pub mod compute_discovery_settings;
pub mod error;
pub mod runtime;
pub mod substrate; // ✅ NEW: Simplified substrate abstraction
pub mod types;

#[cfg(feature = "cpu")]
pub use backends::CpuComputeUnit;
#[cfg(feature = "wgpu-backend")]
pub use backends::WgpuComputeUnit;
pub use capabilities::{
    CapabilityDiscovery, LatencyRequirement, PowerConstraint, ThroughputRequirement,
    WorkloadProfile, WorkloadSize,
};
pub use compute_discovery_settings::ComputeDiscoverySettings;
pub use error::SubstrateError;
pub use runtime::{RuntimeStats, UniversalRuntime};
pub use substrate::{
    BufferMetadata, BufferOperation, BufferOutput, ComputeSubstrate, PerformanceMetrics,
    PowerMeasurement, SubstrateAdapter, SubstrateCapabilities, SubstrateType, UnaryOp,
};
pub use types::{
    Capabilities, ComputeError, ComputeUnit, ComputeUnitDispatch, ComputeUnitType, DataType,
    ExecutionModel, LatencyProfile, OperationType, Output, OutputMetadata, Parallelism, ParamValue,
    PowerProfile, Workload, WorkloadBuilder, WorkloadData, WorkloadParams,
};
