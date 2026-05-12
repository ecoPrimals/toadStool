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
use toadstool_ember::vfio_handle::VfioResourceHandle;
use toadstool_ember::held_resource::HeldResource;
use types::{DispatchJob, PipelineJob};

/// Factory that produces a local `ComputeDevice` from a PCI BDF string.
type LocalDeviceFactory =
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
            device_pool: Arc::new(RwLock::new(HashMap::new())),
            local_device_factory: None,
        }
    }

    /// Acquire a device handle from the pool, creating one if absent.
    ///
    /// This is the lifecycle hook for Phase A — tracks which devices are
    /// actively being used for dispatch. When Phase D enables local VFIO
    /// dispatch, this handle will actually open and hold the device fd.
    pub(super) async fn acquire_device_handle(&self, bdf: &str) {
        let mut pool = self.device_pool.write().await;
        if pool.contains_key(bdf) {
            tracing::debug!(bdf, "ember: reusing existing device handle");
        } else {
            let handle = VfioResourceHandle::new(bdf.to_string());
            let held = HeldResource::new(handle);
            tracing::info!(bdf, "ember: device handle acquired for dispatch");
            pool.insert(bdf.to_string(), held);
        }
    }

    /// Return the number of actively held device handles.
    pub(super) async fn held_device_count(&self) -> usize {
        let pool = self.device_pool.read().await;
        pool.values().filter(|h| h.is_alive()).count()
    }

    /// Set the local device factory for Phase D sovereign dispatch.
    ///
    /// When set, dispatch attempts local execution before falling back to
    /// coral_client IPC. The factory receives a BDF and returns a `ComputeDevice`
    /// if the device can be opened locally.
    #[allow(dead_code, reason = "wired when NvVfioComputeDevice is absorbed into cylinder")]
    pub fn set_local_device_factory(
        &mut self,
        factory: LocalDeviceFactory,
    ) {
        self.local_device_factory = Some(factory);
    }

    /// Attempt local dispatch through cylinder's ComputeDevice.
    ///
    /// Returns `Some(result)` if local dispatch was attempted (success or failure).
    /// Returns `None` if no local device factory is configured or the device
    /// cannot be opened locally — caller should fall through to coral_client.
    pub(super) async fn try_local_dispatch(
        &self,
        bdf: &str,
        binary: &[u8],
        workgroup_size: [u32; 3],
        shader_info: Option<&serde_json::Value>,
    ) -> Option<Result<serde_json::Value, String>> {
        let factory = self.local_device_factory.as_ref()?;

        let pool = self.device_pool.read().await;
        let held = pool.get(bdf)?;
        if !held.is_alive() {
            tracing::warn!(bdf, "local dispatch: device handle not alive");
            return None;
        }
        drop(pool);

        let mut device = factory(bdf)?;

        tracing::info!(bdf, binary_len = binary.len(), "Phase D: local dispatch via cylinder");

        let dims = toadstool_cylinder::DispatchDims::new(
            workgroup_size[0],
            workgroup_size[1],
            workgroup_size[2],
        );

        let info = if let Some(si) = shader_info {
            toadstool_cylinder::ShaderInfo {
                gpr_count: si.get("gpr_count").and_then(serde_json::Value::as_u64).unwrap_or(0) as u32,
                shared_mem_bytes: si.get("shared_mem_bytes").and_then(serde_json::Value::as_u64).unwrap_or(0) as u32,
                barrier_count: si.get("barrier_count").and_then(serde_json::Value::as_u64).unwrap_or(0) as u32,
                workgroup: workgroup_size,
                wave_size: si.get("wave_size").and_then(serde_json::Value::as_u64).unwrap_or(32) as u32,
                local_mem_bytes: si.get("local_mem_bytes").and_then(serde_json::Value::as_u64).map(|v| v as u32),
            }
        } else {
            toadstool_cylinder::ShaderInfo {
                workgroup: workgroup_size,
                ..Default::default()
            }
        };

        match device.dispatch(binary, &[], dims, &info) {
            Ok(()) => {
                if let Err(e) = device.sync() {
                    return Some(Err(format!("local dispatch sync failed: {e}")));
                }
                Some(Ok(serde_json::json!({
                    "dispatch_path": "local_cylinder",
                    "status": "completed",
                })))
            }
            Err(e) => Some(Err(format!("local dispatch failed: {e}"))),
        }
    }
}
