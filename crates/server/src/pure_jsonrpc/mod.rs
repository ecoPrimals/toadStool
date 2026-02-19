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
//! | `handler` | `JsonRpcHandler` — routing, method dispatch, SemanticMethodRegistry wiring |

mod handler;
mod types;

pub use handler::JsonRpcHandler;
pub use types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, JsonWorkloadSubmission};

#[cfg(test)]
mod tests;
