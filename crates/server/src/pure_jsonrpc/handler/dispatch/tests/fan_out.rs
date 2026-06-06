// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for `compute.fan_out` — parallel clone dispatch per S263 wire contract.

use super::test_handler;
use crate::pure_jsonrpc::handler::method_gate::{
    CallerContext, DispatchTrustLevel, ResourceEnvelope,
};

#[tokio::test]
async fn fan_out_assigns_single_unit() {
    let handler = test_handler();
    let params = serde_json::json!({
        "work_units": [{ "unit_id": "clone-001" }],
        "dag_session_id": "tenaillon-2016",
    });
    let result = handler
        .fan_out(Some(&params), &CallerContext::default())
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
        .fan_out(Some(&params), &CallerContext::default())
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
        .fan_out(Some(&params), &CallerContext::default())
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
        .fan_out(Some(&params), &CallerContext::default())
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
        .fan_out(Some(&params), &CallerContext::default())
        .await
        .unwrap_err();
    assert!(err.message.contains("at least one work unit"));
}

#[tokio::test]
async fn fan_out_rejects_missing_work_units() {
    let handler = test_handler();
    let params = serde_json::json!({});
    let err = handler
        .fan_out(Some(&params), &CallerContext::default())
        .await
        .unwrap_err();
    assert!(err.message.contains("work_units"));
}

#[tokio::test]
async fn fan_out_rejects_no_params() {
    let handler = test_handler();
    let err = handler
        .fan_out(None, &CallerContext::default())
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
        .fan_out(Some(&params), &CallerContext::default())
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
        .fan_out(Some(&params), &CallerContext::default())
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
        .fan_out(Some(&params), &CallerContext::default())
        .await
        .expect("fan_out without dag_session_id");
    assert!(result.get("dag_session_id").is_none());
}

fn ctx_with_envelope(cpu_cores: u32) -> CallerContext {
    CallerContext {
        identity: Some("did:key:z6Mk_fan_out_test".into()),
        envelope: Some(ResourceEnvelope {
            cpu_cores: Some(cpu_cores),
            ..ResourceEnvelope::default()
        }),
        ..CallerContext::anonymous()
    }
}

fn work_units_json(count: usize) -> serde_json::Value {
    let units: Vec<serde_json::Value> = (0..count)
        .map(|i| serde_json::json!({ "unit_id": format!("unit-{i}") }))
        .collect();
    serde_json::json!({ "work_units": units })
}

#[tokio::test]
async fn fan_out_envelope_rejects_excess_units() {
    let handler = test_handler();
    let ctx = ctx_with_envelope(1);
    let params = work_units_json(5);
    let err = handler
        .fan_out(Some(&params), &ctx)
        .await
        .unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
    );
    assert!(err.message.contains("Fan-out unit count"));
    assert!(err.message.contains("cpu_cores"));
}

#[tokio::test]
async fn fan_out_no_envelope_allows_any_count() {
    let handler = test_handler();
    let ctx = CallerContext::anonymous();
    let params = work_units_json(20);
    let result = handler
        .fan_out(Some(&params), &ctx)
        .await
        .expect("anonymous caller should not cap fan-out units");
    assert_eq!(result["total_units"], 20);
}

#[tokio::test]
async fn fan_out_includes_caller_context_in_response() {
    let handler = test_handler();
    let ctx = CallerContext {
        gate_id: Some("gate-audit-01".into()),
        trust_level: DispatchTrustLevel::BtspVerified,
        ..CallerContext::anonymous()
    };
    let params = serde_json::json!({
        "work_units": [{ "unit_id": "u1" }],
    });
    let result = handler
        .fan_out(Some(&params), &ctx)
        .await
        .expect("fan_out should echo caller context");
    assert_eq!(result["caller"]["gate_id"], "gate-audit-01");
    assert_eq!(result["caller"]["trust_level"], "btsp_verified");
}
