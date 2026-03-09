// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Benchmark validation for shader compilation pipeline.
//!
//! Validates that the naga pipeline (local compilation path) handles
//! representative cross-spring WGSL shader patterns. Measures handler
//! response times to detect regressions.
//!
//! coralReef and barraCuda benchmarks are owned by their respective teams.

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Instant;

fn test_handler() -> toadstool_server::pure_jsonrpc::JsonRpcHandler {
    let executor = Arc::new(toadstool_server::tarpc_server::StandaloneExecutor::new());
    toadstool_server::pure_jsonrpc::JsonRpcHandler::new(executor, "bench-1.0.0".to_string(), None)
}

fn mk_request(
    method: &str,
    params: Option<serde_json::Value>,
) -> toadstool_server::pure_jsonrpc::JsonRpcRequest<'static> {
    toadstool_server::pure_jsonrpc::JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Owned(method.to_string()),
        params,
        id: Some(serde_json::json!(1)),
    }
}

/// Representative WGSL snippets from each spring domain for validation.
const CROSS_SPRING_WGSL_SAMPLES: &[(&str, &str)] = &[
    (
        "hotSpring-md",
        "@compute @workgroup_size(64) fn kinetic_energy(\
         @builtin(global_invocation_id) gid: vec3<u32>) { \
         var ke: f32 = 0.0; }",
    ),
    (
        "hotSpring-lattice",
        "@compute @workgroup_size(32) fn cg_update(\
         @builtin(global_invocation_id) gid: vec3<u32>) { \
         var r: f32 = 0.0; var p: f32 = 0.0; }",
    ),
    (
        "neuralSpring-attention",
        "@compute @workgroup_size(64) fn sdpa_scores(\
         @builtin(global_invocation_id) gid: vec3<u32>) { \
         var score: f32 = 0.0; }",
    ),
    (
        "wetSpring-bio",
        "@compute @workgroup_size(64) fn map_reduce(\
         @builtin(global_invocation_id) gid: vec3<u32>) { \
         var acc: f32 = 0.0; }",
    ),
    (
        "airSpring-hydro",
        "@compute @workgroup_size(64) fn hargreaves_et0(\
         @builtin(global_invocation_id) gid: vec3<u32>) { \
         var et0: f32 = 0.0; }",
    ),
    (
        "groundSpring-anderson",
        "@compute @workgroup_size(64) fn lyapunov(\
         @builtin(global_invocation_id) gid: vec3<u32>) { \
         var gamma: f32 = 0.0; }",
    ),
];

#[tokio::test]
async fn test_cross_spring_wgsl_samples_all_accepted() {
    let handler = test_handler();

    for (domain, source) in CROSS_SPRING_WGSL_SAMPLES {
        let request = mk_request(
            "shader.compile.wgsl",
            Some(serde_json::json!({"source": source})),
        );
        let response = handler.handle_request(&request).await;
        assert!(
            response.error.is_none(),
            "{domain} shader should be accepted: {:?}",
            response.error
        );
        let result = response.result.expect("result present");
        assert!(
            result["status"] == "accepted" || result["status"] == "compiled",
            "{domain} shader status should be accepted or compiled"
        );
    }
}

#[tokio::test]
async fn test_shader_compile_handler_response_time() {
    let handler = test_handler();

    for (domain, source) in CROSS_SPRING_WGSL_SAMPLES {
        let request = mk_request(
            "shader.compile.wgsl",
            Some(serde_json::json!({"source": source})),
        );

        let start = Instant::now();
        let response = handler.handle_request(&request).await;
        let elapsed = start.elapsed();

        assert!(response.error.is_none(), "{domain} should succeed");
        assert!(
            elapsed.as_millis() < 500,
            "{domain} handler took {}ms (should be <500ms for naga fallback)",
            elapsed.as_millis()
        );
    }
}

#[tokio::test]
async fn test_capabilities_handler_response_time() {
    let handler = test_handler();
    let request = mk_request("shader.compile.capabilities", None);

    let start = Instant::now();
    let response = handler.handle_request(&request).await;
    let elapsed = start.elapsed();

    assert!(response.error.is_none());
    assert!(
        elapsed.as_millis() < 1000,
        "Capabilities handler took {}ms (should be <1000ms)",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_provenance_handler_response_time() {
    let handler = test_handler();
    let request = mk_request("toadstool.provenance", None);

    let start = Instant::now();
    let response = handler.handle_request(&request).await;
    let elapsed = start.elapsed();

    assert!(response.error.is_none());
    assert!(
        elapsed.as_millis() < 50,
        "Provenance handler took {}ms (should be <50ms — pure data)",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_batch_compile_throughput() {
    let handler = test_handler();
    let source = "@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) gid: vec3<u32>) { var x: f32 = 0.0; }";

    let start = Instant::now();
    for _ in 0..50 {
        let request = mk_request(
            "shader.compile.wgsl",
            Some(serde_json::json!({"source": source})),
        );
        let response = handler.handle_request(&request).await;
        assert!(response.error.is_none());
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 2000,
        "50 compiles took {}ms (should be <2000ms)",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_spirv_compile_validation() {
    let handler = test_handler();
    let request = mk_request(
        "shader.compile.spirv",
        Some(serde_json::json!({
            "spirv_binary": [0x07230203_u32, 0x00010000, 0x00000000],
            "arch": "sm70"
        })),
    );
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["source_language"], "spirv");
}
