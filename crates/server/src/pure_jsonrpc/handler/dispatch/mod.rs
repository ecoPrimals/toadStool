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
    pub fn set_local_device_factory(
        &mut self,
        factory: LocalDeviceFactory,
    ) {
        self.local_device_factory = Some(factory);
    }

    /// Attempt local dispatch through cylinder's `ComputeDevice`.
    ///
    /// Full lifecycle: alloc → upload → dispatch → sync → readback.
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
        buffer_descs: &serde_json::Value,
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

        Some(Self::run_local_lifecycle(&mut *device, binary, &dims, &info, buffer_descs))
    }
}

struct BufMeta {
    handle: toadstool_cylinder::BufferHandle,
    size: u64,
    readback: bool,
}

impl DispatchHandler {
    /// Full alloc → upload → dispatch → sync → readback lifecycle on a local device.
    fn run_local_lifecycle(
        device: &mut dyn toadstool_cylinder::ComputeDevice,
        binary: &[u8],
        dims: &toadstool_cylinder::DispatchDims,
        info: &toadstool_cylinder::ShaderInfo,
        buffer_descs: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let buf_arr = buffer_descs.as_array();
        let mut handles: Vec<toadstool_cylinder::BufferHandle> = Vec::new();
        let mut metas: Vec<BufMeta> = Vec::new();

        if let Some(descs) = buf_arr {
            for desc in descs {
                let size = desc
                    .get("size")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                if size == 0 {
                    continue;
                }

                let direction = desc
                    .get("direction")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("inout");

                let domain = match desc
                    .get("domain")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("vram")
                {
                    "gtt" => toadstool_cylinder::MemoryDomain::Gtt,
                    "vram_or_gtt" => toadstool_cylinder::MemoryDomain::VramOrGtt,
                    _ => toadstool_cylinder::MemoryDomain::Vram,
                };

                let handle = device
                    .alloc(size, domain)
                    .map_err(|e| format!("buffer alloc ({size} bytes): {e}"))?;

                if matches!(direction, "in" | "inout")
                    && let Some(data) = desc.get("data").and_then(serde_json::Value::as_array)
                {
                    let bytes: Vec<u8> =
                        data.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect();
                    device
                        .upload(handle, 0, &bytes)
                        .map_err(|e| format!("buffer upload: {e}"))?;
                }

                let needs_readback = matches!(direction, "out" | "inout");
                metas.push(BufMeta {
                    handle,
                    size,
                    readback: needs_readback,
                });
                handles.push(handle);
            }
        }

        device
            .dispatch(binary, &handles, *dims, info)
            .map_err(|e| format!("local dispatch failed: {e}"))?;

        device
            .sync()
            .map_err(|e| format!("local dispatch sync failed: {e}"))?;

        let readback_start = std::time::Instant::now();
        let mut readback_results: Vec<serde_json::Value> = Vec::new();
        for meta in &metas {
            if meta.readback {
                match device.readback(meta.handle, 0, meta.size as usize) {
                    Ok(data) => {
                        readback_results.push(serde_json::json!({
                            "size": meta.size,
                            "data_b64": base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                &data,
                            ),
                        }));
                    }
                    Err(e) => {
                        readback_results.push(serde_json::json!({
                            "size": meta.size,
                            "error": format!("{e}"),
                        }));
                    }
                }
            }
        }
        let readback_ms = readback_start.elapsed().as_millis() as u64;

        for meta in &metas {
            let _ = device.free(meta.handle);
        }

        Ok(serde_json::json!({
            "dispatch_path": "local_cylinder",
            "status": "completed",
            "buffers": readback_results,
            "readback_ms": readback_ms,
        }))
    }
}

impl DispatchHandler {
    /// `device.vfio.open` — open a VFIO device by BDF, return capabilities and status.
    pub(super) async fn device_vfio_open(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, super::super::types::JsonRpcError> {
        use super::super::types::JsonRpcError;

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        let factory = self.local_device_factory.as_ref().ok_or_else(|| {
            JsonRpcError::internal_error("no local device factory configured")
        })?;

        self.acquire_device_handle(bdf).await;

        match factory(bdf) {
            Some(device) => {
                let caps = device.capabilities();
                Ok(serde_json::json!({
                    "domain": "device.vfio",
                    "operation": "open",
                    "bdf": bdf,
                    "status": "ready",
                    "capabilities": {
                        "vendor": format!("{:?}", caps.vendor),
                        "device_name": caps.device_name,
                        "generation": caps.generation_name,
                        "has_f64": caps.has_hardware_f64,
                        "max_shared_mem_bytes": caps.max_shared_mem_bytes,
                    },
                }))
            }
            None => Ok(serde_json::json!({
                "domain": "device.vfio",
                "operation": "open",
                "bdf": bdf,
                "status": "unavailable",
                "error": "device not available — FECS cold or not VFIO-bound",
            })),
        }
    }

    /// `device.vfio.roundtrip` — alloc→upload→dispatch→sync→readback in one call.
    ///
    /// Convenience endpoint for springs that want a single RPC for the full
    /// compute lifecycle on a VFIO device. Returns a `job_id` and inline results.
    pub(super) async fn device_vfio_roundtrip(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, super::super::types::JsonRpcError> {
        use super::super::types::JsonRpcError;
        use std::sync::atomic::Ordering;

        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params(
                "Expected { bdf, binary_b64|binary, workgroup_size?, buffers?, shader_info? }",
            )
        })?;

        let bdf = p
            .get("bdf")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        let binary_bytes = submit::resolve_binary_param(p)?;
        if binary_bytes.is_empty() {
            return Err(JsonRpcError::invalid_params("binary must not be empty"));
        }

        let factory = self.local_device_factory.as_ref().ok_or_else(|| {
            JsonRpcError::internal_error("no local device factory configured")
        })?;

        self.acquire_device_handle(bdf).await;

        let mut device = factory(bdf).ok_or_else(|| {
            JsonRpcError::internal_error(format!(
                "VFIO device {bdf} not available — FECS cold or not VFIO-bound"
            ))
        })?;

        let workgroup_size = submit::resolve_workgroup_size(p);
        let buffer_descs = submit::resolve_buffers(p);
        let shader_info = p.get("shader_info").cloned();

        let dims = toadstool_cylinder::DispatchDims::new(
            workgroup_size[0],
            workgroup_size[1],
            workgroup_size[2],
        );

        let info = if let Some(ref si) = shader_info {
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

        let job_id = uuid::Uuid::new_v4().to_string();
        let start = std::time::Instant::now();

        self.dispatch_count.fetch_add(1, Ordering::Relaxed);

        match Self::run_local_lifecycle(&mut *device, &binary_bytes, &dims, &info, &buffer_descs) {
            Ok(output) => {
                let dispatch_ms = start.elapsed().as_millis() as u64;
                Ok(serde_json::json!({
                    "domain": "device.vfio",
                    "operation": "roundtrip",
                    "job_id": job_id,
                    "bdf": bdf,
                    "status": "completed",
                    "output": output,
                    "timing": { "dispatch_ms": dispatch_ms },
                }))
            }
            Err(e) => {
                let dispatch_ms = start.elapsed().as_millis() as u64;
                Ok(serde_json::json!({
                    "domain": "device.vfio",
                    "operation": "roundtrip",
                    "job_id": job_id,
                    "bdf": bdf,
                    "status": "failed",
                    "error": e,
                    "timing": { "dispatch_ms": dispatch_ms },
                }))
            }
        }
    }
}

/// Create a local device factory for Phase D sovereign dispatch.
///
/// The factory resolves a PCI BDF to a local `ComputeDevice` via two paths:
/// 1. **DRM path** — sysfs render node → driver probe → `AmdDevice` (amdgpu)
/// 2. **VFIO path** — no render node → check vfio-pci binding → BAR0 warm
///    FECS detection → `NvVfioComputeDevice` (NVIDIA, warm-handoff only)
#[cfg(target_os = "linux")]
pub(super) fn create_cylinder_device_factory() -> LocalDeviceFactory {
    Arc::new(|bdf: &str| -> Option<Box<dyn toadstool_cylinder::ComputeDevice>> {
        // Path 1: DRM render node available → kernel driver active
        if let Some(render_path) = resolve_render_node(bdf)
            && let Ok(drm_dev) = toadstool_cylinder::drm::DrmDevice::open(&render_path)
            && let Ok(driver) = drm_dev.driver_name()
        {
            drop(drm_dev);
            match driver.as_str() {
                "amdgpu" => {
                    return match toadstool_cylinder::amd::AmdDevice::open_path(&render_path) {
                        Ok(dev) => {
                            tracing::info!(bdf, render = %render_path, "Phase D: opened AMD compute device");
                            Some(Box::new(dev))
                        }
                        Err(e) => {
                            tracing::warn!(bdf, render = %render_path, error = %e, "AMD device open failed");
                            None
                        }
                    };
                }
                "nouveau" => {
                    tracing::debug!(bdf, "NVIDIA on nouveau — not available for VFIO dispatch while kernel driver is bound");
                    return None;
                }
                other => {
                    tracing::debug!(bdf, driver = other, "no local ComputeDevice impl for this driver");
                    return None;
                }
            }
        }

        // Path 2: No DRM render node — check for VFIO-bound NVIDIA GPU
        try_vfio_nvidia(bdf)
    })
}

/// Attempt to open an NVIDIA GPU via VFIO with warm FECS detection.
///
/// When a GPU is bound to `vfio-pci`, it has no DRM render node. This
/// probes BAR0 via sysfs for chip identity and warm-preserved FECS state
/// from a prior nouveau/nvidia-470 session.
///
/// Kepler GPUs (SM 35–37) use `BootStrategy::NoAcr` and don't need a warm
/// FECS handoff — they boot FECS directly via PIO. For these devices,
/// `probe_capabilities` identifies the chip and `open_vfio` creates a
/// Kepler-specific PFIFO channel with GK104 doorbell.
#[cfg(target_os = "linux")]
fn try_vfio_nvidia(bdf: &str) -> Option<Box<dyn toadstool_cylinder::ComputeDevice>> {
    use toadstool_cylinder::ComputeDevice as _;

    let driver_link = format!("/sys/bus/pci/devices/{bdf}/driver");
    let driver_target = std::fs::read_link(&driver_link).ok()?;
    let driver_name = driver_target.file_name()?.to_str()?;

    if driver_name != "vfio-pci" {
        tracing::debug!(bdf, driver = driver_name, "not vfio-pci — skipping VFIO path");
        return None;
    }

    tracing::info!(bdf, "VFIO-bound device detected — probing for identity and FECS");

    let mut dev = toadstool_cylinder::nv::compute_device::NvVfioComputeDevice::new(bdf.to_string());

    // Probe BOOT0 for chip identity and warm FECS state.
    let warm_fecs = dev.probe_warm_fecs();

    // Check if this is a Kepler (NoAcr) device that doesn't need warm FECS.
    let device_name = dev.capabilities().device_name.to_owned();
    let is_kepler = device_name.contains("Kepler")
        || device_name.contains("gk210")
        || device_name.contains("gk110");

    if warm_fecs || is_kepler {
        if is_kepler && !warm_fecs {
            dev.set_fecs_ready(true);
            tracing::info!(
                bdf,
                device = %device_name,
                "Kepler NoAcr: FECS boots via PIO — marking compute-ready"
            );
        }

        match dev.open_vfio() {
            Ok(()) => {
                tracing::info!(bdf, "Phase D: NVIDIA VFIO device opened — PBDMA dispatch ready");
            }
            Err(e) => {
                tracing::warn!(bdf, error = %e, "VFIO device open failed — caps-only mode");
            }
        }
        Some(Box::new(dev))
    } else {
        tracing::info!(
            bdf,
            "NVIDIA VFIO device detected but FECS cold — waiting for firmware bridge"
        );
        None
    }
}

#[cfg(not(target_os = "linux"))]
pub(super) fn create_cylinder_device_factory() -> Option<LocalDeviceFactory> {
    None
}

/// Resolve a PCI BDF to its DRM render node path via sysfs.
///
/// Reads `/sys/bus/pci/devices/{bdf}/drm/` for `renderD*` entries.
#[cfg(target_os = "linux")]
fn resolve_render_node(bdf: &str) -> Option<String> {
    let drm_dir = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "drm");
    let entries = std::fs::read_dir(drm_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("renderD") {
            return Some(format!("/dev/dri/{name_str}"));
        }
    }
    None
}
