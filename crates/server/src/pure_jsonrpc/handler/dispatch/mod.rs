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
mod fan_out;
mod forward;
mod pipeline;
mod queries;
mod routing;
mod shader_dispatch;
mod sovereign;
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
use toadstool_ember::vfio_handle::VfioResourceHandle;
use toadstool_ember::held_resource::HeldResource;
use types::{DispatchJob, PipelineJob};

/// Shared collection of VFIO warm-keepalive anchors.
///
/// Each anchor holds dup'd VFIO fds for a GPU. On SIGTERM, all anchors
/// are leaked to prevent bus resets during daemon restart.
pub type AnchorStore = Arc<tokio::sync::Mutex<HashMap<String, VfioAnchor>>>;

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

#[expect(
    deprecated,
    reason = "SecurityClient delegates to crypto.encrypt/decrypt; crypto_integration migration tracked"
)]
impl DispatchHandler {
    pub fn new(
        coral_client: SharedVisualizationClient,
        security_client: Option<Arc<toadstool_distributed::security::client::SecurityClient>>,
    ) -> Self {
        let anchor_store: AnchorStore = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        Self {
            coral_client,
            security_client,
            cached_purpose_key: Arc::new(RwLock::new(None)),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            pipelines: Arc::new(RwLock::new(HashMap::new())),
            dispatch_count: AtomicU64::new(0),
            device_pool: Arc::new(RwLock::new(HashMap::new())),
            local_device_factory: None,
            cached_devices: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            anchor_store,
            resource_orchestrator: None,
        }
    }

    /// Get a clone of the anchor store for use in the SIGTERM handler.
    pub fn anchor_store(&self) -> AnchorStore {
        Arc::clone(&self.anchor_store)
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

    pub fn set_resource_orchestrator(
        &mut self,
        orchestrator: Arc<toadstool_runtime_orchestration::ResourceOrchestrator>,
    ) {
        self.resource_orchestrator = Some(orchestrator);
    }

    /// Pre-dispatch resource check via the orchestrator (no-op when unset).
    ///
    /// Caller identity is always `"anonymous"` until BearDog JH-1 ships
    /// `CallerContext` through these entry points.
    fn pre_dispatch_resource_check(
        &self,
        bdf: &str,
    ) -> Result<
        Option<toadstool_runtime_orchestration::ResourceAllocation>,
        super::super::types::JsonRpcError,
    > {
        use super::super::types::JsonRpcError;
        use toadstool_common::constants::jsonrpc::error_codes;

        let Some(orchestrator) = self.resource_orchestrator.as_ref() else {
            return Ok(None);
        };

        let preferred_devices = toadstool_sysmon::discover_gpus()
            .into_iter()
            .find(|gpu| gpu.pci_slot == bdf)
            .map(|gpu| vec![gpu.card_index])
            .unwrap_or_default();

        let request = toadstool_runtime_orchestration::ResourceRequest {
            tenant_id: String::from("anonymous"),
            priority: 3,
            preferred_devices,
            min_vram_bytes: 0,
            estimated_duration: std::time::Duration::from_secs(60),
        };

        match orchestrator.allocate(&request) {
            Ok(allocation) => Ok(Some(allocation)),
            Err(toadstool_runtime_orchestration::OrchestrationError::GuestLoadExceeded(msg)) => {
                Err(JsonRpcError::server_error(
                    error_codes::CAPABILITY_NOT_AVAILABLE,
                    msg,
                ))
            }
            Err(toadstool_runtime_orchestration::OrchestrationError::QuotaExceeded(msg)) => {
                Err(JsonRpcError::server_error(
                    error_codes::RESOURCE_EXHAUSTED,
                    msg,
                ))
            }
            Err(err) => Err(JsonRpcError::internal_error(err.to_string())),
        }
    }

    /// Get a cached device or create one via the factory.
    ///
    /// VFIO devices hold iommufd FDs and DMA mappings. Dropping them
    /// can trigger a GPU reset, so we cache them across RPC calls.
    ///
    /// When the factory creates a device in "caps-only" mode (VFIO group
    /// held by ember → EBUSY), this method tries to adopt dup'd fds from
    /// the anchor store, bridging sovereign.init's warm state into the
    /// dispatch path.
    async fn get_or_create_device(
        &self,
        bdf: &str,
    ) -> Option<tokio::sync::MutexGuard<'_, HashMap<String, Box<dyn toadstool_cylinder::ComputeDevice>>>> {
        let factory = self.local_device_factory.as_ref()?;
        let mut cache = self.cached_devices.lock().await;
        if !cache.contains_key(bdf) {
            let mut device = factory(bdf)?;

            if let Some(anchor_fds) = device.dup_anchor_fds() {
                let anchor = match anchor_fds {
                    toadstool_cylinder::vfio::DupAnchorFds::Iommufd { device_fd, iommufd, ioas_id } => {
                        VfioAnchor::from_iommufd(bdf.to_string(), device_fd, iommufd, ioas_id)
                    }
                    toadstool_cylinder::vfio::DupAnchorFds::Legacy { device_fd, container, group } => {
                        VfioAnchor::from_legacy(bdf.to_string(), device_fd, container, group)
                    }
                };
                let mut anchors = self.anchor_store.lock().await;
                tracing::info!(bdf, "VfioAnchor created — warm keepalive active");
                anchors.insert(bdf.to_string(), anchor);
            } else if device.dma_backend().is_none() {
                // Device is in caps-only mode — VFIO group was EBUSY because
                // ember holds it via anchors. Try to adopt dup'd fds from the
                // anchor store to complete the VFIO session.
                if let Some(received) = self.dup_received_fds_from_anchor(bdf).await {
                    match device.adopt_anchor_fds(received) {
                        Ok(()) => {
                            tracing::info!(bdf, "VFIO session adopted from anchor fds — dispatch ready");
                        }
                        Err(e) => {
                            tracing::warn!(bdf, err = %e, "adopt_anchor_fds failed — caps-only mode persists");
                        }
                    }
                } else {
                    tracing::debug!(bdf, "no anchor fds available for adoption");
                }
            }

            cache.insert(bdf.to_string(), device);
            tracing::info!(bdf, "VFIO device cached for persistent dispatch");
        }
        Some(cache)
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
        let pool = self.device_pool.read().await;
        let held = pool.get(bdf)?;
        if !held.is_alive() {
            tracing::warn!(bdf, "local dispatch: device handle not alive");
            return None;
        }
        drop(pool);

        let mut cache = self.get_or_create_device(bdf).await?;
        let device = cache.get_mut(bdf)?;

        tracing::info!(bdf, binary_len = binary.len(), "Phase D: local dispatch via cylinder");

        let dims = toadstool_cylinder::DispatchDims::new(
            workgroup_size[0],
            workgroup_size[1],
            workgroup_size[2],
        );

        let info = if let Some(si) = shader_info {
            submit::resolve_shader_info(si, workgroup_size)
        } else {
            toadstool_cylinder::ShaderInfo {
                workgroup: workgroup_size,
                ..Default::default()
            }
        };

        Some(Self::run_local_lifecycle(&mut **device, binary, &dims, &info, buffer_descs))
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
    ///
    /// The opened device is cached persistently — VFIO iommufd FDs and DMA
    /// mappings survive across calls. Dropping them triggers GPU reset.
    pub(super) async fn device_vfio_open(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, super::super::types::JsonRpcError> {
        use super::super::types::JsonRpcError;

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        self.pre_dispatch_resource_check(bdf)?;
        self.acquire_device_handle(bdf).await;
        crate::background::pcie_keepalive::activity_tracker().record();

        match self.get_or_create_device(bdf).await {
            Some(cache) => {
                let device = cache
                    .get(bdf)
                    .ok_or_else(|| JsonRpcError::internal_error("device cache miss after insert"))?;
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

        self.pre_dispatch_resource_check(bdf)?;
        self.acquire_device_handle(bdf).await;
        crate::background::pcie_keepalive::activity_tracker().record();

        let mut cache = self.get_or_create_device(bdf).await.ok_or_else(|| {
            JsonRpcError::internal_error(format!(
                "VFIO device {bdf} not available — FECS cold or not VFIO-bound"
            ))
        })?;

        let device = cache.get_mut(bdf).ok_or_else(|| {
            JsonRpcError::internal_error(format!(
                "VFIO device {bdf} not in cache after creation"
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
            submit::resolve_shader_info(si, workgroup_size)
        } else {
            toadstool_cylinder::ShaderInfo {
                workgroup: workgroup_size,
                ..Default::default()
            }
        };

        if let Some(entries_arr) = p.get("gr_init_entries").and_then(serde_json::Value::as_array) {
            let method_entries: Vec<(u32, u32)> = entries_arr
                .iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    let reg = pair.first()?.as_u64()? as u32;
                    let val = pair.get(1)?.as_u64()? as u32;
                    Some((reg, val))
                })
                .collect();

            if !method_entries.is_empty()
                && let Err(e) = device.init_gr_context(&method_entries)
            {
                return Err(JsonRpcError::internal_error(format!(
                    "GR context init failed: {e}"
                )));
            }
        }

        let job_id = uuid::Uuid::new_v4().to_string();
        let start = std::time::Instant::now();

        self.dispatch_count.fetch_add(1, Ordering::Relaxed);

        match Self::run_local_lifecycle(&mut **device, &binary_bytes, &dims, &info, &buffer_descs) {
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

impl DispatchHandler {
    /// `device.gr.init` — submit GR context init method entries to a VFIO device.
    ///
    /// Accepts `(register, value)` pairs captured from warm-catch experiments and
    /// submits them as a GR context init pushbuffer. Required before first compute
    /// dispatch on warm-caught Volta+ GPUs (Kepler does not need this).
    pub(super) async fn device_gr_init(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, super::super::types::JsonRpcError> {
        use super::super::types::JsonRpcError;

        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params(
                "Expected { bdf, method_entries: [[register, value], ...] }",
            )
        })?;

        let bdf = p
            .get("bdf")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        let entries_arr = p
            .get("method_entries")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                JsonRpcError::invalid_params(
                    "Missing 'method_entries' array of [register, value] pairs",
                )
            })?;

        let method_entries: Vec<(u32, u32)> = entries_arr
            .iter()
            .filter_map(|entry| {
                let pair = entry.as_array()?;
                let reg = pair.first()?.as_u64()? as u32;
                let val = pair.get(1)?.as_u64()? as u32;
                Some((reg, val))
            })
            .collect();

        if method_entries.is_empty() {
            return Err(JsonRpcError::invalid_params(
                "method_entries must contain at least one [register, value] pair",
            ));
        }

        self.acquire_device_handle(bdf).await;
        crate::background::pcie_keepalive::activity_tracker().record();

        let mut cache = self.get_or_create_device(bdf).await.ok_or_else(|| {
            JsonRpcError::internal_error(format!(
                "VFIO device {bdf} not available — FECS cold or not VFIO-bound"
            ))
        })?;

        let device = cache.get_mut(bdf).ok_or_else(|| {
            JsonRpcError::internal_error(format!(
                "VFIO device {bdf} not in cache after creation"
            ))
        })?;

        let start = std::time::Instant::now();

        match device.init_gr_context(&method_entries) {
            Ok(()) => {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                Ok(serde_json::json!({
                    "domain": "device.gr",
                    "operation": "init",
                    "bdf": bdf,
                    "status": "completed",
                    "entries_submitted": method_entries.len(),
                    "timing": { "init_ms": elapsed_ms },
                }))
            }
            Err(e) => {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                Ok(serde_json::json!({
                    "domain": "device.gr",
                    "operation": "init",
                    "bdf": bdf,
                    "status": "failed",
                    "error": format!("{e}"),
                    "entries_submitted": 0,
                    "timing": { "init_ms": elapsed_ms },
                }))
            }
        }
    }

    /// Dup VFIO fds from the anchor store into `ReceivedVfioFds` for
    /// device adoption. Returns `None` if no anchor exists for this BDF.
    async fn dup_received_fds_from_anchor(
        &self,
        bdf: &str,
    ) -> Option<toadstool_cylinder::vfio::ReceivedVfioFds> {
        use std::os::fd::AsFd;
        let anchors = self.anchor_store.lock().await;
        let anchor = anchors.get(bdf)?;

        let device_fd = anchor.device_fd().try_clone_to_owned().ok()?;

        match anchor.backend_arc() {
            toadstool_ember::vfio_anchor::AnchorBackendRef::Iommufd { iommufd, ioas_id } => {
                let iommufd_dup = iommufd.as_fd().try_clone_to_owned().ok()?;
                Some(toadstool_cylinder::vfio::ReceivedVfioFds::Iommufd {
                    iommufd: iommufd_dup,
                    device: device_fd,
                    ioas_id,
                })
            }
            toadstool_ember::vfio_anchor::AnchorBackendRef::LegacyGroup { container } => {
                let container_dup = container.as_fd().try_clone_to_owned().ok()?;
                let group_fd = anchor.group_fd()?.try_clone_to_owned().ok()?;
                Some(toadstool_cylinder::vfio::ReceivedVfioFds::Legacy {
                    container: container_dup,
                    device: device_fd,
                    group: group_fd,
                })
            }
        }
    }

    /// Try to engage the clutch for a BDF from the anchor store.
    ///
    /// Uses `WarmKeepalive` to streamline fd extraction and DMA construction.
    async fn try_engage_clutch(
        &self,
        bdf: &str,
    ) -> Option<toadstool_cylinder::vfio::clutch::ClutchEngaged> {
        let anchors = self.anchor_store.lock().await;
        let anchor = anchors.get(bdf)?;
        let view = toadstool_ember::WarmKeepalive::from_ref(anchor);
        let dma = dma_from_keepalive(&view)?;

        match toadstool_cylinder::vfio::clutch::Clutch::engage(bdf, view.device_fd(), dma.clone()) {
            Ok(engaged) => {
                tracing::info!(bdf, "clutch engaged from keepalive (VFIO BAR0)");
                Some(engaged)
            }
            Err(e) => {
                tracing::warn!(bdf, err = %e, "clutch VFIO engage failed — trying sysfs BAR0");
                toadstool_cylinder::vfio::clutch::Clutch::engage_sysfs(bdf, dma).ok()
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

    // Bypass ember gate for server-internal probes — toadstool IS ember,
    // so the gate would deadlock or reject our own BAR0 access.
    let _gate_bypass = toadstool_cylinder::vfio::ember_gate::EmberGateBypass::enter();

    let mut dev = toadstool_cylinder::nv::compute_device::NvVfioComputeDevice::new(bdf.to_string());

    let warm_fecs = dev.probe_warm_fecs();

    // Check if this is a Kepler (NoAcr) device that doesn't need warm FECS.
    let device_name = dev.capabilities().device_name.to_owned();
    let is_kepler = device_name.contains("Kepler")
        || device_name.contains("gk210")
        || device_name.contains("gk110");

    // Check if this GPU can cold boot via PIO firmware upload from
    // /lib/firmware/nvidia/{chip}/gr/. This covers Volta (pre-GSP) on
    // systems where open nvidia.ko refuses to bind and nouveau is absent.
    let can_pio_cold_boot = if !warm_fecs && !is_kepler {
        let sm = dev.sm_version();
        if sm > 0 {
            let profile = toadstool_cylinder::nv::generation::profile_for_sm(sm);
            let bridge = toadstool_cylinder::nv::nv_gsp_bridge::NvGspBridge::new(
                profile.firmware_chip,
            );
            bridge.has_gr_firmware()
        } else {
            false
        }
    } else {
        false
    };

    if warm_fecs || is_kepler || can_pio_cold_boot {
        if (is_kepler || can_pio_cold_boot) && !warm_fecs {
            dev.set_fecs_ready(true);
            tracing::info!(
                bdf,
                device = %device_name,
                pio_cold_boot = can_pio_cold_boot,
                "FECS boots via PIO from /lib/firmware — marking compute-ready"
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
/// Convert a `WarmKeepaliveRef`'s DMA spec into cylinder's `DmaBackend`.
fn dma_from_keepalive(
    view: &toadstool_ember::warm_keepalive::WarmKeepaliveRef<'_>,
) -> Option<toadstool_cylinder::vfio::DmaBackend> {
    let spec = view.make_dma_backend();
    if let Some((iommufd, ioas_id)) = spec.as_iommufd() {
        Some(toadstool_cylinder::vfio::clutch::Clutch::dma_backend_from_iommufd(iommufd, ioas_id))
    } else if let Some(container) = spec.as_legacy_container() {
        Some(toadstool_cylinder::vfio::clutch::Clutch::dma_backend_from_legacy(container))
    } else {
        tracing::error!("DmaSpec is neither iommufd nor legacy — cannot create DMA backend");
        None
    }
}

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
