// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration type definitions for legacy systems
//!
//! This module organizes legacy system configuration types by domain:
//! - **compilation**: Target formats, toolchains, optimization levels
//! - **communication**: Connection types, authentication, protocols
//! - **terminal**: Terminal types, session configs, encodings
//! - **storage**: Paper tape, ROM, disk image formats
//! - **management**: Job priorities, monitoring, administration
//! - **mainframe**: IBM mainframe-specific configuration
//! - **embedded**: Embedded systems configuration
//! - **industrial**: Industrial control systems (PLC, SCADA, etc.)
//! - **realtime**: Real-time operating system configuration
//! - **emulation**: Emulator configuration

pub mod compilation;
pub mod communication;
pub mod terminal;
pub mod storage;
pub mod management;
pub mod mainframe;
pub mod embedded;
pub mod industrial;
pub mod realtime;
pub mod emulation;

// Re-export all public types for backward compatibility
pub use compilation::*;
pub use communication::*;
pub use terminal::*;
pub use storage::*;
pub use management::*;
pub use mainframe::*;
pub use embedded::*;
pub use industrial::*;
pub use realtime::*;
pub use emulation::*;
