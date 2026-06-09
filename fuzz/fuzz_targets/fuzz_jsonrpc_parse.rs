// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fuzz target: JSON-RPC 2.0 request parsing.
//!
//! Feeds arbitrary bytes to `serde_json::from_slice` targeting the
//! JSON-RPC request shape. This exercises the deserialization path
//! that `process_request` uses before routing to handlers.
#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(serde::Deserialize)]
#[expect(dead_code, reason = "fields populated by serde Deserialize for fuzz validation")]
struct JsonRpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    #[serde(default)]
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<JsonRpcRequest<'_>>(data);

    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(data) {
        let _ = val.get("method");
        let _ = val.get("params");
    }
});
