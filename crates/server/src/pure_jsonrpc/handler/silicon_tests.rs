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

#[tokio::test]
async fn query_partial_match_tolerance_too_tight() {
    let handler = SiliconHandler::new();
    handler
        .report(Some(&make_measurement(
            "matmul",
            SiliconUnit::TensorCore,
            142_000.0,
            1e-3,
        )))
        .await
        .unwrap();

    let query = serde_json::json!({
        "operation": "matmul",
        "tolerance_required": 1e-14
    });
    let result = handler.query(Some(&query)).await.unwrap();
    assert!(result["recommendation"].is_null());
    assert!(
        result["message"]
            .as_str()
            .unwrap()
            .contains("no measurements")
    );
}

#[tokio::test]
async fn query_missing_operation_param() {
    let handler = SiliconHandler::new();
    let err = handler
        .query(Some(&serde_json::json!({"tolerance_required": 1e-7})))
        .await
        .unwrap_err();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("operation"));
}

#[tokio::test]
async fn query_missing_tolerance_param() {
    let handler = SiliconHandler::new();
    let err = handler
        .query(Some(&serde_json::json!({"operation": "matmul"})))
        .await
        .unwrap_err();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("tolerance_required"));
}

#[tokio::test]
async fn query_best_shader_core_no_separate_fallback() {
    let handler = SiliconHandler::new();
    handler
        .report(Some(&make_measurement(
            "force_eval",
            SiliconUnit::ShaderCore,
            35_580.0,
            1e-7,
        )))
        .await
        .unwrap();

    let result = handler
        .query(Some(&serde_json::json!({
            "operation": "force_eval",
            "tolerance_required": 1e-6
        })))
        .await
        .unwrap();
    assert_eq!(result["recommended_unit"], "shader_core");
    assert_eq!(result["fallback_unit"], "shader_core");
}

#[tokio::test]
async fn report_invalid_measurement_shape() {
    let handler = SiliconHandler::new();
    let err = handler
        .report(Some(&serde_json::json!({"bad": true})))
        .await
        .unwrap_err();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("invalid measurement"));
}

#[tokio::test]
async fn route_multi_unit_single_op() {
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

    let result = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [{ "op": "neighbor_search", "tolerance": 1e-2 }]
        })))
        .await
        .unwrap();
    let ops = result["operations"].as_array().unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0]["silicon_unit"], "rt_core");
    assert_eq!(result["gpu_target"], "default");
}

#[tokio::test]
async fn route_multi_unit_heterogeneous_units() {
    let handler = SiliconHandler::new();
    handler
        .report(Some(&make_measurement(
            "matmul",
            SiliconUnit::TensorCore,
            142_000.0,
            1e-3,
        )))
        .await
        .unwrap();
    handler
        .report(Some(&make_measurement(
            "histogram_deposit",
            SiliconUnit::Rop,
            2700.0,
            1e-7,
        )))
        .await
        .unwrap();

    let result = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [
                { "op": "matmul", "tolerance": 1e-2 },
                { "op": "histogram_deposit", "tolerance": 1e-7 },
                { "op": "eos_table_lookup", "tolerance": 1e-7 }
            ]
        })))
        .await
        .unwrap();
    let ops = result["operations"].as_array().unwrap();
    assert_eq!(ops.len(), 3);
    assert_eq!(ops[0]["silicon_unit"], "tensor_core");
    assert_eq!(ops[1]["silicon_unit"], "rop");
    assert_eq!(ops[2]["silicon_unit"], "texture_unit");
    assert!(ops[2]["reason"].as_str().unwrap().contains("heuristic"));
}

#[tokio::test]
async fn route_multi_unit_tolerance_exceeds_surface_uses_heuristic() {
    let handler = SiliconHandler::new();
    handler
        .report(Some(&make_measurement(
            "matmul",
            SiliconUnit::TensorCore,
            142_000.0,
            1e-3,
        )))
        .await
        .unwrap();

    let result = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [{ "op": "matmul", "tolerance": 1e-14 }]
        })))
        .await
        .unwrap();
    let op = &result["operations"][0];
    assert_eq!(op["silicon_unit"], "shader_core");
    assert_eq!(op["precision_mode"], "df64");
    assert!(op["reason"].as_str().unwrap().contains("heuristic"));
}

#[tokio::test]
async fn route_multi_unit_workload_item_missing_op() {
    let handler = SiliconHandler::new();
    let err = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [{ "tolerance": 1e-7 }]
        })))
        .await
        .unwrap_err();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("op"));
}

#[tokio::test]
async fn route_multi_unit_workload_item_missing_tolerance() {
    let handler = SiliconHandler::new();
    let err = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [{ "op": "matmul" }]
        })))
        .await
        .unwrap_err();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("tolerance"));
}

#[tokio::test]
async fn route_heuristic_matmul_loose_vs_tight_tolerance() {
    let handler = SiliconHandler::new();

    let loose = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [{ "op": "cg_solve_matmul", "tolerance": 1e-3 }]
        })))
        .await
        .unwrap();
    assert_eq!(loose["operations"][0]["silicon_unit"], "tensor_core");
    assert_eq!(loose["operations"][0]["precision_mode"], "fp16");

    let tight = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [{ "op": "cg_solve_matmul", "tolerance": 1e-14 }]
        })))
        .await
        .unwrap();
    assert_eq!(tight["operations"][0]["silicon_unit"], "shader_core");
    assert_eq!(tight["operations"][0]["precision_mode"], "df64");
}

#[tokio::test]
async fn route_heuristic_spatial_and_scatter_ops() {
    let handler = SiliconHandler::new();

    let spatial = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [{ "op": "bvh_spatial_query", "tolerance": 1e-7 }]
        })))
        .await
        .unwrap();
    assert_eq!(spatial["operations"][0]["silicon_unit"], "rt_core");

    let scatter = handler
        .route_multi_unit(Some(&serde_json::json!({
            "workload": [{ "op": "scatter_deposit", "tolerance": 1e-7 }]
        })))
        .await
        .unwrap();
    assert_eq!(scatter["operations"][0]["silicon_unit"], "rop");
}
