// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign dispatch handler — the canonical GPU dispatch surface.
//!
//! Accepts compiled GPU binaries and routes them to the target GPU via VFIO,
//! DRM, or wgpu. Handles both direct submit (`compute.dispatch.submit`) and
//! shader pipeline (`shader.dispatch`) paths.
//!
//! Includes pipeline dispatch (`compute.dispatch.pipeline.*`) for ordered
//! multi-stage workloads with DAG-based dependency resolution.

mod capabilities;
mod dag;
#[cfg(target_os = "linux")]
mod device;
mod fan_out;
mod forward;
mod pipeline;
mod queries;
mod routing;
mod shader_dispatch;
#[cfg(target_os = "linux")]
mod sovereign;
mod state;
mod submit;
mod submit_params;
pub mod telemetry;
pub(crate) mod trust;
mod types;
mod wgpu_dispatch;

#[cfg(test)]
mod tests;

use crate::visualization_client::SharedVisualizationClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
#[cfg(target_os = "linux")]
use toadstool_ember::VfioAnchor;
#[cfg(target_os = "linux")]
use toadstool_ember::held_resource::HeldResource;
#[cfg(target_os = "linux")]
use toadstool_ember::vfio_handle::VfioResourceHandle;
use tokio::sync::RwLock;
use types::{DispatchJob, PipelineJob};

/// Shared collection of VFIO warm-keepalive anchors.
#[cfg(target_os = "linux")]
pub type AnchorStore = Arc<tokio::sync::Mutex<HashMap<String, VfioAnchor>>>;

/// Factory that produces a local `ComputeDevice` from a PCI BDF string.
#[cfg(target_os = "linux")]
pub(super) type LocalDeviceFactory =
    Arc<dyn Fn(&str) -> Option<Box<dyn toadstool_cylinder::ComputeDevice>> + Send + Sync>;

/// Handler for `compute.dispatch.*` JSON-RPC methods.
///
/// Includes single-shot dispatch (`compute.dispatch.submit`, `shader.dispatch`)
/// and ordered pipeline dispatch (`compute.dispatch.pipeline.submit`) for
/// multi-stage workloads like ML inference (tokenize → attention → FFN).
///
/// When a Tower crypto client is available (NUCLEUS composition), payloads are
/// encrypted via `crypto.encrypt` using the `compute` purpose key before dispatch,
/// and results are decrypted via `crypto.decrypt` on return.
pub struct DispatchHandler {
    coral_client: SharedVisualizationClient,
    crypto_client: Option<Arc<toadstool_distributed::crypto_integration::CryptoServiceClient>>,
    /// Cached compute purpose key (lazily fetched on first encrypted dispatch).
    /// Arc-wrapped to avoid cloning key material on every cache hit.
    cached_purpose_key: Arc<RwLock<Option<Arc<toadstool::encryption::EncryptionKey>>>>,
    jobs: Arc<RwLock<HashMap<String, DispatchJob>>>,
    pipelines: Arc<RwLock<HashMap<String, PipelineJob>>>,
    dispatch_count: AtomicU64,
    /// Device pool — ember-managed VFIO handles keyed by BDF.
    #[cfg(target_os = "linux")]
    device_pool: Arc<RwLock<HashMap<String, HeldResource<VfioResourceHandle>>>>,
    /// Local compute device factory — produces ComputeDevice from BDF when
    /// cylinder can dispatch locally (Phase D). None = fall through to coral_client.
    #[cfg(target_os = "linux")]
    local_device_factory: Option<LocalDeviceFactory>,
    /// Persistent cache of opened VFIO compute devices keyed by BDF.
    #[cfg(target_os = "linux")]
    cached_devices:
        Arc<tokio::sync::Mutex<HashMap<String, Box<dyn toadstool_cylinder::ComputeDevice>>>>,
    /// Warm-keepalive anchors — dup'd VFIO fds that persist independently
    /// of cached_devices. On SIGTERM, these are leaked to prevent bus reset.
    #[cfg(target_os = "linux")]
    anchor_store: AnchorStore,
    /// Multi-tenant GPU resource orchestrator (`None` = LocalDirect, zero overhead).
    resource_orchestrator: Option<Arc<toadstool_runtime_orchestration::ResourceOrchestrator>>,
    /// Hardware owner gate id for yield-to-owner bypass (shared with `JobHandler`).
    gate_ownership: Option<Arc<crate::cross_gate::GateOwnership>>,
}

/// Create a local device factory for Phase D sovereign dispatch.
#[cfg(target_os = "linux")]
pub(super) fn create_cylinder_device_factory() -> LocalDeviceFactory {
    device::create_cylinder_device_factory()
}
