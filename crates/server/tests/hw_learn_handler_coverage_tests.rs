// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::unwrap_used)] // test code
//! Comprehensive tests for `hw_learn` JSON-RPC handler modules.
//!
//! Covers: `auto_init`, apply, `observe_distill`, `share_recipe`, status, telemetry.

#![allow(clippy::redundant_closure_for_method_calls)]

use temp_env::with_vars;
use toadstool_server::pure_jsonrpc::{HwLearnHandler, JsonRpcError};

/// Minimal valid mmiotrace line (W = write, 4 = width, timestamp, pid, addr, value).
const MINIMAL_MMIOTRACE: &str = "W 4 1.000000 1 0xfee00000 0x00000001 0x00000000 0x0";

/// Build a valid `InitRecipe` JSON for testing.
fn valid_recipe_json() -> String {
    use hw_learn::distiller::{GpuArch, InitRecipe, InitStep, RegFunction, Vendor, VerifyCheck};

    let arch = GpuArch {
        vendor: Vendor::Nvidia,
        generation: "Volta".into(),
        chip: "GV100".into(),
        compute_class: "sm70".into(),
    };
    let recipe = InitRecipe {
        source_arch: arch.clone(),
        source_driver: hw_learn::distiller::DriverKind::Nouveau,
        target_arch: arch,
        steps: vec![
            InitStep::RegisterWrite {
                offset: 0x20000,
                value: 1,
                function: RegFunction::PowerGate,
            },
            InitStep::Verify {
                check: VerifyCheck::ComputeReadback,
            },
        ],
        confidence: 0.0,
        description: "test recipe for coverage".into(),
    };
    hw_learn::knowledge::export_recipe(&recipe).unwrap()
}

// ───── apply.rs ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn apply_valid_recipe_json_dry_run() {
    let dir = std::env::temp_dir().join(format!("hw_apply_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let handler = with_vars(
        [(
            "TOADSTOOL_HW_LEARN_STORE",
            Some(dir.to_string_lossy().as_ref()),
        )],
        HwLearnHandler::new,
    );
    let params = serde_json::json!({
        "recipe_json": valid_recipe_json(),
    });
    let result = handler.hw_learn_apply(Some(&params)).await;
    assert!(result.is_ok(), "apply dry_run failed: {:?}", result.err());
    let value = result.unwrap();
    assert_eq!(value["domain"], "compute.hardware");
    assert_eq!(value["operation"], "apply");
    assert_eq!(value["mode"], "dry_run");
    assert!(value["verdict"].as_str().is_some());
    assert!(value["steps_total"].as_u64().is_some());
}

#[tokio::test]
async fn apply_missing_params() {
    let handler = HwLearnHandler::new();
    let result = handler.hw_learn_apply(None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn apply_empty_params_object() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({});
    let result = handler.hw_learn_apply(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn apply_invalid_recipe_json() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({
        "recipe_json": "not valid json {{{",
    });
    let result = handler.hw_learn_apply(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn apply_recipe_id_nonexistent() {
    let dir = std::env::temp_dir().join(format!("hw_apply_id_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let handler = with_vars(
        [(
            "TOADSTOOL_HW_LEARN_STORE",
            Some(dir.to_string_lossy().as_ref()),
        )],
        HwLearnHandler::new,
    );
    let params = serde_json::json!({
        "recipe_id": "nonexistent-recipe-id-12345",
    });
    let result = handler.hw_learn_apply(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn apply_with_card_path() {
    let dir = std::env::temp_dir().join(format!("hw_apply_card_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let handler = with_vars(
        [(
            "TOADSTOOL_HW_LEARN_STORE",
            Some(dir.to_string_lossy().as_ref()),
        )],
        HwLearnHandler::new,
    );
    let params = serde_json::json!({
        "recipe_json": valid_recipe_json(),
        "card_path": "/dev/dri/card0",
    });
    let result = handler.hw_learn_apply(Some(&params)).await;
    assert!(result.is_ok());
}

// ───── observe_distill.rs ───────────────────────────────────────────────────

#[tokio::test]
async fn observe_valid_trace_data() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({
        "trace_data": MINIMAL_MMIOTRACE,
    });
    let result = handler.hw_learn_observe(Some(&params)).await;
    assert!(result.is_ok(), "observe failed: {:?}", result.err());
    let value = result.unwrap();
    assert_eq!(value["domain"], "compute.hardware");
    assert_eq!(value["operation"], "observe");
    assert!(value["events_count"].as_u64().is_some());
    assert!(value["gpu_id"].as_str().is_some());
}

#[tokio::test]
async fn observe_missing_trace_data() {
    let handler = HwLearnHandler::new();
    let result = handler.hw_learn_observe(None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn observe_empty_params() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({});
    let result = handler.hw_learn_observe(Some(&params)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn distill_valid_params() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({
        "baseline": MINIMAL_MMIOTRACE,
        "compute": "W 4 1.100000 1 0xfee00000 0x00000002 0x00000000 0x0",
        "chip": "gv100",
    });
    let result = handler.hw_learn_distill(Some(&params)).await;
    assert!(result.is_ok(), "distill failed: {:?}", result.err());
    let value = result.unwrap();
    assert_eq!(value["domain"], "compute.hardware");
    assert_eq!(value["operation"], "distill");
    assert_eq!(value["chip"], "gv100");
    assert!(value["diff_count"].as_u64().is_some());
    assert!(value["recipe_steps"].as_u64().is_some());
    assert!(value.get("recipe").is_some());
}

#[tokio::test]
async fn distill_missing_params() {
    let handler = HwLearnHandler::new();
    let result = handler.hw_learn_distill(None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn distill_missing_baseline() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({
        "compute": MINIMAL_MMIOTRACE,
        "chip": "gv100",
    });
    let result = handler.hw_learn_distill(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn distill_missing_compute() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({
        "baseline": MINIMAL_MMIOTRACE,
        "chip": "gv100",
    });
    let result = handler.hw_learn_distill(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn distill_missing_chip() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({
        "baseline": MINIMAL_MMIOTRACE,
        "compute": MINIMAL_MMIOTRACE,
    });
    let result = handler.hw_learn_distill(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

// ───── share_recipe.rs ──────────────────────────────────────────────────────

#[tokio::test]
async fn share_recipe_list() {
    let dir = std::env::temp_dir().join(format!("hw_share_list_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let handler = with_vars(
        [(
            "TOADSTOOL_HW_LEARN_STORE",
            Some(dir.to_string_lossy().as_ref()),
        )],
        HwLearnHandler::new,
    );
    let params = serde_json::json!({ "action": "list" });
    let result = handler.hw_learn_share_recipe(Some(&params)).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["domain"], "compute.hardware");
    assert_eq!(value["operation"], "share_recipe");
    assert_eq!(value["action"], "list");
    assert!(value["architectures"].is_array());
    assert!(value["count"].as_u64().is_some());
}

#[tokio::test]
async fn share_recipe_list_default_action() {
    let dir = std::env::temp_dir().join(format!("hw_share_default_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let handler = with_vars(
        [(
            "TOADSTOOL_HW_LEARN_STORE",
            Some(dir.to_string_lossy().as_ref()),
        )],
        HwLearnHandler::new,
    );
    let params = serde_json::json!({});
    let result = handler.hw_learn_share_recipe(Some(&params)).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["action"], "list");
}

#[tokio::test]
async fn share_recipe_save() {
    let dir = std::env::temp_dir().join(format!("hw_share_save_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let handler = with_vars(
        [(
            "TOADSTOOL_HW_LEARN_STORE",
            Some(dir.to_string_lossy().as_ref()),
        )],
        HwLearnHandler::new,
    );
    let params = serde_json::json!({
        "action": "save",
        "recipe_json": valid_recipe_json(),
    });
    let result = handler.hw_learn_share_recipe(Some(&params)).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["action"], "save");
    assert!(value["recipe_id"].as_str().is_some());
}

#[tokio::test]
async fn share_recipe_save_missing_recipe_json() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({ "action": "save" });
    let result = handler.hw_learn_share_recipe(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn share_recipe_save_invalid_json() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({
        "action": "save",
        "recipe_json": "invalid {{{",
    });
    let result = handler.hw_learn_share_recipe(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn share_recipe_load_nonexistent() {
    let dir = std::env::temp_dir().join(format!("hw_share_load_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let handler = with_vars(
        [(
            "TOADSTOOL_HW_LEARN_STORE",
            Some(dir.to_string_lossy().as_ref()),
        )],
        HwLearnHandler::new,
    );
    let params = serde_json::json!({
        "action": "load",
        "recipe_id": "nonexistent-id-999",
    });
    let result = handler.hw_learn_share_recipe(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn share_recipe_load_missing_recipe_id() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({ "action": "load" });
    let result = handler.hw_learn_share_recipe(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn share_recipe_save_then_load() {
    let dir = std::env::temp_dir().join(format!("hw_share_roundtrip_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let handler = with_vars(
        [(
            "TOADSTOOL_HW_LEARN_STORE",
            Some(dir.to_string_lossy().as_ref()),
        )],
        HwLearnHandler::new,
    );
    let save_params = serde_json::json!({
        "action": "save",
        "recipe_json": valid_recipe_json(),
    });
    let save_result = handler.hw_learn_share_recipe(Some(&save_params)).await;
    assert!(save_result.is_ok());
    let recipe_id = save_result.unwrap()["recipe_id"]
        .as_str()
        .unwrap()
        .to_string();

    let load_params = serde_json::json!({
        "action": "load",
        "recipe_id": recipe_id,
    });
    let load_result = handler.hw_learn_share_recipe(Some(&load_params)).await;
    assert!(load_result.is_ok());
    let value = load_result.unwrap();
    assert!(value.get("recipe").is_some());
}

#[tokio::test]
async fn share_recipe_unknown_action() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({ "action": "unknown_action" });
    let result = handler.hw_learn_share_recipe(Some(&params)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn share_recipe_missing_params() {
    let handler = HwLearnHandler::new();
    let result = handler.hw_learn_share_recipe(None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

// ───── status.rs ────────────────────────────────────────────────────────────

#[tokio::test]
async fn status_no_params() {
    let handler = HwLearnHandler::new();
    let result = handler.hw_learn_status(None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["domain"], "compute.hardware");
    assert_eq!(value["operation"], "status");
    assert!(value.get("pipeline").is_some());
    assert!(value.get("recipes").is_some());
    assert!(value.get("gpus_detected").is_some());
}

#[tokio::test]
async fn status_with_chip() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({ "chip": "gv100" });
    let result = handler.hw_learn_status(Some(&params)).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.get("firmware").is_some());
}

// ───── telemetry.rs ─────────────────────────────────────────────────────────

#[tokio::test]
async fn gpu_telemetry_no_params() {
    let handler = HwLearnHandler::new();
    let result = handler.gpu_telemetry(None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["domain"], "gpu");
    assert_eq!(value["operation"], "telemetry");
    assert!(value["gpus"].is_array());
    assert!(value["gpu_count"].as_u64().is_some());
}

#[tokio::test]
async fn gpu_telemetry_with_params_ignored() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({ "foo": "bar" });
    let result = handler.gpu_telemetry(Some(&params)).await;
    assert!(result.is_ok());
}

// ───── auto_init.rs ────────────────────────────────────────────────────────

#[tokio::test]
async fn auto_init_all_no_gpus_or_empty() {
    let handler = HwLearnHandler::new();
    let result = handler.hw_learn_auto_init_all(None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["domain"], "compute.hardware");
    assert_eq!(value["operation"], "auto_init_all");
    assert!(value["gpus"].is_array());
    assert!(value["total"].as_u64().is_some());
    assert!(value["succeeded"].as_u64().is_some());
    assert!(value["failed"].as_u64().is_some());
}

#[tokio::test]
async fn auto_init_all_with_dry_run_param() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({ "dry_run": true });
    let result = handler.hw_learn_auto_init_all(Some(&params)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn auto_init_all_with_parallel_param() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({ "parallel": false });
    let result = handler.hw_learn_auto_init_all(Some(&params)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn auto_init_missing_gpus_or_recipe() {
    let handler = HwLearnHandler::new();
    let result = handler.hw_learn_auto_init(None).await;
    // Either no GPUs (internal_error) or no recipe (internal_error) or GPU not found (invalid_params)
    assert!(result.is_err());
}

#[tokio::test]
async fn hw_learn_vfio_devices() {
    let handler = HwLearnHandler::new();
    let result = handler.hw_learn_vfio_devices(None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.get("devices").is_some());
    assert!(value.get("count").is_some());
}

#[tokio::test]
async fn auto_init_invalid_bdf() {
    let handler = HwLearnHandler::new();
    let params = serde_json::json!({ "bdf": "0000:99:99.9" });
    let result = handler.hw_learn_auto_init(Some(&params)).await;
    assert!(result.is_err());
    // Either invalid_params (GPU not found) or internal_error (no GPUs at all)
    let err = result.unwrap_err();
    assert!(err.code == JsonRpcError::INVALID_PARAMS || err.code == JsonRpcError::INTERNAL_ERROR);
}
