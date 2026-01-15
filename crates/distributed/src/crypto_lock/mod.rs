//! # `ToadStool` Crypto Lock System
//!
//! Cryptographic access control for external integrations:
//! - 🔓 Pure Rust ecosystem: Always unlocked, no crypto needed
//! - 🔐 External integrations: Require `BearDog` crypto permissions
//! - 🐻 `BearDog` controls all access: Crypto keys and permissions
//! - 🚫 No phone home: Pure cryptographic proof system
//! - 🤝 Delegatable: People can lend access through `BearDog`
//! - 🎯 Granular: Fine-grained permission control
//!
//! ## Architecture
//!
//! This module is organized into 4 layers:
//! - **permissions**: Permission types and data structures (Layer 1)
//! - **validation**: Cryptographic validation and verification (Layer 2)
//! - **access_control**: Policy enforcement and access control (Layer 3)
//! - **cache**: Performance caching (Layer 4)

pub mod permissions;
pub mod validation;
pub mod access_control;
pub mod cache;

// Re-export all public types for backward compatibility
pub use permissions::*;
pub use validation::*;
pub use access_control::*;
pub use cache::*;

// Helper function for duration from days
#[must_use]
pub const fn duration_from_days(days: u64) -> std::time::Duration {
    std::time::Duration::from_secs(days * 86400)
}
