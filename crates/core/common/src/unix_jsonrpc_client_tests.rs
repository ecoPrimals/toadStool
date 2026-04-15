// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use proptest::prelude::*;

fn arb_jsonrpc_response_result() -> impl Strategy<Value = JsonRpcResponse<'static>> {
    (
        (1u64..10000u64),
        prop_oneof![
            any::<bool>().prop_map(|b| serde_json::json!(b)),
            (0i64..10000i64).prop_map(|n| serde_json::json!(n)),
            "[a-zA-Z0-9_ ]{0,100}".prop_map(|s| serde_json::json!(s)),
            prop::collection::vec(any::<i64>(), 0..5).prop_map(|v| serde_json::json!(v)),
            prop::collection::hash_map("[a-z]{1,10}", any::<i64>(), 0..5)
                .prop_map(|m| serde_json::json!(m)),
        ],
    )
        .prop_map(|(id, result)| JsonRpcResponse {
            jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
            id,
            result: Some(result),
            error: None,
        })
}

fn arb_jsonrpc_response_error() -> impl Strategy<Value = JsonRpcResponse<'static>> {
    (
        (1u64..10000u64),
        (-32768i32..0i32),
        "[a-zA-Z0-9 _-]{1,80}",
        prop::option::of(prop_oneof![
            any::<bool>().prop_map(|b| serde_json::json!(b)),
            (0i64..100i64).prop_map(|n| serde_json::json!(n)),
            "[a-z]{1,30}".prop_map(|s| serde_json::json!(s)),
        ]),
    )
        .prop_map(|(id, code, message, data)| JsonRpcResponse {
            jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: Cow::Owned(message),
                data,
            }),
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_jsonrpc_response_result_roundtrip(resp in arb_jsonrpc_response_result()) {
        let json = serde_json::to_string(&resp).unwrap();
        let restored: JsonRpcResponse<'_> = serde_json::from_slice(json.as_bytes()).unwrap();
        prop_assert_eq!(resp.id, restored.id);
        prop_assert!(restored.result.is_some());
        prop_assert!(restored.error.is_none());
    }

    #[test]
    fn prop_jsonrpc_response_error_roundtrip(resp in arb_jsonrpc_response_error()) {
        let json = serde_json::to_string(&resp).unwrap();
        let restored: JsonRpcResponse<'_> = serde_json::from_slice(json.as_bytes()).unwrap();
        prop_assert_eq!(resp.id, restored.id);
        prop_assert!(restored.result.is_none());
        prop_assert!(restored.error.is_some());
        let err = restored.error.unwrap();
        prop_assert_eq!(resp.error.as_ref().unwrap().code, err.code);
        prop_assert_eq!(resp.error.as_ref().unwrap().message.as_ref(), err.message.as_ref());
    }
}

#[test]
fn test_client_creation() {
    let client = UnixJsonRpcClient::new("/tmp/test.sock");
    assert_eq!(client.socket_path(), Path::new("/tmp/test.sock"));
}

#[test]
fn test_request_serialization() {
    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
        id: 1,
        method: Cow::Borrowed("test.method"),
        params: serde_json::json!({"key": "value"}),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"method\":\"test.method\""));
    assert!(json.contains("\"id\":1"));
}

#[test]
fn test_response_deserialization() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
    let response: JsonRpcResponse<'_> = serde_json::from_slice(json.as_bytes()).unwrap();

    assert_eq!(response.jsonrpc.as_ref(), "2.0");
    assert_eq!(response.id, 1);
    assert!(response.result.is_some());
    assert!(response.error.is_none());
}

#[test]
fn test_error_response_deserialization() {
    let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
    let response: JsonRpcResponse<'_> = serde_json::from_slice(json.as_bytes()).unwrap();

    assert!(response.result.is_none());
    assert!(response.error.is_some());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32_600);
    assert_eq!(error.message.as_ref(), "Invalid Request");
}

#[test]
fn test_request_with_empty_params() {
    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
        id: 42,
        method: Cow::Borrowed("simple.method"),
        params: serde_json::json!(null),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"id\":42"));
    assert!(json.contains("\"method\":\"simple.method\""));
}

#[test]
fn test_request_with_array_params() {
    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
        id: 1,
        method: Cow::Borrowed("test.array"),
        params: serde_json::json!([1, 2, 3]),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("[1,2,3]"));
}

#[test]
fn test_response_with_empty_object_result() {
    // Test with empty object instead of null (more realistic)
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
    let response: JsonRpcResponse<'_> = serde_json::from_slice(json.as_bytes()).unwrap();

    assert!(response.result.is_some());
    assert!(response.result.unwrap().is_object());
}

#[test]
fn test_error_with_data() {
    let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32603,"message":"Internal error","data":{"details":"stack trace here"}}}"#;
    let response: JsonRpcResponse<'_> = serde_json::from_slice(json.as_bytes()).unwrap();

    let error = response.error.unwrap();
    assert_eq!(error.code, -32_603);
    assert!(error.data.is_some());
    assert!(
        error.data.unwrap()["details"]
            .as_str()
            .unwrap()
            .contains("stack trace")
    );
}

#[test]
fn test_client_path_conversion() {
    // Test with &str
    let client1 = UnixJsonRpcClient::new("/tmp/test1.sock");
    assert_eq!(client1.socket_path(), Path::new("/tmp/test1.sock"));

    // Test with String
    let client2 = UnixJsonRpcClient::new("/tmp/test2.sock".to_string());
    assert_eq!(client2.socket_path(), Path::new("/tmp/test2.sock"));

    // Test with PathBuf
    let path = PathBuf::from("/tmp/test3.sock");
    let client3 = UnixJsonRpcClient::new(path);
    assert_eq!(client3.socket_path(), Path::new("/tmp/test3.sock"));
}

#[test]
fn test_client_clone() {
    let client1 = UnixJsonRpcClient::new("/tmp/original.sock");
    let client2 = client1.clone();

    assert_eq!(client1.socket_path(), client2.socket_path());
}

#[test]
fn test_request_id_increment() {
    let client = UnixJsonRpcClient::new("/tmp/test.sock");

    // Access the atomic counter
    let id1 = client
        .next_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let id2 = client
        .next_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let id3 = client
        .next_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // IDs should increment
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_jsonrpc_request_debug() {
    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
        id: 1,
        method: Cow::Borrowed("test"),
        params: serde_json::json!({}),
    };

    let debug_str = format!("{request:?}");
    assert!(debug_str.contains("JsonRpcRequest"));
    assert!(debug_str.contains("test"));
}

#[test]
fn test_jsonrpc_response_debug() {
    let response = JsonRpcResponse {
        jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
        id: 1,
        result: Some(serde_json::json!({"ok": true})),
        error: None,
    };

    let debug_str = format!("{response:?}");
    assert!(debug_str.contains("JsonRpcResponse"));
}

#[test]
fn test_jsonrpc_error_debug() {
    let error = JsonRpcError {
        code: -32_700,
        message: Cow::Borrowed("Parse error"),
        data: None,
    };

    let debug_str = format!("{error:?}");
    assert!(debug_str.contains("JsonRpcError"));
    assert!(debug_str.contains("-32700"));
}

#[test]
fn test_response_serialization_skips_none() {
    // Response with only result (no error)
    let response1 = JsonRpcResponse {
        jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
        id: 1,
        result: Some(serde_json::json!({"data": "value"})),
        error: None,
    };

    let json1 = serde_json::to_string(&response1).unwrap();
    assert!(!json1.contains("\"error\":"));

    // Response with only error (no result)
    let response2 = JsonRpcResponse {
        jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
        id: 2,
        result: None,
        error: Some(JsonRpcError {
            code: -32_600,
            message: Cow::Borrowed("Bad request"),
            data: None,
        }),
    };

    let json2 = serde_json::to_string(&response2).unwrap();
    assert!(!json2.contains("\"result\":"));
}

#[test]
fn test_error_without_data() {
    let error = JsonRpcError {
        code: -32_601,
        message: Cow::Borrowed("Method not found"),
        data: None,
    };

    let json = serde_json::to_string(&error).unwrap();
    assert!(!json.contains("\"data\":"));
    assert!(json.contains("\"code\":-32601"));
}

#[test]
fn test_client_debug() {
    let client = UnixJsonRpcClient::new("/tmp/debug.sock");
    let debug_str = format!("{client:?}");
    assert!(debug_str.contains("UnixJsonRpcClient"));
}
