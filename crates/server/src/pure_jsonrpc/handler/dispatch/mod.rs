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
mod device;
mod fan_out;
mod forward;
mod pipeline;
mod queries;
mod routing;
mod shader_dispatch;
mod sovereign;
mod state;
mod submit;
mod types;

#[cfg(test)]
mod tests;

use crate::visualization_client::SharedVisualizationClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;
use toadstool_ember::VfioAnchor;
use toadstool_ember::held_resource::HeldResource;
use toadstool_ember::vfio_handle::VfioResourceHandle;
use types::{DispatchJob, PipelineJob};

/// Shared collection of VFIO warm-keepalive anchors.
///
/// Each anchor holds dup'd VFIO fds for a GPU. On SIGTERM, all anchors
/// are leaked to prevent bus resets during daemon restart.
pub type AnchorStore = Arc<tokio::sync::Mutex<HashMap<String, VfioAnchor>>>;

/// Factory that produces a local `ComputeDevice` from a PCI BDF string.
pub(super) type LocalDeviceFactory =
    Arc<dyn Fn(&str) -> Option<Box<dyn toadstool_cylinder::ComputeDevice>> + Send + Sync>;

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
    /// Device pool — ember-managed VFIO handles keyed by BDF.
    /// Acquired before dispatch, released after completion.
    device_pool: Arc<RwLock<HashMap<String, HeldResource<VfioResourceHandle>>>>,
    /// Local compute device factory — produces ComputeDevice from BDF when
    /// cylinder can dispatch locally (Phase D). None = fall through to coral_client.
    local_device_factory: Option<LocalDeviceFactory>,
    /// Persistent cache of opened VFIO compute devices keyed by BDF.
    /// Devices hold iommufd/VFIO FDs and DMA mappings — dropping them
    /// triggers GPU reset. Cached to survive across multiple RPC calls.
    cached_devices: Arc<tokio::sync::Mutex<HashMap<String, Box<dyn toadstool_cylinder::ComputeDevice>>>>,
    /// Warm-keepalive anchors — dup'd VFIO fds that persist independently
    /// of cached_devices. On SIGTERM, these are leaked to prevent bus reset.
    anchor_store: AnchorStore,
    /// Multi-tenant GPU resource orchestrator (`None` = LocalDirect, zero overhead).
    resource_orchestrator: Option<Arc<toadstool_runtime_orchestration::ResourceOrchestrator>>,
}

/// Create a local device factory for Phase D sovereign dispatch.
#[cfg(target_os = "linux")]
pub(super) fn create_cylinder_device_factory() -> LocalDeviceFactory {
    device::create_cylinder_device_factory()
}

#[cfg(not(target_os = "linux"))]
pub(super) fn create_cylinder_device_factory() -> Option<LocalDeviceFactory> {
    device::create_cylinder_device_factory()
}