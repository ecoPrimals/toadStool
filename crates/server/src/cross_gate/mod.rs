// SPDX-License-Identifier: AGPL-3.0-only
//! Cross-Gate Compute Delegation
//!
//! Routes compute jobs to the best available GPU across the mesh.
//!
//! ## Architecture
//!
//! - Plasmodium knows all gates and their GPU capabilities
//! - Job router selects gate by: VRAM available, model already loaded, queue depth
//! - Jobs forwarded via Unix socket or TCP to remote toadStool instances
//! - Results returned through the relay
//!
//! ## Example
//!
//! Gate2 (RTX 3090, 24GB) is better for large models. Tower (RTX 4070)
//! is better for quick inference. The router picks the right gate automatically.

mod dispatcher;
mod router;
mod types;

#[cfg(test)]
mod tests;

pub use dispatcher::RemoteDispatcher;
pub use router::JobRouter;
pub use types::{
    GateGpuInfo, RemoteDispatchError, RoutingDecision, RoutingReason,
};
