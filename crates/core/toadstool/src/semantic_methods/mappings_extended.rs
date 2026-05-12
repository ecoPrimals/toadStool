// SPDX-License-Identifier: AGPL-3.0-or-later
//! Extended semantic mappings: runtime, science, ecology, discovery, deploy, shader,
//! silicon performance, provenance, inference, and GPU query surfaces.

/// Register runtime, science stack, shader, performance surface, provenance, inference,
/// and GPU query mappings.
///
/// Call order matches the historical `SemanticMethodRegistry::new` layout.
pub(crate) fn register<F>(add_mapping: &mut F)
where
    F: FnMut(&str, &str),
{
    // ═══════════════════════════════════════════════════════════
    // RUNTIME DOMAIN - Runtime engine management
    // ═══════════════════════════════════════════════════════════

    add_mapping("runtime.engine.list", "list_runtime_engines");
    add_mapping("runtime.engine.get", "get_runtime_engine");
    add_mapping("runtime.engine.capabilities", "get_runtime_capabilities");

    add_mapping("runtime.workload.submit", "submit_workload");
    add_mapping("runtime.workload.status", "get_workload_status");
    add_mapping("runtime.workload.result", "get_workload_result");
    add_mapping("runtime.workload.list", "list_workloads");
    add_mapping("runtime.workload.validate", "validate");

    // ═══════════════════════════════════════════════════════════
    // SCIENCE DOMAIN - Scientific compute IPC for springs
    // ═══════════════════════════════════════════════════════════

    add_mapping("science.compute.submit", "science_compute_submit");
    add_mapping("science.compute.status", "science_compute_status");
    add_mapping("science.compute.result", "science_compute_result");
    add_mapping("science.compute.cancel", "science_compute_cancel");

    add_mapping("science.gpu.dispatch", "science_gpu_dispatch");
    add_mapping("science.gpu.capabilities", "science_gpu_capabilities");

    add_mapping("science.npu.dispatch", "science_npu_dispatch");
    add_mapping("science.npu.capabilities", "science_npu_capabilities");

    add_mapping("science.substrate.discover", "science_substrate_discover");
    add_mapping("science.substrate.probe", "science_substrate_probe");

    // ═══════════════════════════════════════════════════════════
    // ECOLOGY DOMAIN - airSpring science offload routing
    //
    // Springs call these through toadStool as compute.offload
    // targets. toadStool routes to the appropriate science
    // primal discovered at runtime via capability sockets.
    // ═══════════════════════════════════════════════════════════

    add_mapping("ecology.et0_fao56", "ecology_et0_fao56");
    add_mapping("ecology.water_balance", "ecology_water_balance");
    add_mapping("ecology.yield_response", "ecology_yield_response");
    add_mapping("ecology.thornthwaite", "ecology_thornthwaite");
    add_mapping("ecology.gdd", "ecology_gdd");
    add_mapping("ecology.pedotransfer", "ecology_pedotransfer");
    add_mapping("ecology.spi_drought_index", "ecology_spi_drought_index");
    add_mapping("ecology.autocorrelation", "ecology_autocorrelation");
    add_mapping("ecology.gamma_cdf", "ecology_gamma_cdf");
    add_mapping("ecology.runoff_scs_cn", "ecology_runoff_scs_cn");
    add_mapping("ecology.van_genuchten_theta", "ecology_van_genuchten_theta");
    add_mapping("ecology.van_genuchten_k", "ecology_van_genuchten_k");
    add_mapping("ecology.bootstrap_ci", "ecology_bootstrap_ci");
    add_mapping("ecology.jackknife_ci", "ecology_jackknife_ci");

    // ═══════════════════════════════════════════════════════════
    // DISCOVERY DOMAIN - NUCLEUS primal discovery (groundSpring V99)
    //
    // Adaptive health checks and direct primal socket discovery
    // absorbed from groundSpring's live NUCLEUS integration.
    // ═══════════════════════════════════════════════════════════

    add_mapping("discovery.primals", "discovery_primals");
    add_mapping("discovery.primal_health", "discovery_primal_health");
    add_mapping("discovery.direct_rpc", "discovery_direct_rpc");
    add_mapping("discovery.topology", "discovery_topology");

    // ═══════════════════════════════════════════════════════════
    // DEPLOY DOMAIN - Science primal deploy graphs (wetSpring V99)
    //
    // Capability routing for science primal orchestration.
    // ═══════════════════════════════════════════════════════════

    add_mapping("deploy.capability_call", "deploy_capability_call");
    add_mapping("deploy.graph_status", "deploy_graph_status");

    // ═══════════════════════════════════════════════════════════
    // SHADER DOMAIN - Dispatch via visualization service pipeline
    //
    // shader.compile.* removed (Mar 2026): compilation is the visualization
    // service's domain. Callers connect via capability discovery.
    // ═══════════════════════════════════════════════════════════

    add_mapping("shader.dispatch", "shader_dispatch");

    // ═══════════════════════════════════════════════════════════
    // COMPUTE DISPATCH DOMAIN - GPU binary dispatch (PG-15 compliance)
    //
    // `compute.dispatch` is the bare entry point (routes to dispatch_submit).
    // `compute.dispatch.submit` through `compute.dispatch.pipeline.status`
    // are literal-only routes in the handler match.
    // ═══════════════════════════════════════════════════════════

    add_mapping("compute.dispatch", "dispatch_submit");

    // ═══════════════════════════════════════════════════════════
    // SILICON DOMAIN - All-silicon pipeline / performance surface (S159)
    //
    // Springs report measured throughput for (op, unit, precision)
    // triples. toadStool stores and queries for tolerance routing.
    // ═══════════════════════════════════════════════════════════

    add_mapping(
        "compute.performance_surface.report",
        "performance_surface_report",
    );
    add_mapping(
        "compute.performance_surface.query",
        "performance_surface_query",
    );
    add_mapping(
        "compute.performance_surface.list",
        "performance_surface_list",
    );
    add_mapping("compute.route.multi_unit", "route_multi_unit");

    // ═══════════════════════════════════════════════════════════
    // PROVENANCE DOMAIN - Cross-spring evolution tracking
    // ═══════════════════════════════════════════════════════════

    add_mapping("toadstool.provenance", "toadstool_provenance"); // deprecated: primal name as domain
    add_mapping("provenance.get", "toadstool_provenance");
    add_mapping("provenance.query", "toadstool_provenance"); // canonical

    // ═══════════════════════════════════════════════════════════
    // INFERENCE DOMAIN - Model inference (capability, not product)
    // ═══════════════════════════════════════════════════════════

    add_mapping("ollama.list_models", "inference_list_models"); // deprecated: product name as domain
    add_mapping("inference.list_models", "inference_list_models"); // canonical
    add_mapping("ollama.inference", "inference_execute"); // deprecated
    add_mapping("inference.execute", "inference_execute"); // canonical
    add_mapping("ollama.load", "inference_load_model"); // deprecated
    add_mapping("inference.load_model", "inference_load_model"); // canonical
    add_mapping("ollama.unload", "inference_unload_model"); // deprecated
    add_mapping("inference.unload_model", "inference_unload_model"); // canonical

    // ═══════════════════════════════════════════════════════════
    // GPU DOMAIN - Query operations (verb form per naming standard)
    // ═══════════════════════════════════════════════════════════

    add_mapping("gpu.info", "gpu_info"); // deprecated: noun → verb
    add_mapping("gpu.query_info", "gpu_info"); // canonical
    add_mapping("gpu.memory", "gpu_memory"); // deprecated: noun → verb
    add_mapping("gpu.query_memory", "gpu_memory"); // canonical
    add_mapping("gpu.telemetry", "gpu_telemetry"); // deprecated: noun → verb
    add_mapping("gpu.query_telemetry", "gpu_telemetry"); // canonical
}
