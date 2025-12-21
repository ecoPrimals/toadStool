//! Centralized Constants Module
//!
//! This module provides a single source of truth for all hardcoded values
//! across the ToadStool codebase, improving maintainability and reducing
//! technical debt.
//!
//! ## Zero-Copy Optimization
//! String constants use `&'static str` for zero-cost sharing across the codebase.

pub mod network;
pub mod resources;
pub mod timeouts;
pub mod versions;

// Re-export commonly used constants
pub use network::*;
pub use resources::*;
pub use timeouts::*;
pub use versions::*;
