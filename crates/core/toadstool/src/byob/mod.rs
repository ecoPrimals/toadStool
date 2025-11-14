//! # BYOB (Bring Your Own Biome) Compute Execution
//!
//! Handles compute execution requests for team biome deployments.
//! Receives requests from Songbird and executes team services using Toadstool's
//! universal compute capabilities.

pub mod byob_impl;
pub mod byob_types;

// Re-export all public types and implementations
pub use byob_impl::*;
pub use byob_types::*;
