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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use toadstool::semantic_methods::SemanticMethodRegistry;
use tracing::{debug, error, info};

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
    /// PG-62 fast-path: set to `true` once the server is fully initialized.
    /// `health.liveness` returns `"starting"` until this is set.
    ready: Arc<AtomicBool>,
    /// Drain flag: set by `health.drain` to reject new dispatches.
    draining: Arc<AtomicBool>,
    /// JH-0 pre-dispatch capability gate. Ships permissive (all calls allowed).
    gate: method_gate::MethodGate,
    semantic_registry: SemanticMethodRegistry,
    dispatch: DispatchHandler,
    /// Shared anchor store for warm keepalive — leaked on SIGTERM.
    anchor_store: dispatch::AnchorStore,
    hw_learn: HwLearnHandler,
    job: JobHandler,
    workload: WorkloadHandler,
    resources: ResourceHandler,
    transport: TransportHandler,
    silicon: SiliconHandler,
    glowplug: SharedGlowPlugClient,
}

impl JsonRpcHandler {
    /// Create new handler with executor.
    ///
    /// Pass `error_count` to share the counter with other servers for unified monitoring.
    /// Pass `ready` to share the readiness flag — `health.liveness` returns `"starting"`
    /// until this flag is set to `true` (PG-62 fast-path).
    pub fn new(
        executor: Arc<crate::tarpc_server::WorkloadExecutorDispatch>,
        version: impl Into<Arc<str>>,
        error_count: Option<Arc<AtomicU64>>,
        ready: Arc<AtomicBool>,
    ) -> Self {
        let local_gate_id = std::env::var("TOADSTOOL_GATE_ID")
            .or_else(|_| std::env::var("HOSTNAME"))
            .or_else(|_| toadstool_sysmon::system::hostname().ok_or(std::env::VarError::NotPresent))
            .unwrap_or_else(|_| String::from("local"));

        let gate = match std::env::var("TOADSTOOL_AUTH_MODE")
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

        let mut dispatch = DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
            Self::try_connect_security_client(),
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
            job: JobHandler::new(local_gate_id),
            workload: WorkloadHandler::new(executor),
            resources: ResourceHandler::new(),
            transport: TransportHandler::new(),
            silicon: SiliconHandler::new(),
            glowplug: glowplug_client::create_glowplug_client(),
        }
    }

    /// Get the anchor store for wiring into the SIGTERM leak handler.
    pub fn anchor_store(&self) -> dispatch::AnchorStore {
        self.anchor_store.clone()
    }

    /// Attempt to connect to the Tower security client (BearDog) for crypto
    /// delegation. Returns `None` in standalone mode (no `BEARDOG_SOCKET`).
    #[expect(
        deprecated,
        reason = "SecurityClient delegates to crypto.encrypt/decrypt on the wire; crypto_integration migration tracked"
    )]
    fn try_connect_security_client()
    -> Option<Arc<toadstool_distributed::security::client::SecurityClient>> {
        let socket = toadstool_common::primal_sockets::get_socket_path_for_capability("crypto");
        if socket.exists() {
            #[expect(
                deprecated,
                reason = "sync new() is used at startup; async discovery is for hot-path"
            )]
            match toadstool_distributed::security::client::SecurityClient::new(
                toadstool_distributed::security::SecurityConfig::default(),
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
            .handle_method(request.method.as_ref(), request.params.as_ref())
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
    ) -> Result<serde_json::Value, JsonRpcError> {
        // JH-2: extract caller context from request (anonymous until BearDog JH-1 ships)
        let caller_ctx = method_gate::CallerContext::anonymous();

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
            // Wire Standard L1/L2: triad shapes differ; full payload only for check + legacy name.
            "toadstool.health" | "health.check" => {
                return core::health(&self.version, self.start_time, &self.error_count).await;
            }
            "health.liveness" => {
                return core::health_liveness(self.ready.load(Ordering::Relaxed)).await;
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
                return core::primal_announce(&self.version, &self.semantic_registry)
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
            "device.swap" => return self.device_swap(params).await,
            "device.warm_catch" => return self.device_warm_catch(params),
            "device.vfio.open" => return self.dispatch.device_vfio_open(params).await,
            "device.vfio.roundtrip" => {
                return self.dispatch.device_vfio_roundtrip(params).await;
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
            "sovereign.profile" => {
                return self.dispatch.sovereign_profile_ember(params).await;
            }
            "sovereign.warm_status" => {
                return self.dispatch.sovereign_warm_status().await;
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
            "sovereign.classify_tier" => return sovereign::sovereign_classify_tier(params),
            "sovereign.experiment" => return sovereign::sovereign_experiment(params),
            "sovereign.devinit" => return sovereign::sovereign_devinit(params),
            "sovereign.kernel_health" => return sovereign::sovereign_kernel_health(params),

            "mmio.read32" => return mmio::mmio_read32(params),
            "mmio.write32" => return mmio::mmio_write32(params),
            "mmio.batch" => return mmio::mmio_batch(params),
            "mmio.pramin.read32" => return mmio::mmio_pramin_read32(params),
            "mmio.bar0.probe" => return mmio::mmio_bar0_probe(params),
            "mmio.falcon.status" => return mmio::mmio_falcon_status(params),

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
                core::primal_announce(&self.version, &self.semantic_registry).await
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
            "device_swap" => self.device_swap(params).await,
            "device_warm_catch" => self.device_warm_catch(params),
            "device_vfio_open" => self.dispatch.device_vfio_open(params).await,
            "device_vfio_roundtrip" => self.dispatch.device_vfio_roundtrip(params).await,
            "device_gr_init" => self.dispatch.device_gr_init(params).await,
            "sovereign_init" => sovereign::sovereign_init(params),
            "sovereign_devinit" => sovereign::sovereign_devinit(params),
            "mmio_read32" => mmio::mmio_read32(params),
            "mmio_write32" => mmio::mmio_write32(params),
            "mmio_batch" => mmio::mmio_batch(params),
            "mmio_pramin_read32" => mmio::mmio_pramin_read32(params),
            "mmio_bar0_probe" => mmio::mmio_bar0_probe(params),
            "mmio_falcon_status" => mmio::mmio_falcon_status(params),
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

    // ═══════════════════════════════════════════════════════════
    // Ember domain — toadStool-native GPU device management
    // ═══════════════════════════════════════════════════════════

    fn ember_list(&self) -> serde_json::Value {
        let list = self.glowplug.list_devices();
        serde_json::to_value(list).unwrap_or_else(|_| serde_json::json!({"devices": []}))
    }

    fn ember_status(&self) -> serde_json::Value {
        let status = self.glowplug.status();
        serde_json::to_value(status).unwrap_or_else(|_| serde_json::json!({"available": false}))
    }

    async fn ember_reacquire(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        let result = self.glowplug.reacquire(bdf).await;
        serde_json::to_value(&result)
            .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
    }

    /// `device.swap` — swap a GPU to a target personality (e.g. "vfio-pci", "nouveau").
    async fn device_swap(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;
        let target = params
            .and_then(|p| p.get("target"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                JsonRpcError::invalid_params("Missing 'target' string parameter (driver name)")
            })?;

        let result = self.glowplug.swap(bdf, target).await;
        serde_json::to_value(&result)
            .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
    }

    /// `device.warm_catch` — detect warm GPU state via PMC_ENABLE probe.
    fn device_warm_catch(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        Ok(self.glowplug.warm_detect(bdf))
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
