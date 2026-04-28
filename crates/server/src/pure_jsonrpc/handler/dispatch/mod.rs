// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign dispatch handler — accepts compiled GPU binaries from visualization service
//! and routes them to the target GPU via VFIO or DRM.
//!
//! This is the missing link in the sovereign compute pipeline:
//! WGSL → visualization service compile → **toadStool dispatch** → GPU result
//!
//! Includes pipeline dispatch (`compute.dispatch.pipeline.*`) for ordered multi-stage
//! workloads — resolves neuralSpring upstream gap for ML inference scheduling.

mod capabilities;
mod dag;
mod forward;
mod pipeline;
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
use types::{DispatchJob, PipelineJob};

/// Handler for `compute.dispatch.*` JSON-RPC methods.
///
/// Includes single-shot dispatch (`compute.dispatch.submit`, `shader.dispatch`)
/// and ordered pipeline dispatch (`compute.dispatch.pipeline.submit`) for
/// multi-stage workloads like ML inference (tokenize → attention → FFN).
///
/// When a Tower security client is available (NUCLEUS composition), payloads are
/// encrypted via `crypto.encrypt` using the `compute` purpose key before dispatch,
/// and results are decrypted via `crypto.decrypt` on return.
#[expect(
    deprecated,
    reason = "SecurityClient delegates to crypto.encrypt/decrypt; crypto_integration migration tracked"
)]
pub struct DispatchHandler {
    coral_client: SharedVisualizationClient,
    security_client: Option<Arc<toadstool_distributed::security::client::SecurityClient>>,
    /// Cached compute purpose key (lazily fetched on first encrypted dispatch).
    cached_purpose_key: Arc<RwLock<Option<toadstool::encryption::EncryptionKey>>>,
    jobs: Arc<RwLock<HashMap<String, DispatchJob>>>,
    pipelines: Arc<RwLock<HashMap<String, PipelineJob>>>,
    dispatch_count: AtomicU64,
}

#[expect(
    deprecated,
    reason = "SecurityClient delegates to crypto.encrypt/decrypt; crypto_integration migration tracked"
)]
impl DispatchHandler {
    pub fn new(
        coral_client: SharedVisualizationClient,
        security_client: Option<Arc<toadstool_distributed::security::client::SecurityClient>>,
    ) -> Self {
        Self {
            coral_client,
            security_client,
            cached_purpose_key: Arc::new(RwLock::new(None)),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            pipelines: Arc::new(RwLock::new(HashMap::new())),
            dispatch_count: AtomicU64::new(0),
        }
    }
}
