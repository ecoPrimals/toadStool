// SPDX-License-Identifier: AGPL-3.0-only
//! # Pure Rust JSON-RPC 2.0 Implementation
//!
//! **Pattern**: BearDog's proven manual implementation  
//! **Dependencies**: ZERO extra crates — only `serde_json` (already in workspace)  
//! **Status**: 100% Pure Rust — NO `jsonrpsee`, NO `ring`!
//!
//! ## Architecture
//!
//! ```text
//! Request
//!   → Parse JSON (types)
//!   → Validate version (handler)
//!   → Resolve semantic alias (SemanticMethodRegistry)
//!   → Dispatch to method handler
//!   → Serialize response (types)
//! ```
//!
//! ## Module layout
//!
//! | Module | Content |
//! |--------|---------|
//! | `types` | `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonWorkloadSubmission` |
//! | `handler` | `JsonRpcHandler` — thin coordinator; delegates to JobHandler, WorkloadHandler, ResourceHandler, TransportHandler, OllamaHandler |

mod connection;
mod handler;
mod types;

pub use connection::{serve_tcp, serve_unix};
pub use handler::JsonRpcHandler;
pub use types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, JsonWorkloadSubmission};

#[cfg(test)]
mod tests;
