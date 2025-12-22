//! # BYOB (Bring Your Own Biome) Compute Execution
//!
//! Handles compute execution requests for team biome deployments.
//! Receives requests from Songbird and executes team services using Toadstool's
//! universal compute capabilities.
//!
//! ## Module Organization
//!
//! - `config` - Executor configuration and settings
//! - `deployment` - Deployment state and lifecycle management
//! - `byob_impl` - Core executor implementation
//! - `byob_types` - Type definitions

pub mod byob_impl;
pub mod byob_types;
pub mod config;
mod deployment; // Internal module
mod validation; // Internal validation logic

// Re-export all public types and implementations
pub use byob_impl::*;
pub use byob_types::*;
pub use config::*;
