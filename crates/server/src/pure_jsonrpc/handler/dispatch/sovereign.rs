// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign GPU handlers — ember-managed VFIO pipeline RPC methods.

use super::DispatchHandler;

impl DispatchHandler {
    /// `sovereign.init` via ember — runs the sovereign pipeline using
    /// the clutch (preferred) or cached device BAR0 + DMA.
    ///
    /// Path 1 (clutch): if a VfioAnchor exists for this BDF, engage the clutch
    /// to get fresh BAR0 + DMA from the anchor's fds. No stale state.
    ///
    /// Path 2 (factory): if no anchor, create device via factory (which also
    /// populates the anchor store for future calls), then try clutch again.
    ///
    /// Path 3 (sysfs): last resort — sysfs BAR0 with DMA from cached device.
    pub(in super::super) async fn sovereign_init_ember(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        use crate::pure_jsonrpc::types::JsonRpcError;

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        self.acquire_device_handle(bdf).await;
        crate::background::pcie_keepalive::activity_tracker().record();

        // Try clutch from existing anchor
        let mut clutch = self.try_engage_clutch(bdf).await;

        // No anchor yet — run factory to create device + anchor, then retry
        let used_clutch = if clutch.is_none() {
            let cache = self.get_or_create_device(bdf).await.ok_or_else(|| {
                JsonRpcError::internal_error(format!(
                    "device {bdf} not available — factory returned None"
                ))
            })?;
            drop(cache);
            clutch = self.try_engage_clutch(bdf).await;
            clutch.is_some()
        } else {
            true
        };

        // Resolve BAR0 + DMA from clutch or sysfs fallback
        let sysfs_bar;
        let (bar0_ref, dma_for_opts): (
            &toadstool_cylinder::vfio::device::MappedBar,
            Option<toadstool_cylinder::vfio::device::DmaBackend>,
        ) = if let Some(ref engaged) = clutch {
            (engaged.bar0(), Some(engaged.dma_backend_clone()))
        } else {
            tracing::warn!(bdf, "no clutch available — sysfs BAR0 fallback");
            let bar = toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(
                bdf,
                16 * 1024 * 1024,
            )
            .map_err(|e| {
                JsonRpcError::internal_error(format!(
                    "sysfs BAR0 open failed for {bdf}: {e}"
                ))
            })?;
            let dma = {
                let cache = self.cached_devices.lock().await;
                cache.get(bdf).and_then(|d| d.dma_backend().cloned())
            };
            sysfs_bar = bar;
            (&sysfs_bar, dma)
        };

        let mut opts: toadstool_cylinder::vfio::sovereign_init::SovereignInitOptions =
            if let Some(p) = params {
                serde_json::from_value(p.clone()).unwrap_or_default()
            } else {
                toadstool_cylinder::vfio::sovereign_init::SovereignInitOptions::default()
            };

        if let Some(path) = opts.vbios_rom_path.as_ref()
            && let Ok(rom) = std::fs::read(path)
        {
            opts.vbios_rom = Some(rom);
        }

        // Load engine_init_path (catalyst replay / golden state) if specified.
        // This was previously only wired in the legacy stateless handler.
        if let Some(ref path) = opts.engine_init_path {
            match std::fs::read_to_string(path) {
                Ok(json_str) => {
                    match toadstool_cylinder::nv::gr_init::GrInitSequence::from_json(&json_str) {
                        Ok(seq) => {
                            let engine = seq.chip.engine_label();
                            tracing::info!(
                                bdf, path, writes = seq.len(), engine = engine.as_str(),
                                "sovereign.init(ember): loaded engine init sequence"
                            );
                            opts.engine_init_sequences.push((engine, seq, None));
                        }
                        Err(e) => {
                            tracing::warn!(
                                bdf, path, err = %e,
                                "sovereign.init(ember): failed to parse engine init JSON"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        bdf, path, err = %e,
                        "sovereign.init(ember): failed to read engine init file"
                    );
                }
            }
        }

        opts.dma_backend = dma_for_opts;
        opts.skip_cold_memory_training = true;

        let sm = opts.sm_version.unwrap_or_else(|| {
            let boot0 = bar0_ref.read_u32(0).unwrap_or(0);
            let chip_id = (boot0 >> 20) & 0x1FF;
            let synthetic = chip_id << 20;
            toadstool_cylinder::nv::identity::boot0_to_sm(synthetic).unwrap_or(70)
        });
        let chip = toadstool_cylinder::nv::identity::chip_name(sm);

        let bridge: std::sync::Arc<dyn toadstool_cylinder::nv::gsp_bridge::GspBridge> = {
            let nv = toadstool_cylinder::nv::nv_gsp_bridge::NvGspBridge::new(chip);
            if nv.has_gr_firmware() {
                tracing::info!(chip, bdf, "sovereign.init(ember): using NvGspBridge");
                std::sync::Arc::new(nv)
            } else {
                tracing::info!(chip, bdf, "sovereign.init(ember): using NoopGspBridge");
                std::sync::Arc::new(toadstool_cylinder::nv::gsp_bridge::NoopGspBridge::default())
            }
        };

        let profile = toadstool_cylinder::nv::generation::profile_for_sm(sm);
        let strategy = toadstool_cylinder::vfio::sovereign_strategy::strategy_for_profile(
            profile, bridge, sm,
        );

        let pre_channel_stages = strategy.pre_channel_init(bar0_ref);
        if !pre_channel_stages.is_empty() {
            tracing::info!(
                bdf,
                stages = pre_channel_stages.len(),
                "sovereign.init(ember): pre_channel_init complete"
            );
            for s in &pre_channel_stages {
                tracing::info!(
                    name = %s.name,
                    status = ?s.status,
                    detail = ?s.detail,
                    ms = s.duration_ms,
                    "pre_channel stage"
                );
            }
        }

        tracing::info!(bdf, halt_before = ?opts.halt_before, "sovereign.init(ember): starting pipeline");

        let result = toadstool_cylinder::vfio::sovereign_init::sovereign_init(
            bar0_ref, bdf, &opts, &*strategy,
        );

        // Confirm anchor is live in store for fd persistence across restarts
        let anchor_held = {
            let store = self.anchor_store.lock().await;
            store.contains_key(bdf)
        };

        tracing::info!(
            bdf,
            all_ok = result.all_ok,
            compute_ready = result.compute_ready,
            total_ms = result.total_ms,
            stages = result.stages.len(),
            warm_detected = result.warm_detected,
            clutch_path = used_clutch,
            anchor_held,
            "sovereign.init(ember): pipeline complete"
        );

        if let Some(engaged) = clutch {
            engaged.disengage();
        }

        serde_json::to_value(&result)
            .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
    }

    /// `sovereign.ce_validate` via ember — validates the sovereign DMA
    /// pipeline by dispatching a CE (Copy Engine) DMA copy and verifying
    /// readback. Independent of PGRAPH/GPC state.
    pub(in super::super) async fn sovereign_ce_validate_ember(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        use crate::pure_jsonrpc::types::JsonRpcError;

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        self.acquire_device_handle(bdf).await;
        crate::background::pcie_keepalive::activity_tracker().record();

        let mut clutch = self.try_engage_clutch(bdf).await;
        if clutch.is_none() {
            let cache = self.get_or_create_device(bdf).await.ok_or_else(|| {
                JsonRpcError::internal_error(format!(
                    "device {bdf} not available — factory returned None"
                ))
            })?;
            drop(cache);
            clutch = self.try_engage_clutch(bdf).await;
        }

        let sysfs_bar;
        let (bar0_ref, dma_opt): (
            &toadstool_cylinder::vfio::device::MappedBar,
            Option<toadstool_cylinder::vfio::device::DmaBackend>,
        ) = if let Some(ref engaged) = clutch {
            (engaged.bar0(), Some(engaged.dma_backend_clone()))
        } else {
            tracing::warn!(bdf, "no clutch available for CE validate — sysfs fallback");
            let bar = toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(
                bdf,
                16 * 1024 * 1024,
            )
            .map_err(|e| {
                JsonRpcError::internal_error(format!(
                    "sysfs BAR0 open failed for {bdf}: {e}"
                ))
            })?;
            let dma = {
                let cache = self.cached_devices.lock().await;
                cache.get(bdf).and_then(|d| d.dma_backend().cloned())
            };
            sysfs_bar = bar;
            (&sysfs_bar, dma)
        };

        let dma_backend = dma_opt.ok_or_else(|| {
            JsonRpcError::internal_error(
                "no DMA backend available — CE validate requires VFIO DMA".to_string(),
            )
        })?;

        let result = toadstool_cylinder::vfio::ce_validate::validate_ce(bar0_ref, dma_backend);

        if let Some(engaged) = clutch {
            engaged.disengage();
        }

        serde_json::to_value(&result)
            .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
    }

    /// `sovereign.pmu_investigate` — Exp 211 PMU mailbox investigation.
    ///
    /// Probes the PMU falcon state after nouveau unbind and attempts
    /// progressive ungating strategies to cross Tier 1 → Tier 2.
    /// No DMA required — purely BAR0 register reads/writes.
    pub(in super::super) async fn sovereign_pmu_investigate(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        use crate::pure_jsonrpc::types::JsonRpcError;

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        self.acquire_device_handle(bdf).await;
        crate::background::pcie_keepalive::activity_tracker().record();

        let mut clutch = self.try_engage_clutch(bdf).await;
        if clutch.is_none() {
            let cache = self.get_or_create_device(bdf).await.ok_or_else(|| {
                JsonRpcError::internal_error(format!(
                    "device {bdf} not available — factory returned None"
                ))
            })?;
            drop(cache);
            clutch = self.try_engage_clutch(bdf).await;
        }

        let sysfs_bar;
        let bar0_ref: &toadstool_cylinder::vfio::device::MappedBar = if let Some(ref engaged) =
            clutch
        {
            engaged.bar0()
        } else {
            tracing::warn!(bdf, "no clutch for PMU investigate — sysfs BAR0 rw fallback");
            let bar = toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(
                bdf,
                16 * 1024 * 1024,
            )
            .map_err(|e| {
                JsonRpcError::internal_error(format!(
                    "sysfs BAR0 open failed for {bdf}: {e}"
                ))
            })?;
            sysfs_bar = bar;
            &sysfs_bar
        };

        let result =
            toadstool_cylinder::vfio::pmu_investigate::investigate_pmu(bar0_ref);

        if let Some(engaged) = clutch {
            engaged.disengage();
        }

        serde_json::to_value(&result)
            .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
    }

    /// `sovereign.warm_handoff` — sovereign driver rotation pipeline.
    ///
    /// Orchestrates the full warm handoff: module patching → insmod →
    /// seeder bind → settle → warm swap to vfio-pci → tier classification
    /// → rmmod. The operator never touches the kernel.
    ///
    /// Params:
    /// - `bdf`: PCI BDF of the target GPU (required)
    /// - `strategy`: warm handoff strategy name (required)
    ///   - `"nouveau_titanv"`: patched nouveau for Volta (GV100)
    ///   - `"nouveau_k80"`: stock nouveau for Kepler (GK210)
    pub(in super::super) async fn sovereign_warm_handoff(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        use crate::pure_jsonrpc::types::JsonRpcError;
        use toadstool_cylinder::vfio::sovereign_handoff::{HandoffConfig, execute_handoff};

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        let strategy = params
            .and_then(|p| p.get("strategy"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'strategy' string parameter"))?;

        let mut config = HandoffConfig::from_strategy(strategy, bdf).ok_or_else(|| {
            JsonRpcError::invalid_params(format!(
                "Unknown warm handoff strategy: '{strategy}'. \
                 Valid: nouveau_titanv, nouveau_k80, nvidia_titanv, nvidia_patched_titanv, nvidia_catalyst_titanv"
            ))
        })?;

        if let Some(secs) = params.and_then(|p| p.get("settle_secs")).and_then(serde_json::Value::as_u64) {
            config.settle = std::time::Duration::from_secs(secs);
        }

        if let Some(json) = params.and_then(|p| p.get("patch_set_json")).and_then(serde_json::Value::as_str) {
            config.patch_set_override = Some(json.to_string());
        }

        if let Some(skip) = params.and_then(|p| p.get("skip_preflight")).and_then(serde_json::Value::as_bool) {
            config.skip_preflight = skip;
        }

        if let Some(name) = params.and_then(|p| p.get("module_name")).and_then(serde_json::Value::as_str) {
            config.module_name = name.to_string();
            config.seeder_driver = name.to_string();
        }

        tracing::info!(
            bdf,
            strategy,
            settle_secs = config.settle.as_secs(),
            skip_preflight = config.skip_preflight,
            has_patch_override = config.patch_set_override.is_some(),
            "sovereign.warm_handoff: starting driver rotation pipeline"
        );

        // Read PMC_ENABLE before releasing anchor to detect cold GPU.
        let gpu_warm = {
            use toadstool_cylinder::vfio::device::MappedBar;
            if let Ok(bar) = MappedBar::from_sysfs_rw(bdf, 16 * 1024 * 1024) {
                let pmc = bar.read_u32(0x200).unwrap_or(0);
                let popcount = pmc.count_ones();
                tracing::info!(bdf, pmc = format_args!("0x{pmc:08x}"), popcount, "pre-release PMC_ENABLE");
                popcount >= 10
            } else {
                true
            }
        };

        // Catalyst strategies ALWAYS need SBR: RM's rm_init_adapter must
        // probe a clean post-reset GPU to populate the GPU manager's probed
        // table. Without SBR, RM sees stale engine state from previous
        // catalyst cycles and silently skips GPU instance registration —
        // GPU_GET_PROBED_IDS returns empty, device_alloc fails with 0x22.
        // Non-catalyst strategies preserve warm state (FLR/SBR suppressed).
        let is_catalyst_strategy = strategy.contains("catalyst");
        let suppress_sbr = if is_catalyst_strategy {
            tracing::info!(bdf, gpu_warm, "catalyst strategy: allowing SBR for clean RM probe");
            false
        } else {
            gpu_warm
        };

        toadstool_cylinder::vfio::guarded_sysfs::prepare_anchor_release(bdf, suppress_sbr);

        // Release VFIO anchor and cached device. The IOMMU group is locked
        // while we hold VFIO container/group FDs — the seeder driver cannot
        // bind until we release them. FLR is already suppressed above.
        {
            let mut anchors = self.anchor_store.lock().await;
            if let Some(anchor) = anchors.remove(bdf) {
                anchor.release_prepared();
            }
        }
        {
            let mut cache = self.cached_devices.lock().await;
            if cache.remove(bdf).is_some() {
                tracing::info!(bdf, "released cached device for warm handoff");
            }
        }

        // Close any leaked sysfs resource0 fds for this BDF. The sovereign
        // pipeline and health monitoring open BAR0 via sysfs and intentionally
        // leak the fd (MappedBar pattern). The kernel's request_mem_region()
        // in the seeder driver (nvsov/nouveau) will fail if the BAR region
        // is still held open. This was the Exp 219 blocker.
        {
            let bdf_owned = bdf.to_string();
            let closed = toadstool_cylinder::vfio::guarded_sysfs::release_bar0_fds(&bdf_owned);
            if closed > 0 {
                tracing::info!(bdf, closed, "released leaked BAR0 resource0 fds for warm handoff");
            }
        }

        // The handoff changes the GPU's driver binding (vfio → nouveau →
        // vfio), so any pre-existing VFIO BAR0 mapping is invalidated.
        // Pass None — the orchestrator uses sysfs BAR0 for post-handoff
        // tier classification after vfio-pci rebind.
        //
        // Wrapped in tokio::time::timeout to prevent indefinite RPC hangs.
        // The handoff itself has internal deadlines via guarded_sysfs, but
        // this outer timeout is the last line of defense.
        // 420s: catalyst teardown on GV100 needs ~160s for nvidia RM
        // shutdown (HBM2 dealloc, falcon halt) + 15s settle + 30s probe
        // + 30s BAR0 capture margin.
        let rpc_timeout = std::time::Duration::from_secs(420);
        let blocking_future = tokio::task::spawn_blocking(move || {
            execute_handoff(&config, None)
        });

        let result = match tokio::time::timeout(rpc_timeout, blocking_future).await {
            Ok(Ok(handoff_result)) => handoff_result,
            Ok(Err(e)) => {
                return Err(JsonRpcError::internal_error(
                    format!("handoff task panicked: {e}"),
                ));
            }
            Err(_elapsed) => {
                tracing::error!(bdf, timeout_s = rpc_timeout.as_secs(),
                    "sovereign.warm_handoff RPC timeout — blocking thread abandoned");
                return Err(JsonRpcError::internal_error(format!(
                    "warm_handoff timed out after {}s (blocking thread abandoned, \
                     internal guarded operations will self-terminate)",
                    rpc_timeout.as_secs(),
                )));
            }
        };

        tracing::info!(
            bdf,
            success = result.success,
            tier = ?result.tier.as_ref().map(|t| t.tier),
            total_ms = result.total_ms,
            "sovereign.warm_handoff: complete"
        );

        serde_json::to_value(&result)
            .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
    }

    /// `sovereign.catalyst_boot` — catalyst-free boot: nouveau warm handoff +
    /// golden state replay + tier classification.
    ///
    /// The end-state pipeline: no proprietary driver at runtime. Uses nouveau
    /// for HBM2 training and basic engine init, then replays the catalyst's
    /// golden state to bring TPC PRI stations alive.
    ///
    /// Params:
    /// - `bdf` (required): PCI BDF of the target GPU
    /// - `engine_init_path` (required): Path to catalyst replay JSON
    /// - `settle_secs` (optional): Override settle duration (default: 5s)
    pub(in super::super) async fn sovereign_catalyst_boot(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        use crate::pure_jsonrpc::types::JsonRpcError;
        use toadstool_cylinder::vfio::sovereign_handoff::{HandoffConfig, execute_handoff};

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        let engine_init_path = params
            .and_then(|p| p.get("engine_init_path"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params(
                "Missing 'engine_init_path' — path to catalyst replay JSON"
            ))?;

        // Validate the replay file exists and parses before starting handoff
        let replay_json = std::fs::read_to_string(engine_init_path).map_err(|e| {
            JsonRpcError::invalid_params(format!(
                "Cannot read engine_init_path '{engine_init_path}': {e}"
            ))
        })?;
        let replay_seq = toadstool_cylinder::nv::gr_init::GrInitSequence::from_json(&replay_json)
            .map_err(|e| {
                JsonRpcError::invalid_params(format!(
                    "Invalid GrInitSequence JSON in '{engine_init_path}': {e}"
                ))
            })?;

        tracing::info!(
            bdf,
            engine_init_path,
            replay_writes = replay_seq.len(),
            "sovereign.catalyst_boot: starting catalyst-free boot"
        );

        // Step 1: Nouveau warm handoff
        let mut config = HandoffConfig::nouveau_titanv(bdf);
        if let Some(secs) = params.and_then(|p| p.get("settle_secs")).and_then(serde_json::Value::as_u64) {
            config.settle = std::time::Duration::from_secs(secs);
        }

        // Suppress FLR before releasing anchor (Exp 225 fix).
        // catalyst_boot always uses nouveau which doesn't need RM DEVINIT,
        // so always suppress SBR to preserve any existing warm state.
        toadstool_cylinder::vfio::guarded_sysfs::prepare_anchor_release(bdf, true);

        // Release VFIO resources — FLR already suppressed above
        {
            let mut anchors = self.anchor_store.lock().await;
            if let Some(anchor) = anchors.remove(bdf) {
                anchor.release_prepared();
            }
        }
        {
            let mut cache = self.cached_devices.lock().await;
            if cache.remove(bdf).is_some() {
                tracing::info!(bdf, "catalyst_boot: released cached device");
            }
        }
        {
            let closed = toadstool_cylinder::vfio::guarded_sysfs::release_bar0_fds(bdf);
            if closed > 0 {
                tracing::info!(bdf, closed, "catalyst_boot: released leaked BAR0 resource0 fds");
            }
        }

        let bdf_owned = bdf.to_string();
        let rpc_timeout = std::time::Duration::from_secs(90);
        let blocking_future = tokio::task::spawn_blocking(move || {
            execute_handoff(&config, None)
        });

        let handoff_result = match tokio::time::timeout(rpc_timeout, blocking_future).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                return Err(JsonRpcError::internal_error(
                    format!("handoff task panicked: {e}"),
                ));
            }
            Err(_) => {
                return Err(JsonRpcError::internal_error(
                    "catalyst_boot: nouveau warm handoff timed out".to_string(),
                ));
            }
        };

        if !handoff_result.success {
            tracing::warn!(
                bdf = bdf_owned.as_str(),
                halted_at = ?handoff_result.halted_at,
                "catalyst_boot: nouveau warm handoff failed"
            );
            return serde_json::to_value(serde_json::json!({
                "success": false,
                "phase": "warm_handoff",
                "handoff": handoff_result,
            }))
            .map_err(|e| JsonRpcError::internal_error(format!("serialization: {e}")));
        }

        tracing::info!(
            bdf = bdf_owned.as_str(),
            handoff_ms = handoff_result.total_ms,
            handoff_tier = ?handoff_result.tier.as_ref().map(|t| t.tier),
            "catalyst_boot: nouveau handoff complete, replaying golden state"
        );

        // Step 2: Replay golden state via sovereign.init
        let init_params = serde_json::json!({
            "bdf": bdf_owned,
            "engine_init_path": engine_init_path,
        });
        let init_result = self
            .sovereign_init_ember(Some(&init_params))
            .await;

        match init_result {
            Ok(init_val) => {
                let final_tier = init_val.get("compute_ready")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);

                tracing::info!(
                    bdf = bdf_owned.as_str(),
                    compute_ready = final_tier,
                    "catalyst_boot: pipeline complete"
                );

                Ok(serde_json::json!({
                    "success": true,
                    "bdf": bdf_owned,
                    "handoff": {
                        "success": handoff_result.success,
                        "tier": handoff_result.tier,
                        "total_ms": handoff_result.total_ms,
                    },
                    "init": init_val,
                    "catalyst_free": true,
                }))
            }
            Err(e) => {
                Ok(serde_json::json!({
                    "success": false,
                    "phase": "sovereign_init",
                    "handoff": {
                        "success": handoff_result.success,
                        "tier": handoff_result.tier,
                        "total_ms": handoff_result.total_ms,
                    },
                    "error": format!("{e:?}"),
                }))
            }
        }
    }

    /// `sovereign.profile` via ember — instrumented pipeline with microsecond
    /// timing, boot state snapshots, and register captures.
    pub(in super::super) async fn sovereign_profile_ember(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        use crate::pure_jsonrpc::types::JsonRpcError;

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

        self.acquire_device_handle(bdf).await;
        crate::background::pcie_keepalive::activity_tracker().record();

        let mut clutch = self.try_engage_clutch(bdf).await;

        if clutch.is_none() {
            let cache = self.get_or_create_device(bdf).await.ok_or_else(|| {
                JsonRpcError::internal_error(format!(
                    "device {bdf} not available — factory returned None"
                ))
            })?;
            drop(cache);
            clutch = self.try_engage_clutch(bdf).await;
        }

        let sysfs_bar;
        let (bar0_ref, dma_for_opts): (
            &toadstool_cylinder::vfio::device::MappedBar,
            Option<toadstool_cylinder::vfio::device::DmaBackend>,
        ) = if let Some(ref engaged) = clutch {
            (engaged.bar0(), Some(engaged.dma_backend_clone()))
        } else {
            tracing::warn!(bdf, "no clutch available — sysfs BAR0 fallback");
            let bar = toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(
                bdf, 16 * 1024 * 1024,
            ).map_err(|e| {
                JsonRpcError::internal_error(format!("sysfs BAR0 open failed for {bdf}: {e}"))
            })?;
            let dma = {
                let cache = self.cached_devices.lock().await;
                cache.get(bdf).and_then(|d| d.dma_backend().cloned())
            };
            sysfs_bar = bar;
            (&sysfs_bar, dma)
        };

        let mut opts: toadstool_cylinder::vfio::sovereign_init::SovereignInitOptions =
            if let Some(p) = params {
                serde_json::from_value(p.clone()).unwrap_or_default()
            } else {
                toadstool_cylinder::vfio::sovereign_init::SovereignInitOptions::default()
            };

        if let Some(path) = opts.vbios_rom_path.as_ref()
            && let Ok(rom) = std::fs::read(path)
        {
            opts.vbios_rom = Some(rom);
        }
        opts.dma_backend = dma_for_opts;
        opts.skip_cold_memory_training = true;

        let sm = opts.sm_version.unwrap_or_else(|| {
            let boot0 = bar0_ref.read_u32(0).unwrap_or(0);
            let chip_id = (boot0 >> 20) & 0x1FF;
            let synthetic = chip_id << 20;
            toadstool_cylinder::nv::identity::boot0_to_sm(synthetic).unwrap_or(70)
        });
        let chip = toadstool_cylinder::nv::identity::chip_name(sm);

        let bridge: std::sync::Arc<dyn toadstool_cylinder::nv::gsp_bridge::GspBridge> = {
            let nv = toadstool_cylinder::nv::nv_gsp_bridge::NvGspBridge::new(chip);
            if nv.has_gr_firmware() {
                std::sync::Arc::new(nv)
            } else {
                std::sync::Arc::new(toadstool_cylinder::nv::gsp_bridge::NoopGspBridge::default())
            }
        };

        let profile = toadstool_cylinder::nv::generation::profile_for_sm(sm);
        let strategy = toadstool_cylinder::vfio::sovereign_strategy::strategy_for_profile(
            profile, bridge, sm,
        );

        tracing::info!(bdf, "sovereign.profile: starting instrumented pipeline");

        let result = toadstool_cylinder::vfio::sovereign_profile::sovereign_profile(
            bar0_ref, bdf, &opts, &*strategy,
        );

        let anchor_held = {
            let store = self.anchor_store.lock().await;
            store.contains_key(bdf)
        };

        tracing::info!(
            bdf,
            compute_ready = result.result.compute_ready,
            pipeline_us = result.result.total_ms * 1000,
            overhead_us = result.profiling_overhead_us,
            stages = result.stage_timings_us.len(),
            anchor_held,
            "sovereign.profile: complete"
        );

        if let Some(engaged) = clutch {
            engaged.disengage();
        }

        serde_json::to_value(&result)
            .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
    }

    /// `sovereign.warm_status` — lightweight warm keepalive status for all known GPUs.
    ///
    /// Reports anchor state, boot state probe (via sysfs BAR0), and fd store
    /// capability without running any pipeline. Used to verify fd persistence
    /// across daemon restarts.
    pub(in super::super) async fn sovereign_warm_status(
        &self,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        let anchors = self.anchor_store.lock().await;
        let fd_store_capable = std::env::var(toadstool_common::interned_strings::socket_env::NOTIFY_SOCKET).is_ok();

        let mut devices = serde_json::Map::new();

        // Report on anchored devices
        for (bdf, _anchor) in anchors.iter() {
            let boot_probe = probe_boot_state_sysfs(bdf);
            let tier = classify_tier_sysfs(bdf);
            devices.insert(bdf.clone(), serde_json::json!({
                "anchor_held": true,
                "boot_state": boot_probe.as_ref().map_or("unknown", |s| s.0.as_str()),
                "pmc_enable": boot_probe.as_ref().map_or("n/a", |s| s.1.as_str()),
                "pramin_ok": boot_probe.as_ref().is_some_and(|s| s.2),
                "fd_store_capable": fd_store_capable,
                "sovereign_tier": tier.as_ref().map(|t| t.tier.level()),
                "sovereign_tier_name": tier.as_ref().map(|t| t.tier.description()),
            }));
        }

        // Also report cached devices not yet anchored
        let cache = self.cached_devices.lock().await;
        for bdf in cache.keys() {
            if !devices.contains_key(bdf) {
                let boot_probe = probe_boot_state_sysfs(bdf);
                let tier = classify_tier_sysfs(bdf);
                devices.insert(bdf.clone(), serde_json::json!({
                    "anchor_held": false,
                    "boot_state": boot_probe.as_ref().map_or("unknown", |s| s.0.as_str()),
                    "pmc_enable": boot_probe.as_ref().map_or("n/a", |s| s.1.as_str()),
                    "pramin_ok": boot_probe.as_ref().is_some_and(|s| s.2),
                    "fd_store_capable": fd_store_capable,
                    "sovereign_tier": tier.as_ref().map(|t| t.tier.level()),
                    "sovereign_tier_name": tier.as_ref().map(|t| t.tier.description()),
                }));
            }
        }

        Ok(serde_json::json!({
            "anchor_count": anchors.len(),
            "fd_store_capable": fd_store_capable,
            "devices": devices,
        }))
    }
}

/// Probe boot state and sovereignty tier via sysfs BAR0.
/// Returns (state_name, pmc_hex, pramin_ok) or None on failure.
fn probe_boot_state_sysfs(bdf: &str) -> Option<(String, String, bool)> {
    use toadstool_cylinder::vfio::device::MappedBar;
    use toadstool_cylinder::vfio::probe_boot_state;

    let bar = MappedBar::from_sysfs_rw(bdf, 16 * 1024 * 1024).ok()?;
    let state = probe_boot_state(&bar, None);
    let pmc = bar.read_u32(0x200).unwrap_or(0);
    let pramin_ok = state.is_warm();
    let state_name = if state.is_warm() { "warm" } else { "cold" };
    Some((state_name.to_string(), format!("0x{pmc:08x}"), pramin_ok))
}

/// Classify the sovereignty tier for a device via sysfs BAR0.
fn classify_tier_sysfs(bdf: &str) -> Option<toadstool_cylinder::vfio::sovereign_tiers::TierEvidence> {
    use toadstool_cylinder::vfio::device::MappedBar;
    let bar = MappedBar::from_sysfs_rw(bdf, 16 * 1024 * 1024).ok()?;
    Some(toadstool_cylinder::vfio::sovereign_tiers::classify_tier(&bar))
}
