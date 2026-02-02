//! NPU Backend Module for BarraCUDA v2.0
//!
//! Event-driven ML execution on Akida neuromorphic processors.
//!
//! **Deep Debt Principles**:
//! - Pure Rust (using akida-driver)
//! - Runtime discovery (no hardcoded devices)
//! - Capability-based configuration
//! - Zero unsafe code
//! - Measured performance (not simulated)

pub mod ml_backend;
pub mod event_codec;
pub mod ops;

pub use ml_backend::NpuMlBackend;
pub use event_codec::EventCodec;
