// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for `compute.fan_out` — parallel clone dispatch per S263 wire contract.

use super::test_handler;

#[tokio::test]
async fn fan_out_assigns_single_unit() {
    let handler = test_handler();
    let params = serde_json::json!({
        "work_units": [{ "unit_id": "clone-001" }],
        "dag_session_id": "tenaillon-2016",
    });
    let result = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .expect("fan_out should succeed");
    assert!(result["dispatch_id"].as_str().unwrap().starts_with("fan-"));
    assert_eq!(result["dag_session_id"], "tenaillon-2016");
    assert_eq!(result["total_units"], 1);
    assert_eq!(result["assigned_count"], 1);
    assert_eq!(result["queued_count"], 0);
    let assigned = result["assigned"].as_array().unwrap();
    assert_eq!(assigned[0]["unit_id"], "clone-001");
    assert_eq!(assigned[0]["status"], "assigned");
}

#[tokio::test]
async fn fan_out_assigns_multiple_units() {
    let handler = test_handler();
    let params = serde_json::json!({
        "work_units": [
            { "unit_id": "clone-001" },
            { "unit_id": "clone-002" },
            { "unit_id": "clone-003" },
        ],
    });
    let result = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .expect("fan_out should succeed with multiple units");
    assert_eq!(result["total_units"], 3);
    assert_eq!(result["assigned_count"], 3);
    assert_eq!(result["queued_count"], 0);
}

#[tokio::test]
async fn fan_out_auto_generates_unit_ids() {
    let handler = test_handler();
    let params = serde_json::json!({
        "work_units": [{}, {}],
    });
    let result = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .expect("fan_out should auto-generate ids");
    let assigned = result["assigned"].as_array().unwrap();
    let dispatch_id = result["dispatch_id"].as_str().unwrap();
    assert_eq!(
        assigned[0]["unit_id"].as_str().unwrap(),
        format!("{dispatch_id}-0")
    );
    assert_eq!(
        assigned[1]["unit_id"].as_str().unwrap(),
        format!("{dispatch_id}-1")
    );
}

#[tokio::test]
async fn fan_out_queues_when_gpu_required_but_unavailable() {
    let handler = test_handler();
    let params = serde_json::json!({
        "work_units": [{ "unit_id": "gpu-job" }],
        "substrate_filter": { "gpu_required": true },
    });
    let result = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .expect("fan_out should queue gpu-required units");
    assert_eq!(result["assigned_count"], 0);
    assert_eq!(result["queued_count"], 1);
    let queued = result["queued"].as_array().unwrap();
    assert_eq!(queued[0]["unit_id"], "gpu-job");
    assert_eq!(queued[0]["status"], "queued");
}

#[tokio::test]
async fn fan_out_rejects_empty_work_units() {
    let handler = test_handler();
    let params = serde_json::json!({ "work_units": [] });
    let err = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .unwrap_err();
    assert!(err.message.contains("at least one work unit"));
}

#[tokio::test]
async fn fan_out_rejects_missing_work_units() {
    let handler = test_handler();
    let params = serde_json::json!({});
    let err = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .unwrap_err();
    assert!(err.message.contains("work_units"));
}

#[tokio::test]
async fn fan_out_rejects_no_params() {
    let handler = test_handler();
    let err = handler
        .fan_out(None, &Default::default())
        .await
        .unwrap_err();
    assert!(err.message.contains("requires params"));
}

#[tokio::test]
async fn fan_out_cpu_substrate_without_gpu() {
    let handler = test_handler();
    let params = serde_json::json!({
        "work_units": [{ "unit_id": "cpu-ok" }],
        "substrate_filter": { "min_cores": 4, "gpu_required": false },
    });
    let result = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .expect("cpu units should be assigned even without GPU");
    assert_eq!(result["assigned_count"], 1);
    assert_eq!(result["queued_count"], 0);
    let assigned = result["assigned"].as_array().unwrap();
    assert_eq!(assigned[0]["substrate"], "cpu");
}

#[tokio::test]
async fn fan_out_includes_timing() {
    let handler = test_handler();
    let params = serde_json::json!({
        "work_units": [{ "unit_id": "t1" }],
    });
    let result = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .expect("fan_out should include timing");
    assert!(result["timing"]["dispatch_ms"].is_number());
}

#[tokio::test]
async fn fan_out_omits_dag_session_id_when_not_provided() {
    let handler = test_handler();
    let params = serde_json::json!({
        "work_units": [{ "unit_id": "u1" }],
    });
    let result = handler
        .fan_out(Some(&params), &Default::default())
        .await
        .expect("fan_out without dag_session_id");
    assert!(result.get("dag_session_id").is_none());
}
