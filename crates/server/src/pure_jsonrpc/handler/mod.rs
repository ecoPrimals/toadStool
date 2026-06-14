// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC request handler and method router
//!
//! Routes JSON-RPC 2.0 requests to the appropriate executor or job queue.
//! Semantic method names are resolved through `SemanticMethodRegistry`
//! before dispatch, enabling both legacy `toadstool.*` names and the
//! standard `{domain}.{operation}` naming convention.

mod auth;
mod core;
mod dispatch;
mod ember;
mod hw_learn;
mod job;
pub mod method_gate;
mod mmio;
mod resources;
mod silicon;
mod sovereign;
mod transport;
mod workload;

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use toadstool::semantic_methods::SemanticMethodRegistry;
use toadstool_common::interned_strings::socket_env;
use tracing::{debug, error, info};

pub use method_gate::{
    CallerContext, ConnectionTrustHints, ConnectionTransport, DispatchTrustLevel,
};

use super::types::{JSONRPC_VERSION, JsonRpcError, JsonRpcRequest, JsonRpcResponse};

use dispatch::DispatchHandler;
pub use hw_learn::HwLearnHandler;
use job::JobHandler;
use resources::ResourceHandler;
use silicon::SiliconHandler;
use transport::TransportHandler;
use workload::WorkloadHandler;

use crate::glowplug_client::{self, SharedGlowPlugClient};

/// Pure Rust JSON-RPC Handler
///
/// Thin coordinator that delegates to specialized handlers.
/// Routes requests to appropriate methods. Supports both legacy `toadstool.*`
/// names and semantic `{domain}.{operation}` names via the registry.
pub struct JsonRpcHandler {
    version: Arc<str>,
    start_time: std::time::Instant,
    error_count: Arc<AtomicU64>,
    /// PG-62: set to `true` once the server is fully initialized.
    /// `health.readiness` returns `"starting"` until this is set.
    pub(super) ready: Arc<AtomicBool>,
    /// Drain flag: set by `health.drain` to reject new dispatches.
    pub(super) draining: Arc<AtomicBool>,
    /// JH-0 pre-dispatch capability gate. Ships permissive (all calls allowed).
    gate: method_gate::MethodGate,
    semantic_registry: SemanticMethodRegistry,
    pub(super) dispatch: DispatchHandler,
    /// Shared anchor store for warm keepalive — leaked on SIGTERM.
    anchor_store: dispatch::AnchorStore,
    hw_learn: HwLearnHandler,
    job: JobHandler,
    workload: WorkloadHandler,
    resources: ResourceHandler,
    transport: TransportHandler,
    silicon: SiliconHandler,
    pub(super) glowplug: SharedGlowPlugClient,
    /// Actual bound JSON-RPC UDS path (set at server startup).
    bound_socket_path: Option<Arc<PathBuf>>,
}

impl JsonRpcHandler {
    /// Create new handler with executor.
    ///
    /// Pass `error_count` to share the counter with other servers for unified monitoring.
    /// Pass `ready` to share the readiness flag — `health.readiness` returns `"starting"`
    /// until this flag is set to `true` (PG-62).
    pub fn new(
        executor: Arc<crate::tarpc_server::WorkloadExecutorDispatch>,
        version: impl Into<Arc<str>>,
        error_count: Option<Arc<AtomicU64>>,
        ready: Arc<AtomicBool>,
        bound_socket_path: Option<Arc<PathBuf>>,
    ) -> Self {
        let local_gate_id = std::env::var(socket_env::TOADSTOOL_GATE_ID)
            .or_else(|_| std::env::var(socket_env::HOSTNAME))
            .or_else(|_| toadstool_sysmon::system::hostname().ok_or(std::env::VarError::NotPresent))
            .unwrap_or_else(|_| String::from("local"));
        let gate_ownership = Arc::new(crate::cross_gate::GateOwnership::new(&local_gate_id));

        let gate = match std::env::var(socket_env::TOADSTOOL_AUTH_MODE)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "enforcing" | "enforced" => {
                info!("MethodGate mode: enforcing (via TOADSTOOL_AUTH_MODE)");
                method_gate::MethodGate::new(method_gate::GateMode::Enforcing)
            }
            _ => method_gate::MethodGate::permissive(),
        };

        let coral_client = crate::visualization_client::create_visualization_client();

        // Spawn ipc.watch background poller — watches songBird for shader
        // capability registrations and invalidates the visualization client
        // cache so dispatch can discover coralReef at any time (GAP-HS-119).
        {
            let watch_client = Arc::clone(&coral_client);
            tokio::spawn(async move {
                crate::background::ipc_watch::run(watch_client).await;
            });
        }

        let mut dispatch = DispatchHandler::new(
            coral_client,
            Self::try_connect_crypto_client(),
        );
        #[cfg(target_os = "linux")]
        {
            tracing::info!("Phase D: local cylinder device factory registered");
            dispatch.set_local_device_factory(dispatch::create_cylinder_device_factory());
        }
        #[cfg(not(target_os = "linux"))]
        if let Some(factory) = dispatch::create_cylinder_device_factory() {
            dispatch.set_local_device_factory(factory);
        }

        // Build resource orchestrator for multi-tenant GPU scheduling.
        // Deployment model from TOADSTOOL_DEPLOYMENT_MODEL env:
        //   "multi" → LocalMulti, "rental" → CloudRental, else LocalDirect (no enforcement)
        let deployment_model = match std::env::var(socket_env::TOADSTOOL_DEPLOYMENT_MODEL).as_deref() {
            Ok("multi") => toadstool_runtime_orchestration::DeploymentModel::LocalMulti,
            Ok("rental") => toadstool_runtime_orchestration::DeploymentModel::CloudRental,
            _ => toadstool_runtime_orchestration::DeploymentModel::LocalDirect,
        };

        if deployment_model != toadstool_runtime_orchestration::DeploymentModel::LocalDirect {
            let gpus = toadstool_sysmon::discover_gpus();
            let devices: Vec<toadstool_runtime_orchestration::AvailableDevice> = gpus
                .iter()
                .map(|gpu| toadstool_runtime_orchestration::AvailableDevice {
                    index: gpu.card_index,
                    total_vram_bytes: gpu.telemetry().vram_total_bytes.unwrap_or(0),
                    allocated_vram_bytes: 0,
                    current_tenant: None,
                })
                .collect();
            let orchestrator = toadstool_runtime_orchestration::ResourceOrchestrator::new(
                deployment_model,
                devices,
            );
            dispatch.set_resource_orchestrator(Arc::new(orchestrator));
            tracing::info!(
                model = ?deployment_model,
                gpu_count = gpus.len(),
                "Resource orchestrator initialized for multi-tenant dispatch"
            );
        }

        dispatch.set_gate_ownership(Arc::clone(&gate_ownership));

        let anchor_store = dispatch.anchor_store();

        Self {
            version: version.into(),
            start_time: std::time::Instant::now(),
            error_count: error_count.unwrap_or_else(|| Arc::new(AtomicU64::new(0))),
            ready,
            draining: Arc::new(AtomicBool::new(false)),
            gate,
            semantic_registry: SemanticMethodRegistry::new(),
            dispatch,
            anchor_store,
            hw_learn: HwLearnHandler::new(),
            job: JobHandler::new(Arc::clone(&gate_ownership)),
            workload: WorkloadHandler::new(executor),
            resources: ResourceHandler::new(),
            transport: TransportHandler::new(),
            silicon: SiliconHandler::new(),
            glowplug: glowplug_client::create_glowplug_client(),
            bound_socket_path,
        }
    }

    /// Get the anchor store for wiring into the SIGTERM leak handler.
    pub fn anchor_store(&self) -> dispatch::AnchorStore {
        self.anchor_store.clone()
    }

    /// Attempt to connect to the Tower crypto client (BearDog) for crypto
    /// delegation. Returns `None` in standalone mode (no crypto capability socket).
    fn try_connect_crypto_client(
    ) -> Option<Arc<toadstool_distributed::crypto_integration::CryptoServiceClient>> {
        let socket = toadstool_common::primal_sockets::get_socket_path_for_capability("crypto");
        if socket.exists() {
            match toadstool_distributed::crypto_integration::CryptoServiceClient::from_local_socket(
                &socket,
            ) {
                Ok(client) => {
                    info!("Tower crypto client connected — compute payloads will be encrypted");
                    Some(Arc::new(client))
                }
                Err(e) => {
                    debug!(
                        "Tower crypto client unavailable: {e} — standalone mode (plaintext dispatch)"
                    );
                    None
                }
            }
        } else {
            debug!(
                path = %socket.display(),
                "crypto socket absent — standalone mode (plaintext dispatch)"
            );
            None
        }
    }

    /// Handle a JSON-RPC request (main entry point).
    ///
    /// Pattern: parse → validate → resolve → route → execute → respond
    pub async fn handle_request(&self, request: &JsonRpcRequest<'_>) -> JsonRpcResponse {
        self.handle_request_with_connection(request, ConnectionTrustHints::default())
            .await
    }

    /// Handle a JSON-RPC request with per-connection trust hints.
    pub async fn handle_request_with_connection(
        &self,
        request: &JsonRpcRequest<'_>,
        conn: ConnectionTrustHints,
    ) -> JsonRpcResponse {
        if request.jsonrpc != JSONRPC_VERSION {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return JsonRpcResponse {
                jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                result: None,
                error: Some(JsonRpcError::invalid_request(
                    "Invalid JSON-RPC version (must be '2.0')",
                )),
                id: request.id.clone().unwrap_or(serde_json::Value::Null),
            };
        }

        let id = request.id.clone().unwrap_or(serde_json::Value::Null);

        info!(method = %request.method.as_ref(), "JSON-RPC request");

        match self
            .handle_method(request.method.as_ref(), request.params.as_ref(), conn)
            .await
        {
            Ok(result) => JsonRpcResponse {
                jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                result: Some(result),
                error: None,
                id,
            },
            Err(err) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                error!(
                    method = %request.method.as_ref(),
                    error = %err.message,
                    "JSON-RPC error",
                );
                JsonRpcResponse {
                    jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                    result: None,
                    error: Some(err),
                    id,
                }
            }
        }
    }

    /// Route a method name to its handler.
    ///
    /// Resolution order:
    /// 0. Pre-dispatch gate check (JH-0/JH-2: permissive default, enforcing future).
    /// 1. Direct literal match (backward-compatible `toadstool.*` and `compute.*` names).
    /// 2. Semantic registry lookup: `{domain}.{operation}` → implementation name → handler.
    async fn handle_method(
        &self,
        method: &str,
        params: Option<&serde_json::Value>,
        conn: ConnectionTrustHints,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let caller_ctx = extract_caller_context(conn);

        // JH-0/JH-2: pre-dispatch capability gate with caller context
        self.gate.check_with_context(method, &caller_ctx)?;

        match method {
            // Auth introspection — always public per JH-0 standard
            "auth.check" => return auth::auth_check(&self.gate, params),
            "auth.mode" => return auth::auth_mode(&self.gate),
            "auth.peer_info" => return auth::auth_peer_info(&caller_ctx),

            "toadstool.submit_workload" => return self.workload.submit_workload(params).await,
            "toadstool.query_status" => return self.job.query_status(params).await,
            "toadstool.cancel_workload" => return self.workload.cancel_workload(params).await,
            "toadstool.list_workloads" => return self.job.list_workloads(params).await,
            "toadstool.validate" => return self.workload.validate(params).await,
            "toadstool.query_capabilities" => return self.workload.query_capabilities().await,
            // GuideStone-mandated bare health probe (Wave 113): {status, primal, version}
            "health" => {
                return core::health_simple(&self.version).await;
            }
            // Wire Standard L1/L2: triad shapes differ; full payload only for check + legacy name.
            "toadstool.health" | "health.check" => {
                return core::health(&self.version, self.start_time, &self.error_count).await;
            }
            "health.liveness" => {
                return core::health_liveness().await;
            }
            "health.readiness" => {
                return core::health_readiness(
                    self.version.as_ref(),
                    self.ready.load(Ordering::Relaxed),
                )
                .await;
            }
            "health.version" => {
                return core::health_version(self.version.as_ref()).await;
            }
            "health.drain" => {
                return core::health_drain(&self.draining, &self.ready).await;
            }
            "identity.get" => {
                return core::identity_get(&self.version, &self.semantic_registry).await;
            }
            "toadstool.version" => return core::version_info(&self.version).await,

            "toadstool.resources.estimate"
            | "toadstool.ai.local_inference"
            | "resources.estimate"
            | "ai.local_inference" => return self.resources.resources_estimate(params).await,
            "toadstool.resources.validate_availability"
            | "toadstool.ai.local_execute"
            | "resources.validate_availability"
            | "ai.local_execute" => {
                return self.resources.resources_validate_availability(params).await;
            }
            "toadstool.resources.suggest_optimizations" | "resources.suggest_optimizations" => {
                return self.resources.resources_suggest_optimizations(params).await;
            }

            "compute.health" => {
                return core::health(&self.version, self.start_time, &self.error_count).await;
            }
            "compute.version" => return core::version_info(&self.version).await,
            "capabilities.list" | "capability.list" | "primal.capabilities" => {
                return core::capabilities_list(&self.semantic_registry, &self.version).await;
            }
            "primal.announce" => {
                return core::primal_announce(
                    &self.version,
                    &self.semantic_registry,
                    self.bound_socket_path.as_deref().map(PathBuf::as_path),
                )
                .await;
            }
            "compute.capabilities" => return self.workload.query_capabilities().await,
            "compute.discover_capabilities" => {
                return core::discover_capabilities(&self.semantic_registry, &self.version).await;
            }

            "compute.execute" => return self.workload.submit_workload(params).await,
            "compute.submit" => return self.job.compute_submit(params).await,
            "compute.status" => return self.job.compute_status(params).await,
            "compute.result" => return self.job.compute_result(params).await,
            "compute.cancel" => return self.job.compute_cancel(params).await,
            "compute.list" => return self.job.compute_list(params).await,

            "compute.dispatch" => {
                return self
                    .dispatch
                    .dispatch_submit_with_context(params, &caller_ctx)
                    .await;
            }
            "compute.dispatch.submit" => {
                return self
                    .dispatch
                    .dispatch_submit_with_context(params, &caller_ctx)
                    .await;
            }
            "compute.fan_out" => {
                return self.dispatch.fan_out(params, &caller_ctx).await;
            }
            "compute.dispatch.status" => return self.dispatch.dispatch_status(params).await,
            "compute.dispatch.result" => return self.dispatch.dispatch_result(params).await,
            "compute.dispatch.forward" => return self.dispatch.dispatch_forward(params).await,
            "dispatch.verify_trust" => {
                return Ok(dispatch::trust::verify_trust(&caller_ctx, params));
            }
            "compute.dispatch.capabilities" => {
                return self.dispatch.dispatch_capabilities(params).await;
            }
            "compute.dispatch.pipeline.submit" => {
                return self
                    .dispatch
                    .pipeline_submit_with_context(params, &caller_ctx)
                    .await;
            }
            "compute.dispatch.pipeline.status" => {
                return self.dispatch.pipeline_status(params).await;
            }
            "dispatch.telemetry.schema" => {
                return Ok(dispatch::telemetry::telemetry_schema());
            }

            "gpu.query_info" | "gpu.info" => return core::gpu_info().await,
            "gpu.query_memory" | "gpu.memory" => return core::gpu_memory().await,
            "gpu.query_telemetry" | "gpu.telemetry" => {
                return self.hw_learn.gpu_telemetry(params).await;
            }

            "gate.update" => return self.job.gate_update(params).await,
            "gate.remove" => return self.job.gate_remove(params).await,
            "gate.list" => return self.job.gate_list().await,
            "gate.route" => return self.job.gate_route(params).await,

            "transport.discover" => return Ok(TransportHandler::transport_discover(params)),
            "transport.list" => return self.transport.transport_list().await,
            "transport.route" => return self.transport.transport_route(params).await,
            "transport.open" => return self.transport.transport_open(params).await,
            "transport.stream" => return self.transport.transport_stream(params).await,
            "transport.status" => return self.transport.transport_status(params).await,

            // Hardware learning domain — biomeOS v2.30 compute.hardware.* capabilities
            "compute.hardware.observe" => return self.hw_learn.hw_learn_observe(params).await,
            "compute.hardware.distill" => return self.hw_learn.hw_learn_distill(params).await,
            "compute.hardware.apply" => return self.hw_learn.hw_learn_apply(params).await,
            "compute.hardware.share_recipe" => {
                return self.hw_learn.hw_learn_share_recipe(params).await;
            }
            "compute.hardware.auto_init" => return self.hw_learn.hw_learn_auto_init(params).await,
            "compute.hardware.auto_init_all" => {
                return self.hw_learn.hw_learn_auto_init_all(params).await;
            }
            "compute.hardware.status" => return self.hw_learn.hw_learn_status(params).await,
            "compute.hardware.vfio_devices" => {
                return self.hw_learn.hw_learn_vfio_devices(params).await;
            }

            "shader.dispatch" => {
                return self
                    .dispatch
                    .shader_dispatch_with_context(params, &caller_ctx)
                    .await;
            }

            "ember.list" => return Ok(self.ember_list()),
            "ember.status" => return Ok(self.ember_status()),
            "ember.reacquire" => return self.ember_reacquire(params).await,
            "ember.warm_cycle" => return self.ember_warm_cycle(params).await,
            "ember.prepare_dma" => return self.dispatch.ember_prepare_dma(params).await,
            "ember.cleanup_dma" => return self.dispatch.ember_cleanup_dma(params).await,
            "ember.adopt_device" => return self.ember_adopt_device(params).await,
            "device.swap" => return self.device_swap(params).await,
            "device.warm_catch" => return self.device_warm_catch(params),
            "device.get" => return self.device_get(params),
            "device.experiment_lifecycle" => return self.device_experiment_lifecycle(params),
            "device.reset" => return self.device_reset(params),
            "device.resurrect" => return self.device_resurrect(params).await,
            "device.health" => return mmio::ember_device_health(params),
            "device.vfio.open" => {
                return self.dispatch.device_vfio_open(params, &caller_ctx).await;
            }
            "device.vfio.roundtrip" => {
                return self
                    .dispatch
                    .device_vfio_roundtrip(params, &caller_ctx)
                    .await;
            }
            "device.gr.init" | "compute.context.init" => {
                return self.dispatch.device_gr_init(params).await;
            }

            "sovereign.init" => {
                // Always route through the ember handler — it has access to
                // the cached device's DMA backend.  The stateless handler
                // can't acquire DMA when the factory already holds the VFIO
                // group, which causes acr_no_dma failures.
                return self.dispatch.sovereign_init_ember(params).await;
            }
            "sovereign.boot" => return self.dispatch.sovereign_init_ember(params).await,
            "sovereign.profile" => {
                return self.dispatch.sovereign_profile_ember(params).await;
            }
            "sovereign.warm_status" => {
                return self.dispatch.sovereign_warm_status().await;
            }
            "sovereign.defense_status" => {
                return Ok(crate::background::catalyst_watchdog::defense_status());
            }
            "sovereign.watchdog_status" => {
                return Ok(crate::background::catalyst_watchdog::watchdog_status());
            }
            "sovereign.ce_validate" | "ce.validate" => {
                return self.dispatch.sovereign_ce_validate_ember(params).await;
            }
            "sovereign.pmu_investigate" | "pmu.investigate" => {
                return self.dispatch.sovereign_pmu_investigate(params).await;
            }
            "sovereign.warm_handoff" => {
                return self.dispatch.sovereign_warm_handoff(params).await;
            }
            "sovereign.catalyst_boot" => {
                return self.dispatch.sovereign_catalyst_boot(params).await;
            }
            "sovereign.classify_tier" => return sovereign::sovereign_classify_tier(params),
            "sovereign.experiment" => return sovereign::sovereign_experiment(params),
            "sovereign.devinit" => return sovereign::sovereign_devinit(params),
            "sovereign.kernel_health" => return sovereign::sovereign_kernel_health(params),
            "sovereign.snapshot" => return sovereign::sovereign_snapshot(params),
            "sovereign.compare" => return sovereign::sovereign_compare(params),
            "sovereign.catalyst_diff" => return sovereign::sovereign_catalyst_diff(params),
            "sovereign.reagent_capture" => return sovereign::sovereign_reagent_capture(params),
            "sovereign.recipe_replay" => return sovereign::sovereign_recipe_replay(params),
            "sovereign.runtime_services_probe" => {
                return sovereign::sovereign_runtime_services_probe(params);
            }

            "mmio.read32" => return mmio::mmio_read32(params),
            "mmio.write32" => return mmio::mmio_write32(params),
            "mmio.batch" => return mmio::mmio_batch(params),
            "mmio.pramin.read32" => return mmio::mmio_pramin_read32(params),
            "mmio.bar0.probe" => return mmio::mmio_bar0_probe(params),
            "mmio.falcon.status" => return mmio::mmio_falcon_status(params),
            "ember.falcon.upload_imem" => return mmio::falcon_upload_imem(params),
            "ember.falcon.upload_dmem" => return mmio::falcon_upload_dmem(params),
            "ember.falcon.start_cpu" => return mmio::falcon_start_cpu(params),
            "ember.falcon.poll" => return mmio::falcon_poll(params),
            "ember.pramin.write" => return mmio::pramin_write(params),
            "ember.pramin.read" => return mmio::pramin_read(params),
            "ember.fecs.state" => return mmio::ember_fecs_state(params),
            "ember.device.health" => return mmio::ember_device_health(params),
            "ember.device.recover" => return mmio::ember_device_recover(params),

            "compute.performance_surface.report" => {
                return self.silicon.report(params).await;
            }
            "compute.performance_surface.query" => {
                return self.silicon.query(params).await;
            }
            "compute.performance_surface.list" => return self.silicon.list().await,
            "compute.route.multi_unit" => {
                return self.silicon.route_multi_unit(params).await;
            }

            "provenance.query" | "provenance.get" | "toadstool.provenance" => {
                return Self::toadstool_provenance().await;
            }

            _ => {}
        }

        if let Some(impl_name) = self.semantic_registry.resolve(method) {
            debug!("Semantic resolve: {} → {}", method, impl_name);
            return self
                .dispatch_by_impl_name(impl_name, params, &caller_ctx)
                .await;
        }

        Err(JsonRpcError::method_not_found(method))
    }

    async fn dispatch_by_impl_name(
        &self,
        impl_name: &str,
        params: Option<&serde_json::Value>,
        ctx: &method_gate::CallerContext,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match impl_name {
            "execute_workload" | "submit_workload" => self.workload.submit_workload(params).await,
            "get_workload_status" | "query_status" => self.job.query_status(params).await,
            "cancel_workload" => self.workload.cancel_workload(params).await,
            "list_workloads" => self.job.list_workloads(params).await,
            "validate" => self.workload.validate(params).await,
            "query_capabilities" => self.workload.query_capabilities().await,
            "check_health" => core::health(&self.version, self.start_time, &self.error_count).await,
            "health_version" => core::health_version(self.version.as_ref()).await,
            "health_drain" => core::health_drain(&self.draining, &self.ready).await,
            "dispatch_submit" => {
                self.dispatch
                    .dispatch_submit_with_context(params, ctx)
                    .await
            }
            "compute_fan_out" => self.dispatch.fan_out(params, ctx).await,
            "dispatch_status" => self.dispatch.dispatch_status(params).await,
            "dispatch_result" => self.dispatch.dispatch_result(params).await,
            "dispatch_capabilities" => self.dispatch.dispatch_capabilities(params).await,
            "shader_dispatch" => {
                self.dispatch
                    .shader_dispatch_with_context(params, ctx)
                    .await
            }
            "pipeline_submit" => {
                self.dispatch
                    .pipeline_submit_with_context(params, ctx)
                    .await
            }
            "pipeline_status" => self.dispatch.pipeline_status(params).await,
            "primal_announce" => {
                core::primal_announce(
                    &self.version,
                    &self.semantic_registry,
                    self.bound_socket_path.as_deref().map(PathBuf::as_path),
                )
                .await
            }
            // Science domain — semantic aliases routing to compute handlers
            "science_compute_submit" => self.workload.submit_workload(params).await,
            "science_compute_status" => self.job.query_status(params).await,
            "science_compute_result" => self.dispatch.dispatch_result(params).await,
            "science_compute_cancel" => self.workload.cancel_workload(params).await,
            "science_gpu_dispatch" => {
                self.dispatch
                    .shader_dispatch_with_context(params, ctx)
                    .await
            }
            "science_gpu_capabilities" => self.dispatch.dispatch_capabilities(params).await,
            "science_npu_dispatch" => {
                self.dispatch
                    .dispatch_submit_with_context(params, ctx)
                    .await
            }
            "science_npu_capabilities" => self.dispatch.dispatch_capabilities(params).await,
            "science_substrate_discover" => self.workload.query_capabilities().await,
            "science_substrate_probe" => self.workload.query_capabilities().await,

            // Inference domain — model lifecycle (capability, not product name)
            "inference_list_models" => self.resources.resources_estimate(params).await,
            "inference_execute" => self.resources.resources_estimate(params).await,
            "inference_load_model" => self.resources.resources_estimate(params).await,
            "inference_unload_model" => self.resources.resources_estimate(params).await,

            "toadstool_provenance" => Self::toadstool_provenance().await,
            "gpu_info" => core::gpu_info().await,
            "gpu_memory" => core::gpu_memory().await,
            "gpu_telemetry" => self.hw_learn.gpu_telemetry(params).await,
            "hw_learn_observe" => self.hw_learn.hw_learn_observe(params).await,
            "hw_learn_distill" => self.hw_learn.hw_learn_distill(params).await,
            "hw_learn_apply" => self.hw_learn.hw_learn_apply(params).await,
            "hw_learn_share_recipe" => self.hw_learn.hw_learn_share_recipe(params).await,
            "hw_learn_status" => self.hw_learn.hw_learn_status(params).await,
            "hw_learn_auto_init" => self.hw_learn.hw_learn_auto_init(params).await,
            "hw_learn_auto_init_all" => self.hw_learn.hw_learn_auto_init_all(params).await,
            "hw_learn_vfio_devices" => self.hw_learn.hw_learn_vfio_devices(params).await,
            "performance_surface_report" => self.silicon.report(params).await,
            "performance_surface_query" => self.silicon.query(params).await,
            "performance_surface_list" => self.silicon.list().await,
            "route_multi_unit" => self.silicon.route_multi_unit(params).await,
            "ember_list" => Ok(self.ember_list()),
            "ember_status" => Ok(self.ember_status()),
            "ember_reacquire" => self.ember_reacquire(params).await,
            "ember_warm_cycle" => self.ember_warm_cycle(params).await,
            "ember_prepare_dma" => self.dispatch.ember_prepare_dma(params).await,
            "ember_cleanup_dma" => self.dispatch.ember_cleanup_dma(params).await,
            "ember_adopt_device" => self.ember_adopt_device(params).await,
            "device_swap" => self.device_swap(params).await,
            "device_warm_catch" => self.device_warm_catch(params),
            "device_get" => self.device_get(params),
            "device_experiment_lifecycle" => self.device_experiment_lifecycle(params),
            "device_reset" => self.device_reset(params),
            "device_resurrect" => self.device_resurrect(params).await,
            "ember_device_health" => mmio::ember_device_health(params),
            "device_vfio_open" => self.dispatch.device_vfio_open(params, ctx).await,
            "device_vfio_roundtrip" => self.dispatch.device_vfio_roundtrip(params, ctx).await,
            "device_gr_init" => self.dispatch.device_gr_init(params).await,
            "sovereign_init" => sovereign::sovereign_init(params),
            "sovereign_init_ember" | "sovereign_boot" => {
                self.dispatch.sovereign_init_ember(params).await
            }
            "sovereign_devinit" => sovereign::sovereign_devinit(params),
            "sovereign_defense_status" => {
                Ok(crate::background::catalyst_watchdog::defense_status())
            }
            "sovereign_watchdog_status" => {
                Ok(crate::background::catalyst_watchdog::watchdog_status())
            }
            "mmio_read32" => mmio::mmio_read32(params),
            "mmio_write32" => mmio::mmio_write32(params),
            "mmio_batch" => mmio::mmio_batch(params),
            "mmio_pramin_read32" => mmio::mmio_pramin_read32(params),
            "mmio_bar0_probe" => mmio::mmio_bar0_probe(params),
            "mmio_falcon_status" => mmio::mmio_falcon_status(params),
            "falcon_upload_imem" => mmio::falcon_upload_imem(params),
            "falcon_upload_dmem" => mmio::falcon_upload_dmem(params),
            "falcon_start_cpu" => mmio::falcon_start_cpu(params),
            "falcon_poll" => mmio::falcon_poll(params),
            "pramin_write" => mmio::pramin_write(params),
            "pramin_read" => mmio::pramin_read(params),
            "ember_fecs_state" => mmio::ember_fecs_state(params),
            "ember_device_recover" => mmio::ember_device_recover(params),
            "auth_check" => auth::auth_check(&self.gate, params),
            "auth_mode" => auth::auth_mode(&self.gate),
            "auth_peer_info" => auth::auth_peer_info(ctx),
            _ => Err(JsonRpcError::method_not_found(impl_name)),
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Provenance domain — cross-spring evolution introspection
    // ═══════════════════════════════════════════════════════════

    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    async fn toadstool_provenance() -> Result<serde_json::Value, JsonRpcError> {
        Ok(toadstool::cross_spring_provenance::provenance_json())
    }
}

/// Extract caller provenance from connection-level trust hints.
///
/// Defaults to anonymous. Unix connections without BTSP get
/// [`DispatchTrustLevel::LocalTransport`]. Completed BTSP handshakes set
/// [`DispatchTrustLevel::BtspVerified`] (BearDog JH-1 will add mutual auth).
pub(super) fn extract_caller_context(conn: ConnectionTrustHints) -> CallerContext {
    let mut ctx = CallerContext::anonymous();
    if conn.mutually_authenticated {
        ctx.trust_level = DispatchTrustLevel::MutuallyAuthenticated;
        ctx.gate_id = resolve_local_gate_id();
    } else if conn.btsp_verified {
        ctx.trust_level = DispatchTrustLevel::BtspVerified;
        ctx.gate_id = resolve_local_gate_id();
    } else if conn.transport == ConnectionTransport::Unix {
        ctx.trust_level = DispatchTrustLevel::LocalTransport;
        ctx.gate_id = resolve_local_gate_id();
    }
    ctx
}

pub(crate) fn resolve_local_gate_id() -> Option<String> {
    std::env::var(socket_env::TOADSTOOL_GATE_ID)
        .or_else(|_| std::env::var(socket_env::HOSTNAME))
        .or_else(|_| toadstool_sysmon::system::hostname().ok_or(std::env::VarError::NotPresent))
        .ok()
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
