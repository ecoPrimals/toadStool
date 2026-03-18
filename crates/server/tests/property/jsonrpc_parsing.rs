// SPDX-License-Identifier: AGPL-3.0-or-later
//! Property-based tests for JSON-RPC request parsing
//!
//! Tests that JSON-RPC requests can be parsed correctly regardless of
//! parameter format (object, array, or missing).

use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
    pub id: Option<Value>,
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_jsonrpc_request_with_object_params(
        method in "[a-z_]+",
        param_key in "[a-z]+",
        param_val in prop::collection::vec(any::<String>(), 0..5),
    ) {
        // Create request with object params
        let mut params_obj = serde_json::Map::new();
        for (i, val) in param_val.iter().enumerate() {
            params_obj.insert(format!("{}_{}", param_key, i), Value::String(val.clone()));
        }
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.clone(),
            params: Some(Value::Object(params_obj.clone())),
            id: Some(Value::Number(1.into())),
        };
        
        // Serialize
        let json = serde_json::to_string(&request).unwrap();
        
        // Parse back
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        
        // Verify properties
        prop_assert_eq!(parsed.jsonrpc, "2.0");
        prop_assert_eq!(parsed.method, method);
        prop_assert!(parsed.params.is_some());
        if let Some(Value::Object(obj)) = &parsed.params {
            prop_assert_eq!(obj.len(), params_obj.len());
        }
    }

    #[test]
    fn prop_jsonrpc_request_with_array_params(
        method in "[a-z_]+",
        param_count in 0usize..10,
    ) {
        // Create request with array params
        let params_array: Vec<Value> = (0..param_count)
            .map(|i| Value::String(format!("param_{}", i)))
            .collect();
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.clone(),
            params: Some(Value::Array(params_array.clone())),
            id: Some(Value::String("test_id".to_string())),
        };
        
        // Serialize
        let json = serde_json::to_string(&request).unwrap();
        
        // Parse back
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        
        // Verify properties
        prop_assert_eq!(parsed.jsonrpc, "2.0");
        prop_assert_eq!(parsed.method, method);
        prop_assert!(parsed.params.is_some());
        if let Some(Value::Array(arr)) = &parsed.params {
            prop_assert_eq!(arr.len(), params_array.len());
        }
    }

    #[test]
    fn prop_jsonrpc_request_without_params(
        method in "[a-z_]+",
    ) {
        // Create request without params
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.clone(),
            params: None,
            id: Some(Value::Number(42.into())),
        };
        
        // Serialize
        let json = serde_json::to_string(&request).unwrap();
        
        // Parse back
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        
        // Verify properties
        prop_assert_eq!(parsed.jsonrpc, "2.0");
        prop_assert_eq!(parsed.method, method);
        prop_assert!(parsed.params.is_none() || parsed.params.as_ref().map(|v| v.is_null()).unwrap_or(true));
    }

    #[test]
    fn prop_jsonrpc_request_id_variants(
        method in "[a-z_]+",
        id_num in 0i64..1000,
    ) {
        // Test with numeric ID
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.clone(),
            params: None,
            id: Some(Value::Number(id_num.into())),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(parsed.method, method);
        prop_assert!(parsed.id.is_some());
    }

    #[test]
    fn prop_jsonrpc_request_string_id(
        method in "[a-z_]+",
        id_str in "[a-zA-Z0-9_]+",
    ) {
        // Test with string ID
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.clone(),
            params: None,
            id: Some(Value::String(id_str.clone())),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(parsed.method, method);
        prop_assert!(parsed.id.is_some());
        if let Some(Value::String(s)) = &parsed.id {
            prop_assert_eq!(s, &id_str);
        }
    }
}
