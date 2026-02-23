//! ToadStool Core - Hardware Infrastructure Layer
//!
//! Deep Debt: ToadStool directly interfaces with hardware in Rust
//! - No scripts, no sudo needed on fresh systems
//! - Self-evolves and adapts to hardware changes
//! - `BarraCuda` runs the math on all hardware via ToadStool

pub mod hardware;

pub use hardware::{HardwareDevice, HardwareManager, HardwareType};
