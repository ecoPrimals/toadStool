// SPDX-License-Identifier: AGPL-3.0-or-later

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
            entry
                .get("latency_ms")
                .and_then(serde_json::Value::as_u64)
                .is_some(),
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
