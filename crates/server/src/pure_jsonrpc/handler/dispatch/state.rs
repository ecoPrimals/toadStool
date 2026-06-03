// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dispatch handler construction, configuration, and anchor-store lifecycle.

use super::DispatchHandler;
use crate::visualization_client::SharedVisualizationClient;
use std::collections::HashMap;
use std::os::fd::AsFd;
use std::sync::Arc;

impl DispatchHandler {
    pub fn new(
        coral_client: SharedVisualizationClient,
        crypto_client: Option<Arc<toadstool_distributed::crypto_integration::CryptoServiceClient>>,
    ) -> Self {
        let anchor_store: super::AnchorStore = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        Self {
            coral_client,
            crypto_client,
            cached_purpose_key: Arc::new(tokio::sync::RwLock::new(None)),
            jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            pipelines: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            dispatch_count: std::sync::atomic::AtomicU64::new(0),
            device_pool: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            local_device_factory: None,
            cached_devices: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            anchor_store,
            resource_orchestrator: None,
        }
    }

    /// Get a clone of the anchor store for use in the SIGTERM handler.
    pub fn anchor_store(&self) -> super::AnchorStore {
        Arc::clone(&self.anchor_store)
    }

    /// Set the local device factory for Phase D sovereign dispatch.
    ///
    /// When set, dispatch attempts local execution before falling back to
    /// coral_client IPC. The factory receives a BDF and returns a `ComputeDevice`
    /// if the device can be opened locally.
    pub fn set_local_device_factory(
        &mut self,
        factory: super::LocalDeviceFactory,
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
    pub(super) fn pre_dispatch_resource_check(
        &self,
        bdf: &str,
    ) -> Result<
        Option<toadstool_runtime_orchestration::ResourceAllocation>,
        crate::pure_jsonrpc::types::JsonRpcError,
    > {
        use crate::pure_jsonrpc::types::JsonRpcError;
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
            estimated_duration: std::time::Duration::from_mins(1),
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

    /// Dup VFIO fds from the anchor store into `ReceivedVfioFds` for
    /// device adoption. Returns `None` if no anchor exists for this BDF.
    pub(super) async fn dup_received_fds_from_anchor(
        &self,
        bdf: &str,
    ) -> Option<toadstool_cylinder::vfio::ReceivedVfioFds> {
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
    pub(super) async fn try_engage_clutch(
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
