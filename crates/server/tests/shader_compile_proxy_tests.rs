// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Tests for shader.compile.* JSON-RPC handlers.
//!
//! Validates parameter validation, coralReef fallback behavior, and
//! dynamic capability reporting.

use std::borrow::Cow;
use std::sync::Arc;

fn test_handler() -> toadstool_server::pure_jsonrpc::JsonRpcHandler {
    let executor = Arc::new(toadstool_server::tarpc_server::StandaloneExecutor::new());
    toadstool_server::pure_jsonrpc::JsonRpcHandler::new(executor, "test-1.0.0".to_string(), None)
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

#[tokio::test]
async fn test_shader_compile_wgsl_missing_source() {
    let handler = test_handler();
    let request = mk_request("shader.compile.wgsl", Some(serde_json::json!({})));
    let response = handler.handle_request(&request).await;
    assert!(
        response.error.is_some(),
        "Should error when 'source' is missing"
    );
}

#[tokio::test]
async fn test_shader_compile_wgsl_empty_source() {
    let handler = test_handler();
    let request = mk_request(
        "shader.compile.wgsl",
        Some(serde_json::json!({"source": ""})),
    );
    let response = handler.handle_request(&request).await;
    assert!(
        response.error.is_some(),
        "Should error when source is empty"
    );
}

#[tokio::test]
async fn test_shader_compile_wgsl_fallback_without_coralreef() {
    let handler = test_handler();
    let request = mk_request(
        "shader.compile.wgsl",
        Some(serde_json::json!({"source": "@compute @workgroup_size(64) fn main() {}"})),
    );
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none(), "Valid WGSL should succeed");
    let result = response.result.expect("result present");
    assert_eq!(result["pipeline"], "naga_wgsl_to_spirv");
    assert_eq!(result["native_compiler_available"], false);
}

#[tokio::test]
async fn test_shader_compile_wgsl_with_arch_param() {
    let handler = test_handler();
    let request = mk_request(
        "shader.compile.wgsl",
        Some(serde_json::json!({
            "source": "fn main() {}",
            "arch": "sm70",
            "opt_level": 2
        })),
    );
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["source_language"], "wgsl");
}

#[tokio::test]
async fn test_shader_compile_spirv_missing_binary() {
    let handler = test_handler();
    let request = mk_request("shader.compile.spirv", Some(serde_json::json!({})));
    let response = handler.handle_request(&request).await;
    assert!(
        response.error.is_some(),
        "Should error when spirv_binary is missing"
    );
}

#[tokio::test]
async fn test_shader_compile_spirv_fallback_without_coralreef() {
    let handler = test_handler();
    let request = mk_request(
        "shader.compile.spirv",
        Some(serde_json::json!({"spirv_binary": [0x07230203, 0x00010000]})),
    );
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["pipeline"], "spirv_passthrough");
    assert_eq!(result["native_compiler_available"], false);
}

#[tokio::test]
async fn test_shader_compile_status_without_coralreef() {
    let handler = test_handler();
    let request = mk_request(
        "shader.compile.status",
        Some(serde_json::json!({"compile_id": "test-123"})),
    );
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["compile_id"], "test-123");
    assert_eq!(result["native_compiler_available"], false);
}

#[tokio::test]
async fn test_shader_compile_status_default_compile_id() {
    let handler = test_handler();
    let request = mk_request("shader.compile.status", None);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["compile_id"], "unknown");
}

#[tokio::test]
async fn test_shader_compile_capabilities_without_coralreef() {
    let handler = test_handler();
    let request = mk_request("shader.compile.capabilities", None);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["native_compiler_available"], false);
    assert_eq!(result["native_binary_compilation"], false);
    assert_eq!(result["naga_pipeline"], true);
    assert_eq!(result["domain"], "shader");
    let source_langs = result["source_languages"].as_array().expect("array");
    assert!(source_langs.iter().any(|v| v == "wgsl"));
}

#[tokio::test]
async fn test_shader_compile_capabilities_supported_archs_empty_without_coralreef() {
    let handler = test_handler();
    let request = mk_request("shader.compile.capabilities", None);
    let response = handler.handle_request(&request).await;
    let result = response.result.expect("result present");
    let archs = result["supported_archs"].as_array().expect("array");
    assert!(archs.is_empty(), "No supported archs without coralReef");
}

#[tokio::test]
async fn test_provenance_method_returns_data() {
    let handler = test_handler();
    let request = mk_request("toadstool.provenance", None);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none(), "Provenance should succeed");
    let result = response.result.expect("result present");
    assert!(result["total_flows"].as_u64().unwrap() >= 15);
    assert!(result["springs"].as_array().unwrap().len() >= 6);
    assert!(!result["flows"].as_array().unwrap().is_empty());
    assert!(!result["matrix"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_provenance_via_semantic_resolution() {
    let handler = test_handler();
    let request = mk_request("toadstool.provenance", None);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    let springs: Vec<&str> = result["springs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(springs.contains(&"hotSpring"));
    assert!(springs.contains(&"wetSpring"));
    assert!(springs.contains(&"neuralSpring"));
    assert!(springs.contains(&"airSpring"));
    assert!(springs.contains(&"groundSpring"));
}
