//! Centralized Constants Module
//!
//! This module provides a single source of truth for all hardcoded values
//! across the ToadStool codebase, improving maintainability and reducing
//! technical debt.
//!
//! ## Zero-Copy Optimization
//! String constants use `&'static str` for zero-cost sharing across the codebase.

pub mod compute;
pub mod display;
pub mod ecosystem;
pub mod jsonrpc;
pub mod network;
pub mod primal_identity;
pub mod resources;
pub mod timeouts;
pub mod versions;

// Re-export commonly used constants
pub use compute::*;
pub use display::*;
pub use jsonrpc::*;
pub use network::*;
pub use primal_identity::PRIMAL_NAME;
pub use resources::*;
pub use timeouts::*;
pub use versions::*;
