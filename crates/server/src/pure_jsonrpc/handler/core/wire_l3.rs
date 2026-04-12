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
        ("toadstool.resources.validate_availability", "resources.validate_availability"),
        ("toadstool.resources.suggest_optimizations", "resources.suggest_optimizations"),
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
mod tests {
    use super::*;

    #[test]
    fn cost_estimates_returns_non_empty_map() {
        let costs = cost_estimates();
        let map = costs.as_object().expect("must be a JSON object");
        assert!(
            map.len() >= 50,
            "expected 50+ cost entries, got {}",
            map.len()
        );
    }

    #[test]
    fn cost_estimates_every_entry_has_required_fields() {
        let costs = cost_estimates();
        let map = costs.as_object().unwrap();
        for (method, entry) in map {
            assert!(
                entry.get("cpu").and_then(|v| v.as_str()).is_some(),
                "{method} missing 'cpu'"
            );
            assert!(
                entry.get("gpu_eligible").is_some(),
                "{method} missing 'gpu_eligible'"
            );
            assert!(
                entry.get("latency_ms").and_then(serde_json::Value::as_u64).is_some(),
                "{method} missing 'latency_ms'"
            );
            assert!(
                entry.get("energy").and_then(|v| v.as_str()).is_some(),
                "{method} missing 'energy'"
            );
            assert!(
                entry
                    .get("memory_pressure")
                    .and_then(|v| v.as_str())
                    .is_some(),
                "{method} missing 'memory_pressure'"
            );
        }
    }

    #[test]
    fn cost_estimates_cpu_values_are_valid_tiers() {
        let valid = ["negligible", "low", "medium", "high", "variable"];
        let costs = cost_estimates();
        for (method, entry) in costs.as_object().unwrap() {
            let cpu = entry["cpu"].as_str().unwrap();
            assert!(
                valid.contains(&cpu),
                "{method}: cpu value '{cpu}' not in {valid:?}"
            );
        }
    }

    #[test]
    fn cost_estimates_energy_values_are_valid_tiers() {
        let valid = ["negligible", "low", "medium", "high", "variable"];
        let costs = cost_estimates();
        for (method, entry) in costs.as_object().unwrap() {
            let energy = entry["energy"].as_str().unwrap();
            assert!(
                valid.contains(&energy),
                "{method}: energy value '{energy}' not in {valid:?}"
            );
        }
    }

    #[test]
    fn cost_estimates_health_methods_are_negligible() {
        let costs = cost_estimates();
        let map = costs.as_object().unwrap();
        for method in ["health.liveness", "health.check", "health.readiness"] {
            let entry = map
                .get(method)
                .unwrap_or_else(|| panic!("missing {method}"));
            assert_eq!(entry["cpu"], "negligible", "{method} cpu");
            assert_eq!(entry["energy"], "negligible", "{method} energy");
            assert!(!entry["gpu_eligible"].as_bool().unwrap(), "{method} gpu");
        }
    }

    #[test]
    fn cost_estimates_gpu_dispatch_is_gpu_eligible() {
        let costs = cost_estimates();
        let map = costs.as_object().unwrap();
        for method in [
            "shader.dispatch",
            "compute.dispatch.submit",
            "compute.dispatch.pipeline.submit",
        ] {
            let entry = map
                .get(method)
                .unwrap_or_else(|| panic!("missing {method}"));
            assert!(
                entry["gpu_eligible"].as_bool().unwrap(),
                "{method} should be gpu_eligible"
            );
        }
    }

    #[test]
    fn cost_estimates_pipeline_submit_has_higher_latency_than_single_dispatch() {
        let costs = cost_estimates();
        let map = costs.as_object().unwrap();
        let single = map["compute.dispatch.submit"]["latency_ms"]
            .as_u64()
            .unwrap();
        let pipeline = map["compute.dispatch.pipeline.submit"]["latency_ms"]
            .as_u64()
            .unwrap();
        assert!(
            pipeline > single,
            "pipeline latency ({pipeline}) should exceed single ({single})"
        );
    }

    #[test]
    fn operation_dependencies_returns_non_empty_object() {
        let deps = operation_dependencies();
        let map = deps.as_object().expect("must be a JSON object");
        assert!(
            map.len() >= 15,
            "expected 15+ dependency entries, got {}",
            map.len()
        );
    }

    #[test]
    fn operation_dependencies_values_are_arrays_of_strings() {
        let deps = operation_dependencies();
        for (method, prereqs) in deps.as_object().unwrap() {
            let arr = prereqs
                .as_array()
                .unwrap_or_else(|| panic!("{method}: prereqs must be an array"));
            for p in arr {
                assert!(
                    p.as_str().is_some(),
                    "{method}: prereq must be a string, got {p}"
                );
            }
        }
    }

    #[test]
    fn operation_dependencies_all_prereqs_exist_in_cost_estimates() {
        let costs = cost_estimates();
        let cost_map = costs.as_object().unwrap();
        let deps = operation_dependencies();
        for (method, prereqs) in deps.as_object().unwrap() {
            for p in prereqs.as_array().unwrap() {
                let prereq = p.as_str().unwrap();
                assert!(
                    cost_map.contains_key(prereq),
                    "dependency for '{method}': prereq '{prereq}' not in cost_estimates"
                );
            }
        }
    }

    #[test]
    fn operation_dependencies_pipeline_status_depends_on_pipeline_submit() {
        let deps = operation_dependencies();
        let map = deps.as_object().unwrap();
        let pipeline_status_prereqs = map["compute.dispatch.pipeline.status"].as_array().unwrap();
        assert!(
            pipeline_status_prereqs
                .iter()
                .any(|v| v == "compute.dispatch.pipeline.submit")
        );
    }

    #[test]
    fn operation_dependencies_transport_stream_depends_on_transport_open() {
        let deps = operation_dependencies();
        let map = deps.as_object().unwrap();
        let prereqs = map["transport.stream"].as_array().unwrap();
        assert!(prereqs.iter().any(|v| v == "transport.open"));
    }

    #[test]
    fn operation_dependencies_no_self_references() {
        let deps = operation_dependencies();
        for (method, prereqs) in deps.as_object().unwrap() {
            for p in prereqs.as_array().unwrap() {
                assert_ne!(
                    p.as_str().unwrap(),
                    method,
                    "method '{method}' lists itself as a prerequisite"
                );
            }
        }
    }
}
