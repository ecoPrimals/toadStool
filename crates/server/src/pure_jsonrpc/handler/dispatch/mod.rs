// SPDX-License-Identifier: AGPL-3.0-only
//! Sovereign dispatch handler — accepts compiled GPU binaries from coralReef
//! and routes them to the target GPU via VFIO or DRM.
//!
//! This is the missing link in the sovereign compute pipeline:
//! barraCuda WGSL → coralReef compile → **toadStool dispatch** → GPU result

mod capabilities;
mod forward;
mod queries;
mod routing;
mod submit;
mod types;

#[cfg(test)]
mod tests;

use crate::coral_reef_client::SharedCoralReefClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;

use types::DispatchJob;

/// Handler for `compute.dispatch.*` JSON-RPC methods.
pub struct DispatchHandler {
    coral_client: SharedCoralReefClient,
    jobs: Arc<RwLock<HashMap<String, DispatchJob>>>,
    dispatch_count: AtomicU64,
}

impl DispatchHandler {
    pub fn new(coral_client: SharedCoralReefClient) -> Self {
        Self {
            coral_client,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            dispatch_count: AtomicU64::new(0),
        }
    }
}
