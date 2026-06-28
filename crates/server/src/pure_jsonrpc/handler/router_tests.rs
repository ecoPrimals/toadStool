// SPDX-License-Identifier: AGPL-3.0-or-later
//! Router contract tests — verify routing tables against semantic registry.

use std::collections::HashSet;

/// All implementation names handled by `dispatch_by_impl_name`.
const DISPATCH_BY_IMPL_HANDLED: &[&str] = &[
    "execute_workload",
    "submit_workload",
    "get_workload_status",
    "query_status",
    "cancel_workload",
    "list_workloads",
    "validate",
    "query_capabilities",
    "check_health",
    "health_version",
    "health_drain",
    "dispatch_submit",
    "compute_fan_out",
    "dispatch_status",
    "dispatch_result",
    "dispatch_capabilities",
    "shader_dispatch",
    "pipeline_submit",
    "pipeline_status",
    "primal_announce",
    "science_compute_submit",
    "science_compute_status",
    "science_compute_result",
    "science_compute_cancel",
    "science_gpu_dispatch",
    "science_gpu_capabilities",
    "science_npu_dispatch",
    "science_npu_capabilities",
    "science_substrate_discover",
    "science_substrate_probe",
    "inference_list_models",
    "inference_execute",
    "inference_load_model",
    "inference_unload_model",
    "toadstool_provenance",
    "gpu_info",
    "gpu_memory",
    "gpu_telemetry",
    "hw_learn_observe",
    "hw_learn_distill",
    "hw_learn_apply",
    "hw_learn_share_recipe",
    "hw_learn_status",
    "hw_learn_auto_init",
    "hw_learn_auto_init_all",
    "hw_learn_vfio_devices",
    "performance_surface_report",
    "performance_surface_query",
    "performance_surface_list",
    "route_multi_unit",
    "ember_list",
    "ember_status",
    "ember_reacquire",
    "ember_warm_cycle",
    "ember_prepare_dma",
    "ember_cleanup_dma",
    "ember_adopt_device",
    "device_swap",
    "device_warm_catch",
    "device_get",
    "device_experiment_lifecycle",
    "device_reset",
    "device_resurrect",
    "ember_device_health",
    "device_vfio_open",
    "device_vfio_roundtrip",
    "device_gr_init",
    "sovereign_init",
    "sovereign_init_ember",
    "sovereign_boot",
    "sovereign_devinit",
    "sovereign_defense_status",
    "sovereign_watchdog_status",
    "mmio_read32",
    "mmio_write32",
    "mmio_batch",
    "mmio_pramin_read32",
    "mmio_bar0_probe",
    "mmio_falcon_status",
    "falcon_upload_imem",
    "falcon_upload_dmem",
    "falcon_start_cpu",
    "falcon_poll",
    "pramin_write",
    "pramin_read",
    "ember_fecs_state",
    "ember_device_recover",
    "auth_check",
    "auth_mode",
    "auth_peer_info",
];

/// All method names handled by the direct `handle_method` match table.
const DIRECT_METHOD_HANDLED: &[&str] = &[
    "auth.check",
    "auth.mode",
    "auth.peer_info",
    "toadstool.submit_workload",
    "toadstool.query_status",
    "toadstool.cancel_workload",
    "toadstool.list_workloads",
    "toadstool.validate",
    "toadstool.query_capabilities",
    "health",
    "toadstool.health",
    "health.check",
    "health.liveness",
    "health.readiness",
    "health.version",
    "health.drain",
    "identity.get",
    "toadstool.version",
    "toadstool.resources.estimate",
    "toadstool.ai.local_inference",
    "resources.estimate",
    "ai.local_inference",
    "toadstool.resources.validate_availability",
    "toadstool.ai.local_execute",
    "resources.validate_availability",
    "ai.local_execute",
    "toadstool.resources.suggest_optimizations",
    "resources.suggest_optimizations",
    "compute.health",
    "compute.version",
    "capabilities.list",
    "capability.list",
    "primal.capabilities",
    "primal.announce",
    "compute.capabilities",
    "compute.discover_capabilities",
    "compute.execute",
    "compute.submit",
    "compute.status",
    "compute.result",
    "compute.cancel",
    "compute.list",
    "compute.dispatch",
    "compute.dispatch.submit",
    "compute.fan_out",
    "compute.dispatch.status",
    "compute.dispatch.result",
    "compute.dispatch.forward",
    "dispatch.verify_trust",
    "compute.dispatch.capabilities",
    "compute.dispatch.pipeline.submit",
    "compute.dispatch.pipeline.status",
    "dispatch.telemetry.schema",
    "gpu.query_info",
    "gpu.info",
    "gpu.query_memory",
    "gpu.memory",
    "gpu.query_telemetry",
    "gpu.telemetry",
    "gate.update",
    "gate.remove",
    "gate.list",
    "gate.route",
    "transport.discover",
    "transport.list",
    "transport.route",
    "transport.open",
    "transport.stream",
    "transport.status",
    "compute.hardware.observe",
    "compute.hardware.distill",
    "compute.hardware.apply",
    "compute.hardware.share_recipe",
    "compute.hardware.auto_init",
    "compute.hardware.auto_init_all",
    "compute.hardware.status",
    "compute.hardware.vfio_devices",
    "shader.dispatch",
    "ember.list",
    "ember.status",
    "ember.reacquire",
    "ember.warm_cycle",
    "ember.prepare_dma",
    "ember.cleanup_dma",
    "ember.adopt_device",
    "device.swap",
    "device.warm_catch",
    "device.get",
    "device.experiment_lifecycle",
    "device.reset",
    "device.resurrect",
    "device.health",
    "device.vfio.open",
    "device.vfio.roundtrip",
    "device.gr.init",
    "compute.context.init",
    "sovereign.init",
    "sovereign.boot",
    "sovereign.profile",
    "sovereign.warm_status",
    "sovereign.defense_status",
    "sovereign.watchdog_status",
    "sovereign.ce_validate",
    "ce.validate",
    "sovereign.pmu_investigate",
    "pmu.investigate",
    "sovereign.warm_handoff",
    "sovereign.catalyst_boot",
    "sovereign.classify_tier",
    "sovereign.experiment",
    "sovereign.devinit",
    "sovereign.kernel_health",
    "sovereign.snapshot",
    "sovereign.compare",
    "sovereign.catalyst_diff",
    "sovereign.reagent_capture",
    "sovereign.recipe_replay",
    "sovereign.runtime_services_probe",
    "mmio.read32",
    "mmio.write32",
    "mmio.batch",
    "mmio.pramin.read32",
    "mmio.bar0.probe",
    "mmio.falcon.status",
    "ember.falcon.upload_imem",
    "ember.falcon.upload_dmem",
    "ember.falcon.start_cpu",
    "ember.falcon.poll",
    "ember.pramin.write",
    "ember.pramin.read",
    "ember.fecs.state",
    "ember.device.health",
    "ember.device.recover",
    "compute.performance_surface.report",
    "compute.performance_surface.query",
    "compute.performance_surface.list",
    "compute.route.multi_unit",
    "provenance.query",
    "provenance.get",
    "toadstool.provenance",
];

#[test]
fn direct_method_table_has_no_duplicates() {
    let mut seen = HashSet::new();
    for method in DIRECT_METHOD_HANDLED {
        assert!(seen.insert(*method), "duplicate direct method: {method}");
    }
}

#[test]
fn dispatch_impl_table_has_no_duplicates() {
    let mut seen = HashSet::new();
    for name in DISPATCH_BY_IMPL_HANDLED {
        assert!(seen.insert(*name), "duplicate dispatch impl name: {name}");
    }
}

#[test]
fn wired_impl_names_are_consistent_with_registry() {
    let registry = toadstool::semantic_methods::SemanticMethodRegistry::new();
    let registered: HashSet<&str> = registry.implementation_names().into_iter().collect();

    let wired_and_registered: Vec<&&str> = DISPATCH_BY_IMPL_HANDLED
        .iter()
        .filter(|name| registered.contains(**name))
        .collect();

    assert!(
        wired_and_registered.len() >= 40,
        "expected at least 40 dispatch arms to match registry, got {}",
        wired_and_registered.len()
    );
}

#[test]
fn semantic_core_methods_all_routable() {
    let registry = toadstool::semantic_methods::SemanticMethodRegistry::new();
    let handled: HashSet<&str> = DISPATCH_BY_IMPL_HANDLED.iter().copied().collect();

    let core_semantics = [
        "compute.execute",
        "compute.cancel",
        "compute.pipeline.submit",
        "compute.pipeline.status",
        "resource.health.check",
        "auth.check",
        "auth.mode",
        "auth.peer_info",
    ];

    for semantic in &core_semantics {
        let impl_name = registry
            .resolve(semantic)
            .unwrap_or_else(|| panic!("{semantic} not in registry"));
        assert!(
            handled.contains(impl_name),
            "{semantic} → {impl_name} not in dispatch_by_impl_name"
        );
    }
}

#[test]
fn direct_method_table_covers_minimum_method_count() {
    assert!(
        DIRECT_METHOD_HANDLED.len() >= 112,
        "expected at least 112 direct methods, got {}",
        DIRECT_METHOD_HANDLED.len()
    );
}

#[test]
fn toadstool_provenance_returns_valid_json() {
    let json = toadstool::cross_spring_provenance::provenance_json();
    assert!(json.is_object());
    let obj = json.as_object().unwrap();
    assert!(obj.contains_key("total_flows"));
    assert!(obj.contains_key("springs"));
    assert!(obj.contains_key("flows"));
}

#[test]
fn all_direct_methods_contain_dot_or_are_bare_health() {
    for method in DIRECT_METHOD_HANDLED {
        assert!(
            method.contains('.') || *method == "health",
            "direct method should be dotted (or bare health): {method}"
        );
    }
}
