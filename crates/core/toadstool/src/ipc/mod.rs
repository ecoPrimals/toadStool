// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal IPC module for ToadStool
//!
//! **Deep Debt Principles**:
//! - ✅ Primal autonomy (owned implementation, no shared crates)
//! - ✅ Modern idiomatic Rust (safe, async, capability-based)
//! - ✅ Universal deployment (Unix + Abstract + TCP)
//! - ✅ Isomorphic design (same code, different transports)
//! - ✅ Runtime discovery (discover primals, not hardcode)
//! - ✅ Self-knowledge (announce own capabilities)
//!
//! ## Architecture
//!
//! ```text
//! ToadStool IPC (owned by us):
//!   ├── Platform layer (Unix, Abstract, TCP)
//!   ├── Transport selection (auto-fallback)
//!   ├── JSON-RPC 2.0 protocol
//!   └── Songbird discovery integration
//! ```
//!
//! ## Evolution from Legacy
//!
//! This module evolves ToadStool's IPC from Unix-only to universal multi-transport,
//! following upstream `wateringHole/UNIVERSAL_IPC_STANDARD_V3.md`.
//!
//! **Phase 1 (Current)**: Module structure + Unix + Abstract (Android MVP)\
//! **Phase 2 (Future)**: Add TCP fallback for cross-device\
//! **Phase 3 (Future)**: Multi-transport server orchestration

pub mod client;
pub mod platform;
pub mod server;

// Re-export key types
pub use client::{IpcClient, IpcStream};
pub use platform::Endpoint;
pub use server::IpcServer;

// Re-export legacy helpers for backward compatibility
// These will gradually migrate to use the new platform layer
#[allow(deprecated)]
pub use crate::ipc_helpers::*;
