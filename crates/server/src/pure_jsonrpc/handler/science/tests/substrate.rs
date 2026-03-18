// SPDX-License-Identifier: AGPL-3.0-or-later

use super::common::{mk_request, test_handler};

#[tokio::test]
async fn science_substrate_discover_structure() {
    let handler = test_handler();
    let request = mk_request("science.substrate.discover", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    let substrates = result.get("substrates").expect("substrates");
    assert!(substrates.get("gpu").is_some());
    assert!(substrates.get("npu").is_some());
    assert!(substrates.get("cpu").is_some());
    assert_eq!(result["domain"], "science");
}

#[tokio::test]
async fn science_substrate_probe_with_capability() {
    let handler = test_handler();
    let params = serde_json::json!({ "capability": "f64_reductions" });
    let request = mk_request("science.substrate.probe", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["capability"], "f64_reductions");
    assert_eq!(result["available"], true);
    assert_eq!(result["domain"], "science");
}

#[tokio::test]
async fn science_substrate_probe_without_params_defaults_unknown() {
    let handler = test_handler();
    let request = mk_request("science.substrate.probe", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["capability"], "unknown");
    assert_eq!(result["available"], true);
}

#[tokio::test]
async fn science_substrate_probe_empty_params() {
    let handler = test_handler();
    let request = mk_request("science.substrate.probe", Some(serde_json::json!({})), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["capability"], "unknown");
}

#[tokio::test]
async fn science_substrate_probe_large_capability_name() {
    let handler = test_handler();
    let cap = "a".repeat(256);
    let params = serde_json::json!({ "capability": cap });
    let request = mk_request("science.substrate.probe", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["capability"], cap);
}
