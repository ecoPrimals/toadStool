// SPDX-License-Identifier: AGPL-3.0-or-later
//! Wire Standard L3: per-method cost estimates and operation dependency DAG.

/// Wire Standard L3: per-method compute cost estimates.
///
/// Models cost as energy, time, and compute intensity — not monetary.
/// Dollar value is an end-user concern built on these primitives.
///
/// Fields per method:
/// - `cpu`: negligible | low | medium | high | variable
/// - `gpu_eligible`: whether this method can trigger GPU work
/// - `latency_ms`: expected p50 latency in milliseconds
/// - `energy`: negligible | low | medium | high | variable
/// - `memory_pressure`: none | low | medium | high | variable
pub(super) fn cost_estimates() -> serde_json::Value {
    let mut map = serde_json::Map::with_capacity(60);

    let cost = |cpu: &str, gpu: bool, latency: u32, energy: &str, mem: &str| -> serde_json::Value {
        serde_json::json!({
            "cpu": cpu, "gpu_eligible": gpu, "latency_ms": latency,
            "energy": energy, "memory_pressure": mem
        })
    };

    // Meta / discovery — pure in-memory, no I/O
    for m in [
        "health.liveness",
        "health.check",
        "health.readiness",
        "health.version",
        "health.drain",
        "capabilities.list",
        "identity.get",
        "toadstool.health",
        "toadstool.version",
        "compute.health",
        "compute.version",
        "compute.capabilities",
        "compute.discover_capabilities",
        "compute.dispatch.capabilities",
        "provenance.query",
    ] {
        map.insert(m.into(), cost("negligible", false, 0, "negligible", "none"));
    }

    // AI inference — variable cost depending on model
    map.insert(
        "ai.local_inference".into(),
        cost("variable", true, 200, "variable", "variable"),
    );
    map.insert(
        "ai.local_execute".into(),
        cost("variable", true, 100, "variable", "variable"),
    );

    // toadstool-prefixed resource methods (aliases to resources.*)
    for (prefixed, canonical) in [
        ("toadstool.resources.estimate", "resources.estimate"),
        (
            "toadstool.resources.validate_availability",
            "resources.validate_availability",
        ),
        (
            "toadstool.resources.suggest_optimizations",
            "resources.suggest_optimizations",
        ),
    ] {
        let _ = canonical;
        map.insert(prefixed.into(), cost("medium", false, 10, "low", "low"));
    }

    // Workload lifecycle
    map.insert(
        "toadstool.submit_workload".into(),
        cost("variable", true, 50, "variable", "variable"),
    );
    map.insert(
        "toadstool.query_status".into(),
        cost("negligible", false, 1, "negligible", "none"),
    );
    map.insert(
        "toadstool.cancel_workload".into(),
        cost("low", false, 5, "low", "none"),
    );
    map.insert(
        "toadstool.list_workloads".into(),
        cost("low", false, 2, "low", "low"),
    );
    map.insert(
        "toadstool.validate".into(),
        cost("low", false, 10, "low", "low"),
    );
    map.insert(
        "toadstool.query_capabilities".into(),
        cost("low", false, 5, "low", "low"),
    );

    // Resource estimation
    map.insert(
        "resources.estimate".into(),
        cost("medium", false, 10, "low", "low"),
    );
    map.insert(
        "resources.validate_availability".into(),
        cost("medium", false, 10, "low", "low"),
    );
    map.insert(
        "resources.suggest_optimizations".into(),
        cost("medium", false, 20, "low", "low"),
    );

    // GPU job queue
    map.insert(
        "compute.submit".into(),
        cost("variable", true, 50, "variable", "variable"),
    );
    map.insert(
        "compute.status".into(),
        cost("negligible", false, 1, "negligible", "none"),
    );
    map.insert(
        "compute.result".into(),
        cost("low", false, 5, "low", "medium"),
    );
    map.insert(
        "compute.cancel".into(),
        cost("low", false, 2, "low", "none"),
    );
    map.insert("compute.list".into(), cost("low", false, 2, "low", "low"));

    // Shader dispatch — high GPU, high energy
    map.insert(
        "shader.dispatch".into(),
        cost("high", true, 100, "high", "high"),
    );
    map.insert(
        "compute.dispatch.submit".into(),
        cost("high", true, 100, "high", "high"),
    );
    map.insert(
        "device.gr.init".into(),
        cost("medium", true, 20, "medium", "low"),
    );
    map.insert(
        "compute.context.init".into(),
        cost("medium", true, 20, "medium", "low"),
    );
    map.insert(
        "compute.dispatch.status".into(),
        cost("negligible", false, 1, "negligible", "none"),
    );
    map.insert(
        "compute.dispatch.result".into(),
        cost("low", false, 10, "low", "medium"),
    );
    map.insert(
        "compute.dispatch.forward".into(),
        cost("medium", false, 50, "medium", "low"),
    );
    map.insert(
        "compute.dispatch.pipeline.submit".into(),
        cost("high", true, 500, "high", "high"),
    );
    map.insert(
        "compute.dispatch.pipeline.status".into(),
        cost("negligible", false, 1, "negligible", "none"),
    );

    // Hardware learning — BAR0/MMIO reads
    map.insert(
        "compute.hardware.observe".into(),
        cost("medium", false, 20, "medium", "low"),
    );
    map.insert(
        "compute.hardware.distill".into(),
        cost("high", false, 100, "medium", "medium"),
    );
    map.insert(
        "compute.hardware.apply".into(),
        cost("medium", false, 50, "medium", "low"),
    );
    map.insert(
        "compute.hardware.share_recipe".into(),
        cost("low", false, 10, "low", "low"),
    );
    map.insert(
        "compute.hardware.auto_init".into(),
        cost("high", false, 200, "high", "medium"),
    );
    map.insert(
        "compute.hardware.auto_init_all".into(),
        cost("high", false, 500, "high", "medium"),
    );
    map.insert(
        "compute.hardware.status".into(),
        cost("low", false, 5, "low", "none"),
    );
    map.insert(
        "compute.hardware.vfio_devices".into(),
        cost("low", false, 10, "low", "none"),
    );

    // Performance surface
    map.insert(
        "compute.performance_surface.report".into(),
        cost("medium", false, 20, "low", "medium"),
    );
    map.insert(
        "compute.performance_surface.query".into(),
        cost("medium", false, 10, "low", "low"),
    );
    map.insert(
        "compute.performance_surface.list".into(),
        cost("low", false, 2, "low", "low"),
    );
    map.insert(
        "compute.route.multi_unit".into(),
        cost("medium", false, 15, "low", "low"),
    );

    // GPU telemetry — sysfs/BAR0 reads
    map.insert(
        "gpu.query_info".into(),
        cost("low", false, 10, "low", "low"),
    );
    map.insert(
        "gpu.query_memory".into(),
        cost("low", false, 5, "low", "none"),
    );
    map.insert(
        "gpu.query_telemetry".into(),
        cost("low", false, 10, "low", "none"),
    );

    // Gate routing — network I/O
    map.insert("gate.update".into(), cost("low", false, 5, "low", "none"));
    map.insert("gate.remove".into(), cost("low", false, 2, "low", "none"));
    map.insert("gate.list".into(), cost("low", false, 1, "low", "low"));
    map.insert(
        "gate.route".into(),
        cost("medium", false, 30, "medium", "low"),
    );

    // Transport — DMA/device I/O
    map.insert(
        "transport.discover".into(),
        cost("medium", false, 50, "medium", "low"),
    );
    map.insert(
        "transport.list".into(),
        cost("low", false, 2, "low", "none"),
    );
    map.insert(
        "transport.route".into(),
        cost("medium", false, 20, "medium", "low"),
    );
    map.insert(
        "transport.open".into(),
        cost("medium", false, 50, "medium", "medium"),
    );
    map.insert(
        "transport.stream".into(),
        cost("high", false, 100, "high", "high"),
    );
    map.insert(
        "transport.status".into(),
        cost("low", false, 2, "low", "none"),
    );

    // Ember — device lifecycle
    map.insert("ember.list".into(), cost("low", false, 5, "low", "none"));
    map.insert("ember.status".into(), cost("low", false, 5, "low", "none"));
    map.insert(
        "ember.reacquire".into(),
        cost("medium", true, 30, "medium", "device"),
    );
    map.insert(
        "device.swap".into(),
        cost("high", true, 200, "high", "device"),
    );
    map.insert(
        "device.warm_catch".into(),
        cost("low", false, 10, "low", "none"),
    );

    // MMIO — BAR0 register access
    map.insert(
        "mmio.read32".into(),
        cost("low", false, 1, "low", "none"),
    );
    map.insert(
        "mmio.write32".into(),
        cost("low", false, 1, "low", "none"),
    );
    map.insert(
        "mmio.batch".into(),
        cost("medium", false, 5, "low", "none"),
    );
    map.insert(
        "mmio.pramin.read32".into(),
        cost("low", false, 1, "low", "none"),
    );
    map.insert(
        "mmio.bar0.probe".into(),
        cost("low", false, 5, "low", "none"),
    );
    map.insert(
        "mmio.falcon.status".into(),
        cost("low", false, 5, "low", "none"),
    );

    serde_json::Value::Object(map)
}

/// Wire Standard L3: method prerequisite DAG.
///
/// Maps methods to their prerequisites — what must be called (or
/// have completed) before a given method is meaningful.
pub(super) fn operation_dependencies() -> serde_json::Value {
    serde_json::json!({
        "compute.status":           ["compute.submit"],
        "compute.result":           ["compute.submit"],
        "compute.cancel":           ["compute.submit"],
        "toadstool.query_status":   ["toadstool.submit_workload"],
        "toadstool.cancel_workload":["toadstool.submit_workload"],
        "compute.dispatch.status":  ["compute.dispatch.submit"],
        "compute.dispatch.result":  ["compute.dispatch.submit"],
        "compute.dispatch.pipeline.status": ["compute.dispatch.pipeline.submit"],
        "compute.hardware.distill": ["compute.hardware.observe"],
        "compute.hardware.apply":   ["compute.hardware.distill"],
        "compute.hardware.share_recipe": ["compute.hardware.distill"],
        "compute.hardware.auto_init":    ["compute.hardware.observe"],
        "compute.hardware.auto_init_all":["compute.hardware.observe"],
        "compute.performance_surface.query": ["compute.performance_surface.report"],
        "compute.route.multi_unit":          ["compute.performance_surface.report"],
        "gate.route":               ["gate.update"],
        "gate.remove":              ["gate.update"],
        "transport.route":          ["transport.discover"],
        "transport.open":           ["transport.discover"],
        "transport.stream":         ["transport.open"],
        "transport.status":         ["transport.open"]
    })
}

#[cfg(test)]
#[path = "wire_l3_tests.rs"]
mod tests;
