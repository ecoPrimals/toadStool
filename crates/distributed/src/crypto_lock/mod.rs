// SPDX-License-Identifier: AGPL-3.0-or-later
//! # `ToadStool` Crypto Lock System
//!
//! Cryptographic access control for external integrations:
//! - 🔓 Pure Rust ecosystem: Always unlocked, no crypto needed
//! - 🔐 External integrations: Require security provider crypto permissions
//! - 🔒 Security provider controls all access: Crypto keys and permissions
//! - 🚫 No phone home: Pure cryptographic proof system
//! - 🤝 Delegatable: People can lend access through security provider
//! - 🎯 Granular: Fine-grained permission control
//!
//! **Deep Debt**: Security provider discovered via Universal Adapter (no hardcoded primal names)
//!
//! ## Architecture
//!
//! This module is organized into 4 layers:
//! - **permissions**: Permission types and data structures (Layer 1)
//! - **validation**: Cryptographic validation and verification (Layer 2)
//! - **access_control**: Policy enforcement and access control (Layer 3)
//! - **cache**: Performance caching (Layer 4)

pub mod access_control;
pub mod cache;
pub mod permissions;
pub mod validation;

// Re-export all public types for backward compatibility
pub use access_control::*;
pub use cache::*;
pub use permissions::*;
pub use validation::*;

/// Converts days to `Duration` for permission validity.
#[must_use]
pub const fn duration_from_days(days: u64) -> std::time::Duration {
    std::time::Duration::from_secs(days * 86400)
}
