//! IPC protocol (JSON-RPC 2.0 over Unix sockets)
//!
//! Provides client-server communication for display operations.
//!
//! ## Architecture
//!
//! ```text
//! petalTongue (Client)
//!    ↓ JSON-RPC over Unix socket
//! DisplayServer
//!    ↓ Calls WindowManager
//! WindowManager
//!    ↓ DRM/KMS operations
//! Hardware
//! ```
//!
//! ## Protocol Methods
//!
//! - `display.createWindow` - Create a new window
//! - `display.destroyWindow` - Destroy a window
//! - `display.resizeWindow` - Resize a window
//! - `display.getWindowInfo` - Get window information
//! - `display.subscribeInput` - Subscribe to input events
//! - `display.pollEvents` - Poll for pending events
//! - `display.getCapabilities` - Get display capabilities
//! - `display.present` - Present framebuffer (future: zero-copy)
//!
//! ## Example (Server)
//!
//! ```rust,no_run
//! use toadstool_display::{DisplayServer, WindowManager};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let manager = WindowManager::new().await?;
//! let server = DisplayServer::new(manager)
//!     .bind("/run/user/1000/toadstool/display.sock")
//!     .await?;
//!
//! server.serve().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Example (Client)
//!
//! ```rust,no_run
//! use toadstool_display::ipc::DisplayClient;
//! use toadstool_display::window::CreateWindowRequest;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut client = DisplayClient::connect("/run/user/1000/toadstool/display.sock").await?;
//!
//! let window_id = client.create_window(CreateWindowRequest::default()).await?;
//! let info = client.get_window_info(window_id).await?;
//!
//! println!("Window: {}x{}", info.width, info.height);
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod server;
pub mod types;

// Re-exports
pub use client::DisplayClient;
pub use server::DisplayServer;
pub use types::{
    DisplayCapabilitiesInfo, DisplayMethod, DisplayResult, JsonRpcError, JsonRpcRequest,
    JsonRpcResponse,
};

// ✅ Phase 1 COMPLETE:
// - JSON-RPC protocol types
// - Unix socket server
// - Client library
// - Complete method implementations
// - Deep Debt compliant!
