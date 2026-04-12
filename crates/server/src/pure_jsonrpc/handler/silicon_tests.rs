// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn make_measurement(op: &str, unit: SiliconUnit, tflops: f64, tol: f64) -> serde_json::Value {
    serde_json::json!({
        "operation": op,
        "silicon_unit": unit.as_str(),
        "precision_mode": "fp32",
        "throughput_gflops": tflops,
        "tolerance_achieved": tol,
        "gpu_model": "RTX 3090",
        "measured_by": "test",
        "timestamp": 1_710_000_000_u64
    })
}

#[tokio::test]
async fn report_and_list() {
    let handler = SiliconHandler::new();
    let m = make_measurement("math.pairwise.yukawa", SiliconUnit::RtCore, 5400.0, 1e-7);
    let result = handler.report(Some(&m)).await.unwrap();
    assert_eq!(result["status"], "recorded");
    assert_eq!(result["total_measurements"], 1);

    let list = handler.list().await.unwrap();
    assert_eq!(list["total_measurements"], 1);
    assert_eq!(list["operations"][0], "math.pairwise.yukawa");
}

#[tokio::test]
async fn query_finds_best_unit() {
    let handler = SiliconHandler::new();

    let m1 = make_measurement("neighbor_search", SiliconUnit::ShaderCore, 540.0, 1e-7);
    let m2 = make_measurement("neighbor_search", SiliconUnit::RtCore, 5400.0, 1e-3);

    handler.report(Some(&m1)).await.unwrap();
    handler.report(Some(&m2)).await.unwrap();

    let query = serde_json::json!({
        "operation": "neighbor_search",
        "tolerance_required": 1e-2
    });
    let result = handler.query(Some(&query)).await.unwrap();
    assert_eq!(result["recommended_unit"], "rt_core");
    assert_eq!(result["fallback_unit"], "shader_core");
}

#[tokio::test]
async fn query_no_matches() {
    let handler = SiliconHandler::new();
    let query = serde_json::json!({
        "operation": "unknown_op",
        "tolerance_required": 1e-7
    });
    let result = handler.query(Some(&query)).await.unwrap();
    assert!(result["recommendation"].is_null());
}

#[tokio::test]
async fn list_empty() {
    let handler = SiliconHandler::new();
    let result = handler.list().await.unwrap();
    assert_eq!(result["total_measurements"], 0);
    assert_eq!(result["all_known_units"].as_array().unwrap().len(), 9);
}

#[tokio::test]
async fn report_missing_params() {
    let handler = SiliconHandler::new();
    let err = handler.report(None).await.unwrap_err();
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn route_multi_unit_with_surface_data() {
    let handler = SiliconHandler::new();

    handler
        .report(Some(&make_measurement(
            "neighbor_search",
            SiliconUnit::RtCore,
            5400.0,
            1e-3,
        )))
        .await
        .unwrap();
    handler
        .report(Some(&make_measurement(
            "neighbor_search",
            SiliconUnit::ShaderCore,
            540.0,
            1e-7,
        )))
        .await
        .unwrap();
    handler
        .report(Some(&make_measurement(
            "force_eval",
            SiliconUnit::ShaderCore,
            3240.0,
            1e-14,
        )))
        .await
        .unwrap();
    handler
        .report(Some(&serde_json::json!({
            "operation": "accumulation",
            "silicon_unit": "rop",
            "precision_mode": "fp32",
            "throughput_gflops": 2700.0,
            "tolerance_achieved": 1e-7,
            "gpu_model": "RTX 3090",
            "measured_by": "test",
            "timestamp": 1_710_000_000_u64
        })))
        .await
        .unwrap();

    let params = serde_json::json!({
        "workload": [
            { "op": "neighbor_search", "tolerance": 1e-2 },
            { "op": "force_eval", "tolerance": 1e-14 },
            { "op": "accumulation", "tolerance": 1e-7 }
        ],
        "gpu": "RTX 3090"
    });

    let result = handler.route_multi_unit(Some(&params)).await.unwrap();

    assert_eq!(result["gpu_target"], "RTX 3090");
    let ops = result["operations"].as_array().unwrap();
    assert_eq!(ops.len(), 3);

    assert_eq!(ops[0]["silicon_unit"], "rt_core");
    assert_eq!(ops[0]["operation"], "neighbor_search");
    assert!(ops[0]["fallback"].is_object());
    assert_eq!(ops[0]["fallback"]["silicon_unit"], "shader_core");

    assert_eq!(ops[1]["silicon_unit"], "shader_core");
    assert_eq!(ops[1]["operation"], "force_eval");

    assert_eq!(ops[2]["silicon_unit"], "rop");
    assert_eq!(ops[2]["operation"], "accumulation");

    let total = result["total_estimated_throughput_gflops"]
        .as_f64()
        .unwrap();
    assert!(total > 10_000.0);
}

#[tokio::test]
async fn route_multi_unit_heuristic_fallback() {
    let handler = SiliconHandler::new();

    let params = serde_json::json!({
        "workload": [
            { "op": "neighbor_search", "tolerance": 1e-2 },
            { "op": "unknown_op", "tolerance": 1e-7 }
        ]
    });

    let result = handler.route_multi_unit(Some(&params)).await.unwrap();
    let ops = result["operations"].as_array().unwrap();

    assert_eq!(ops[0]["silicon_unit"], "rt_core");
    assert!(ops[0]["reason"].as_str().unwrap().contains("heuristic"));

    assert_eq!(ops[1]["silicon_unit"], "shader_core");
    assert!(ops[1]["reason"].as_str().unwrap().contains("heuristic"));
}

#[tokio::test]
async fn route_multi_unit_empty_workload() {
    let handler = SiliconHandler::new();
    let params = serde_json::json!({ "workload": [] });
    let err = handler.route_multi_unit(Some(&params)).await.unwrap_err();
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn route_multi_unit_missing_params() {
    let handler = SiliconHandler::new();
    let err = handler.route_multi_unit(None).await.unwrap_err();
    assert_eq!(err.code, -32602);
}
