//! # Toadstool Display Backend
//!
//! **100% Pure Rust display and input backend** for ecoPrimals ecosystem.
//!
//! ## Mission
//!
//! Enable TRUE PRIMAL architecture where the compute primal (Toadstool) provisions
//! ALL hardware (display, input, GPU), allowing UI primals (petalTongue) to achieve
//! 100% Pure Rust with zero C dependencies.
//!
//! ## Architecture
//!
//! ```text
//! petalTongue (UI Primal)
//!    ↓ JSON-RPC over Unix sockets
//! Toadstool Display Backend
//!    ├── DRM/KMS (display hardware)
//!    ├── evdev (input devices)
//!    ├── Window Manager (multi-window)
//!    └── Framebuffer Ops (rendering)
//!    ↓ Direct hardware access
//! Hardware (GPU, display, keyboard, mouse)
//! ```
//!
//! ## Features
//!
//! - ✅ **100% Pure Rust** - Zero C dependencies!
//! - ✅ **DRM/KMS** - Direct display hardware control
//! - ✅ **evdev** - Universal input handling
//! - ✅ **Multi-window** - Multiple simultaneous windows
//! - ✅ **Async** - Modern async/await throughout
//! - ✅ **IPC** - JSON-RPC over Unix sockets
//! - ✅ **Capability-based** - Runtime discovery, no hardcoding
//!
//! ## Example
//!
//! ```rust,no_run
//! use toadstool_display::{DisplayServer, WindowManager};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize display backend
//!     let manager = WindowManager::new().await?;
//!     
//!     // Start IPC server
//!     let server = DisplayServer::new(manager)
//!         .bind("/run/user/1000/toadstool/display.sock")
//!         .await?;
//!     
//!     // Serve requests
//!     server.serve().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Deep Debt Compliance
//!
//! This crate adheres to Toadstool's Deep Debt principles:
//!
//! - ✅ 100% Pure Rust (validated!)
//! - ✅ Modern async Rust (tokio, futures)
//! - ✅ Capability-based discovery (no hardcoding)
//! - ✅ Complete implementation (no mocks in production)
//! - ✅ Safe abstractions (unsafe isolated and documented)
//! - ✅ Self-knowledge only (Toadstool discovers own hardware)
//!
//! ## Status
//!
//! **Phase 0**: Foundation (In Progress)  
//! **Version**: 0.1.0  
//! **Grade**: TBD (targeting S++)
//!
//! ## Collaboration
//!
//! Built in collaboration with petalTongue team to enable 100% Pure Rust GUI!

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs, rust_2018_idioms)]

// Public modules
pub mod capabilities;
pub mod drm;
pub mod input;
pub mod ipc;
pub mod window;

// Re-exports
pub use capabilities::DisplayCapabilities;
pub use drm::DrmBackend;
pub use input::{InputEvent, InputManager};
pub use ipc::{DisplayClient, DisplayServer};
pub use window::{Window, WindowId, WindowManager};

/// Display backend errors
#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    /// DRM device not found
    #[error("DRM device not found: {0}")]
    DeviceNotFound(std::path::PathBuf),

    /// Failed to open DRM device
    #[error("Failed to open DRM device: {0}")]
    OpenFailed(#[from] std::io::Error),

    /// DRM ioctl failed
    #[error("DRM ioctl failed: {0}")]
    IoctlFailed(String),

    /// Buffer allocation failed
    #[error("Buffer allocation failed")]
    AllocationFailed,

    /// Window not found
    #[error("Window not found: {0}")]
    WindowNotFound(WindowId),

    /// Input device error
    #[error("Input device error: {0}")]
    InputError(String),

    /// IPC error
    #[error("IPC error: {0}")]
    IpcError(String),
}

/// Result type for display operations
pub type Result<T> = std::result::Result<T, DisplayError>;

// Module-level documentation
#[doc(hidden)]
pub mod __doc {
    //! Internal documentation

    /// Version info
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Pure Rust status
    pub const PURE_RUST: bool = true;
}
