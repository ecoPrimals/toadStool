#![deny(unsafe_code)]

//! ToadStool Universal Compute Runtime
//!
//! A unified runtime that treats CPU, GPU, and neuromorphic processors as
//! different orders of the same parallel compute architecture.
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
//! use toadstool_runtime_universal::UniversalRuntime;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
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
pub mod runtime;
pub mod substrate; // ✅ NEW: Simplified substrate abstraction
pub mod types;

pub use backends::*;
pub use capabilities::*;
pub use runtime::*;
pub use substrate::*; // ✅ Export substrate traits
pub use types::*;
