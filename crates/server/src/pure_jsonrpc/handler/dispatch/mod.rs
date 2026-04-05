// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign dispatch handler — accepts compiled GPU binaries from coralReef
//! and routes them to the target GPU via VFIO or DRM.
//!
//! This is the missing link in the sovereign compute pipeline:
//! barraCuda WGSL → coralReef compile → **toadStool dispatch** → GPU result

mod capabilities;
mod forward;
mod queries;
mod routing;
mod shader_dispatch;
mod submit;
mod types;

#[cfg(test)]
mod tests;

use crate::visualization_client::SharedVisualizationClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;

use types::DispatchJob;

/// Handler for `compute.dispatch.*` JSON-RPC methods.
pub struct DispatchHandler {
    coral_client: SharedVisualizationClient,
    jobs: Arc<RwLock<HashMap<String, DispatchJob>>>,
    dispatch_count: AtomicU64,
}

impl DispatchHandler {
    pub fn new(coral_client: SharedVisualizationClient) -> Self {
        Self {
            coral_client,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            dispatch_count: AtomicU64::new(0),
        }
    }
}
