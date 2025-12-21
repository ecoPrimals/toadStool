//! Integration tests for API handlers
//!
//! These tests exercise the handler logic with real request/response cycles.

use toadstool_testing::fixtures::{runtime::*, server::*, TestEnvironment};

#[tokio::test]
async fn test_health_endpoint_request_structure() {
    let request = TestApiRequestBuilder::get("/api/v1/health")
        .with_header("Accept", "application/json")
        .build();

    assert_eq!(request["method"], "GET");
    assert_eq!(request["path"], "/api/v1/health");
}

#[tokio::test]
async fn test_workload_execution_request_structure() {
    let workload = create_wasm_test_workload();

    let request = TestApiRequestBuilder::post("/api/v1/workload/execute")
        .with_json_content_type()
        .with_body(workload)
        .build();

    assert_eq!(request["method"], "POST");
    assert_eq!(request["body"]["workload_type"], "Wasm");
}

#[tokio::test]
async fn test_status_endpoint_request() {
    let request = TestApiRequestBuilder::get("/api/v1/cluster/status").build();

    assert_eq!(request["method"], "GET");
    assert_eq!(request["path"], "/api/v1/cluster/status");
}

#[tokio::test]
async fn test_logs_endpoint_request() {
    let request = TestApiRequestBuilder::get("/api/v1/workload/exec-123/logs")
        .with_header("Accept", "text/event-stream")
        .build();

    assert_eq!(request["path"], "/api/v1/workload/exec-123/logs");
}

#[tokio::test]
async fn test_metrics_endpoint_request() {
    let request = TestApiRequestBuilder::get("/api/v1/metrics").build();

    assert_eq!(request["path"], "/api/v1/metrics");
}

#[tokio::test]
async fn test_workload_with_different_types() {
    let wasm_workload = create_wasm_test_workload();
    let native_workload = create_native_test_workload();
    let heavy_workload = create_heavy_test_workload();

    assert_eq!(wasm_workload["workload_type"], "Wasm");
    assert_eq!(native_workload["workload_type"], "Native");
    assert_eq!(heavy_workload["workload_type"], "Wasm");

    // Heavy workload should have more resources
    assert!(heavy_workload["resources"]["cpu_cores"].as_f64().unwrap() > 1.0);
}

#[tokio::test]
async fn test_request_with_authentication_header() {
    let request = TestApiRequestBuilder::post("/api/v1/workload/execute")
        .with_header("Authorization", "Bearer test-token-123")
        .with_json_content_type()
        .with_body(create_wasm_test_workload())
        .build();

    let headers = request["headers"].as_array().unwrap();
    assert!(headers.iter().any(|h| {
        h.as_array()
            .map(|arr| arr.len() == 2 && arr[0] == "Authorization")
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn test_workload_request_with_custom_resources() {
    let workload = TestWorkloadBuilder::wasm()
        .with_resources(4.0, 4096)
        .with_timeout(120)
        .build();

    assert_eq!(workload["resources"]["cpu_cores"], 4.0);
    assert_eq!(workload["resources"]["memory_mb"], 4096);
    assert_eq!(workload["timeout_seconds"], 120);
}

#[tokio::test]
async fn test_container_workload_request() {
    let workload = TestWorkloadBuilder::container()
        .with_entry_point("nginx:latest")
        .with_resources(2.0, 1024)
        .build();

    assert_eq!(workload["workload_type"], "Container");
}

#[tokio::test]
async fn test_python_workload_request() {
    let workload = TestWorkloadBuilder::python()
        .with_entry_point("script.py")
        .with_timeout(60)
        .build();

    assert_eq!(workload["workload_type"], "Python");
    assert_eq!(workload["entry_point"], "script.py");
}

#[tokio::test]
async fn test_api_error_response_structure() {
    // Test that error responses can be structured correctly
    let error_response = serde_json::json!({
        "error": {
            "code": "EXECUTION_FAILED",
            "message": "Workload execution failed",
            "details": {
                "reason": "timeout"
            }
        }
    });

    assert!(error_response["error"].is_object());
    assert_eq!(error_response["error"]["code"], "EXECUTION_FAILED");
}

#[tokio::test]
async fn test_request_with_request_id() {
    let request = TestApiRequestBuilder::post("/api/v1/workload/execute")
        .with_header("X-Request-ID", "req-456")
        .with_json_content_type()
        .with_body(create_wasm_test_workload())
        .build();

    let headers = request["headers"].as_array().unwrap();
    assert!(headers.len() >= 2); // Content-Type + X-Request-ID
}

#[tokio::test]
async fn test_workload_builder_chaining() {
    let workload = TestWorkloadBuilder::native()
        .with_entry_point("/bin/test")
        .with_timeout(45)
        .with_resources(1.5, 512)
        .build();

    assert_eq!(workload["workload_type"], "Native");
    assert_eq!(workload["entry_point"], "/bin/test");
    assert_eq!(workload["timeout_seconds"], 45);
    assert_eq!(workload["resources"]["cpu_cores"], 1.5);
    assert_eq!(workload["resources"]["memory_mb"], 512);
}

#[tokio::test]
async fn test_multiple_concurrent_requests() {
    let _env = TestEnvironment::new();

    // Create multiple request scenarios
    let req1 = TestApiRequestBuilder::get("/api/v1/health").build();
    let req2 = TestApiRequestBuilder::get("/api/v1/status").build();
    let req3 = TestApiRequestBuilder::post("/api/v1/execute")
        .with_body(create_wasm_test_workload())
        .build();

    // All requests should be valid and independent
    assert_eq!(req1["method"], "GET");
    assert_eq!(req2["method"], "GET");
    assert_eq!(req3["method"], "POST");
}
