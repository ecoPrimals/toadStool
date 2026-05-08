// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use super::DispatchHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

fn test_handler() -> DispatchHandler {
    DispatchHandler::new(
        Arc::new(crate::visualization_client::VisualizationClient::unavailable()),
        None,
    )
}

fn submit_params(bdf: &str, dispatch_mode: &str) -> serde_json::Value {
    serde_json::json!({
        "binary": [1u8, 2, 3],
        "bdf": bdf,
        "dispatch_mode": dispatch_mode,
    })
}

#[tokio::test]
async fn dispatch_capabilities_returns_expected_structure() {
    let handler = test_handler();
    let result = handler
        .dispatch_capabilities(None)
        .await
        .expect("capabilities");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "capabilities");
    assert_eq!(result["status"], "completed");
    assert!(result["output"]["sovereign_pipeline"].as_bool().unwrap());
    assert!(result["output"]["dispatch_modes"].as_array().is_some());
    assert!(result["output"]["vfio_gpus"].as_array().is_some());
    assert!(result["output"]["drm_gpus"].as_array().is_some());
    assert!(result["output"]["total_dispatch_count"].as_u64().is_some());
    assert!(result["output"]["shader_compiler_available"].is_boolean());
}

#[tokio::test]
async fn dispatch_capabilities_total_dispatch_count_increments_after_submit() {
    let handler = test_handler();
    let before = handler
        .dispatch_capabilities(None)
        .await
        .expect("capabilities")["output"]["total_dispatch_count"]
        .as_u64()
        .expect("total_dispatch_count");

    let params = submit_params("0000:03:00.0", "passthrough");
    handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");

    let after = handler
        .dispatch_capabilities(None)
        .await
        .expect("capabilities")["output"]["total_dispatch_count"]
        .as_u64()
        .expect("total_dispatch_count");
    assert_eq!(after, before + 1);
}

#[tokio::test]
async fn dispatch_submit_missing_params_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .dispatch_submit(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("binary"));
}

#[tokio::test]
async fn dispatch_submit_empty_binary_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "binary": [] });
    let err = handler
        .dispatch_submit(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("empty"));
}

#[tokio::test]
async fn dispatch_submit_missing_binary_field_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "bdf": "0000:03:00.0" });
    let err = handler
        .dispatch_submit(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_submit_binary_not_array_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": "not-an-array",
        "bdf": "0000:03:00.0",
    });
    let err = handler
        .dispatch_submit(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_submit_vfio_mode_without_coral_returns_failed_payload() {
    let coral = crate::visualization_client::create_visualization_client();
    if coral.is_available().await {
        return;
    }
    let handler = DispatchHandler::new(coral, None);
    let params = submit_params("0000:03:00.0", "vfio");
    let result = handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["status"], "failed");
    assert!(
        result["error"]
            .as_str()
            .is_some_and(|s| s.contains("visualization") || s.contains("shader"))
    );
}

#[tokio::test]
async fn dispatch_submit_drm_mode_without_coral_returns_failed_payload() {
    let coral = crate::visualization_client::create_visualization_client();
    if coral.is_available().await {
        return;
    }
    let handler = DispatchHandler::new(coral, None);
    let params = submit_params("0000:03:00.0", "drm");
    let result = handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");
    assert_eq!(result["status"], "failed");
    assert!(
        result["error"]
            .as_str()
            .is_some_and(|s| s.contains("visualization") || s.contains("shader"))
    );
}

#[tokio::test]
async fn dispatch_submit_custom_dispatch_mode_registers_job_for_status_and_result() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1u8, 2, 3],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
        "workgroup_size": [128, 2, 4],
        "buffers": [{ "name": "a", "size": 16 }],
        "timeout_ms": 9999u64,
    });
    let result = handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");
    assert_eq!(result["domain"], "compute.dispatch");
    let job_id = result["job_id"].as_str().expect("job_id");
    assert_eq!(
        result["metadata"]["workgroup_size"],
        serde_json::json!([128, 2, 4])
    );

    let status = handler
        .dispatch_status(Some(&serde_json::json!({ "job_id": job_id })))
        .await
        .expect("status");
    assert_eq!(status["job_id"], job_id);
    assert!(status["status"].as_str().is_some());
    assert_eq!(status["metadata"]["bdf"], "0000:03:00.0");

    let got = handler
        .dispatch_result(Some(&serde_json::json!({ "job_id": job_id })))
        .await
        .expect("result");
    assert_eq!(got["job_id"], job_id);
}

#[tokio::test]
async fn dispatch_status_unknown_job_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": "nonexistent-uuid" });
    let err = handler
        .dispatch_status(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("not found"));
}

#[tokio::test]
async fn dispatch_status_missing_job_id_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .dispatch_status(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);

    let err = handler
        .dispatch_status(Some(&serde_json::json!({})))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_status_job_id_not_string_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": 12345 });
    let err = handler
        .dispatch_status(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_result_unknown_job_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": "nonexistent-uuid" });
    let err = handler
        .dispatch_result(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("not found"));
}

#[tokio::test]
async fn dispatch_result_missing_job_id_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .dispatch_result(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_result_job_id_not_string_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": true });
    let err = handler
        .dispatch_result(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_forward_missing_params_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .dispatch_forward(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_forward_missing_endpoint_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "binary": [1] });
    let err = handler
        .dispatch_forward(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("endpoint"));
}

#[tokio::test]
async fn dispatch_forward_unreachable_returns_internal_error() {
    let handler = test_handler();
    let params = serde_json::json!({
        "endpoint": "127.0.0.1:1",
        "binary": [1, 2],
        "bdf": "0000:03:00.0",
    });
    let err = handler
        .dispatch_forward(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("127.0.0.1:1") || err.message.contains("failed"));
}

#[tokio::test]
async fn dispatch_forward_uses_nested_params_when_present() {
    let handler = test_handler();
    let params = serde_json::json!({
        "endpoint": "127.0.0.1:1",
        "params": {
            "binary": [9],
            "bdf": "0000:03:00.0",
        },
    });
    let err = handler
        .dispatch_forward(Some(&params))
        .await
        .expect_err("expected transport error");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
}

// ═══════════════════════════════════════════════════════════
// crypto dispatch path tests (Phase 55)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn dispatch_submit_standalone_mode_has_no_encrypted_flag() {
    let handler = test_handler();
    let params = submit_params("0000:03:00.0", "passthrough");
    let result = handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");
    assert!(
        result["metadata"].get("encrypted").is_none() || result["metadata"]["encrypted"] == false,
        "standalone dispatch must not set encrypted=true"
    );
}

#[tokio::test]
async fn dispatch_handler_new_with_none_security_client_works() {
    let handler = DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
        None,
    );
    let params = submit_params("0000:03:00.0", "passthrough");
    let result = handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");
    assert_eq!(result["domain"], "compute.dispatch");
}

// ═══════════════════════════════════════════════════════════
// shader.dispatch tests (ludoSpring V35 / visualization service Iter 70)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn shader_dispatch_missing_params_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .shader_dispatch(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("binary"));
}

#[tokio::test]
async fn shader_dispatch_missing_binary_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "bdf": "0000:03:00.0" });
    let err = handler
        .shader_dispatch(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("binary") || err.message.contains("compile_result"));
}

#[tokio::test]
async fn shader_dispatch_empty_binary_array_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "binary": [], "bdf": "0000:03:00.0" });
    let err = handler
        .shader_dispatch(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("empty"));
}

#[tokio::test]
async fn shader_dispatch_empty_base64_returns_invalid_params() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": STANDARD.encode([]),
        "bdf": "0000:03:00.0"
    });
    let err = handler
        .shader_dispatch(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn shader_dispatch_invalid_base64_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": "!!!not-valid-base64!!!",
        "bdf": "0000:03:00.0"
    });
    let err = handler
        .shader_dispatch(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("base64"));
}

#[tokio::test]
async fn shader_dispatch_binary_not_string_or_array_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": 42,
        "bdf": "0000:03:00.0"
    });
    let err = handler
        .shader_dispatch(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("base64 string") || err.message.contains("array"));
}

#[tokio::test]
async fn shader_dispatch_accepts_base64_binary() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let handler = test_handler();
    let binary_data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02];
    let params = serde_json::json!({
        "binary": STANDARD.encode(&binary_data),
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
    });
    let result = handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "shader");
    assert_eq!(result["metadata"]["binary_size"], binary_data.len());
    assert!(result["job_id"].as_str().is_some());
}

#[tokio::test]
async fn shader_dispatch_accepts_u8_array_binary() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [0xDE, 0xAD, 0xBE, 0xEF],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
    });
    let result = handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["metadata"]["binary_size"], 4);
}

#[tokio::test]
async fn shader_dispatch_accepts_compile_result_shape() {
    let handler = test_handler();
    let params = serde_json::json!({
        "compile_result": {
            "binary": [1, 2, 3, 4, 5],
            "arch": "sm89",
            "target_device": 0
        },
        "workgroup_size": [64, 1, 1],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
    });
    let result = handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["metadata"]["arch"], "sm89");
    assert_eq!(result["metadata"]["binary_size"], 5);
    assert_eq!(
        result["metadata"]["workgroup_size"],
        serde_json::json!([64, 1, 1])
    );
}

#[tokio::test]
async fn shader_dispatch_compile_result_base64_binary() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let handler = test_handler();
    let binary = vec![0x90, 0x90, 0x90];
    let params = serde_json::json!({
        "compile_result": {
            "binary": STANDARD.encode(&binary),
            "arch": "gfx1030",
        },
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
    });
    let result = handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");
    assert_eq!(result["metadata"]["arch"], "gfx1030");
    assert_eq!(result["metadata"]["binary_size"], 3);
}

#[tokio::test]
async fn shader_dispatch_compile_result_missing_binary_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({
        "compile_result": { "arch": "sm89" },
        "bdf": "0000:03:00.0",
    });
    let err = handler
        .shader_dispatch(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("binary"));
}

#[tokio::test]
async fn shader_dispatch_readback_defaults_to_true() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1, 2, 3],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
    });
    let result = handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");
    assert_eq!(result["metadata"]["readback"], true);
}

#[tokio::test]
async fn shader_dispatch_readback_false_honored() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1, 2, 3],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
        "readback": false,
    });
    let result = handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");
    assert_eq!(result["metadata"]["readback"], false);
}

#[tokio::test]
async fn shader_dispatch_vfio_without_coral_returns_failed() {
    let coral = crate::visualization_client::create_visualization_client();
    if coral.is_available().await {
        return;
    }
    let handler = DispatchHandler::new(coral, None);
    let params = serde_json::json!({
        "binary": [1, 2, 3],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "vfio",
    });
    let result = handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["status"], "failed");
    assert!(
        result["error"]
            .as_str()
            .is_some_and(|s| s.contains("visualization") || s.contains("shader"))
    );
}

#[tokio::test]
async fn shader_dispatch_increments_dispatch_count() {
    let handler = test_handler();
    let before =
        handler.dispatch_capabilities(None).await.expect("caps")["output"]["total_dispatch_count"]
            .as_u64()
            .expect("count");

    let params = serde_json::json!({
        "binary": [1, 2],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
    });
    handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");

    let after =
        handler.dispatch_capabilities(None).await.expect("caps")["output"]["total_dispatch_count"]
            .as_u64()
            .expect("count");
    assert_eq!(after, before + 1);
}

#[tokio::test]
async fn shader_dispatch_job_trackable_via_status_and_result() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [0xCA, 0xFE],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
        "workgroup_size": [32, 2, 1],
    });
    let result = handler
        .shader_dispatch(Some(&params))
        .await
        .expect("shader dispatch");
    let job_id = result["job_id"].as_str().expect("job_id");

    let status = handler
        .dispatch_status(Some(&serde_json::json!({ "job_id": job_id })))
        .await
        .expect("status");
    assert_eq!(status["job_id"], job_id);
    assert!(status["status"].as_str().is_some());

    let got = handler
        .dispatch_result(Some(&serde_json::json!({ "job_id": job_id })))
        .await
        .expect("result");
    assert_eq!(got["job_id"], job_id);
}

// ═══════════════════════════════════════════════════════════
// JH-2: Resource envelope enforcement tests
// ═══════════════════════════════════════════════════════════

mod envelope_tests {
    use super::*;
    use crate::pure_jsonrpc::handler::method_gate::{CallerContext, ResourceEnvelope};
    use super::super::submit::enforce_envelope;

    fn envelope(
        mem_mb: Option<u64>,
        cpu_cores: Option<u32>,
        max_timeout_ms: Option<u64>,
    ) -> ResourceEnvelope {
        ResourceEnvelope {
            mem_mb,
            cpu_cores,
            max_timeout_ms,
            method_allowlist: vec![],
        }
    }

    fn ctx_with(env: ResourceEnvelope) -> CallerContext {
        CallerContext {
            identity: Some("did:key:z6Mk_test".into()),
            envelope: Some(env),
        }
    }

    #[test]
    fn no_envelope_always_passes() {
        let ctx = CallerContext::anonymous();
        assert!(enforce_envelope(&ctx, 1024 * 1024 * 100, 256, 5000).is_ok());
    }

    #[test]
    fn envelope_without_mem_limit_passes() {
        let ctx = ctx_with(envelope(None, Some(4), None));
        assert!(enforce_envelope(&ctx, 1024 * 1024 * 500, 256, 5000).is_ok());
    }

    #[test]
    fn envelope_mem_limit_allows_within_bounds() {
        let ctx = ctx_with(envelope(Some(100), None, None));
        assert!(enforce_envelope(&ctx, 50 * 1024 * 1024, 256, 5000).is_ok());
    }

    #[test]
    fn envelope_mem_limit_rejects_over_bounds() {
        let ctx = ctx_with(envelope(Some(10), None, None));
        let err = enforce_envelope(&ctx, 20 * 1024 * 1024, 256, 5000).unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
        );
        assert!(err.message.contains("exceeds token envelope"));
    }

    #[test]
    fn envelope_mem_limit_boundary_exact() {
        let ctx = ctx_with(envelope(Some(1), None, None));
        assert!(enforce_envelope(&ctx, 1024 * 1024, 256, 5000).is_ok());
        assert!(enforce_envelope(&ctx, 1024 * 1024 + 1, 256, 5000).is_err());
    }

    #[test]
    fn envelope_cpu_cores_allows_within_bounds() {
        let ctx = ctx_with(envelope(None, Some(2), None));
        assert!(enforce_envelope(&ctx, 100, 2048, 5000).is_ok());
    }

    #[test]
    fn envelope_cpu_cores_rejects_over_bounds() {
        let ctx = ctx_with(envelope(None, Some(2), None));
        let err = enforce_envelope(&ctx, 100, 2049, 5000).unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
        );
        assert!(err.message.contains("cpu_cores"));
    }

    #[test]
    fn envelope_timeout_allows_within_bounds() {
        let ctx = ctx_with(envelope(None, None, Some(10_000)));
        assert!(enforce_envelope(&ctx, 100, 256, 10_000).is_ok());
    }

    #[test]
    fn envelope_timeout_rejects_over_bounds() {
        let ctx = ctx_with(envelope(None, None, Some(5_000)));
        let err = enforce_envelope(&ctx, 100, 256, 5_001).unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
        );
        assert!(err.message.contains("timeout"));
    }

    #[test]
    fn envelope_all_dimensions_checked() {
        let ctx = ctx_with(envelope(Some(100), Some(4), Some(30_000)));
        assert!(enforce_envelope(&ctx, 50 * 1024 * 1024, 4096, 30_000).is_ok());
        assert!(enforce_envelope(&ctx, 200 * 1024 * 1024, 4096, 30_000).is_err());
        assert!(enforce_envelope(&ctx, 50 * 1024 * 1024, 4097, 30_000).is_err());
        assert!(enforce_envelope(&ctx, 50 * 1024 * 1024, 4096, 30_001).is_err());
    }

    #[tokio::test]
    async fn dispatch_submit_with_context_no_envelope_succeeds() {
        let handler = test_handler();
        let params = submit_params("0000:01:00.0", "passthrough");
        let ctx = CallerContext::anonymous();
        let result = handler
            .dispatch_submit_with_context(Some(&params), &ctx)
            .await
            .expect("should succeed without envelope");
        assert!(result["job_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn dispatch_submit_with_context_envelope_allows() {
        let handler = test_handler();
        let params = submit_params("0000:01:00.0", "passthrough");
        let ctx = ctx_with(envelope(Some(100), None, None));
        let result = handler
            .dispatch_submit_with_context(Some(&params), &ctx)
            .await
            .expect("3-byte binary is well within 100 MB envelope");
        assert!(result["job_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn dispatch_submit_with_context_envelope_rejects_mem() {
        let handler = test_handler();
        let mut large_binary = vec![0u8; 2 * 1024 * 1024];
        large_binary[0] = 1;
        let params = serde_json::json!({
            "binary": large_binary,
            "bdf": "0000:01:00.0",
            "dispatch_mode": "passthrough",
        });
        let ctx = ctx_with(envelope(Some(1), None, None));
        let err = handler
            .dispatch_submit_with_context(Some(&params), &ctx)
            .await
            .unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
        );
    }

    #[tokio::test]
    async fn dispatch_submit_rejects_timeout_over_envelope() {
        let handler = test_handler();
        let params = serde_json::json!({
            "binary": [1, 2, 3],
            "bdf": "0000:01:00.0",
            "dispatch_mode": "passthrough",
            "timeout_ms": 60_000,
        });
        let ctx = ctx_with(envelope(None, None, Some(5_000)));
        let err = handler
            .dispatch_submit_with_context(Some(&params), &ctx)
            .await
            .unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
        );
        assert!(err.message.contains("timeout"));
    }

    #[tokio::test]
    async fn shader_dispatch_enforces_envelope() {
        let handler = test_handler();
        let mut large_binary = vec![0u8; 2 * 1024 * 1024];
        large_binary[0] = 1;
        let params = serde_json::json!({
            "binary": large_binary,
            "bdf": "0000:01:00.0",
            "dispatch_mode": "passthrough",
        });
        let ctx = ctx_with(envelope(Some(1), None, None));
        let err = handler
            .shader_dispatch_with_context(Some(&params), &ctx)
            .await
            .unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
        );
    }
}
