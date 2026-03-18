// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::pure_jsonrpc::types::JsonRpcError;

type JsonRpcResult = Result<serde_json::Value, JsonRpcError>;

#[allow(clippy::unused_async)] // async for JSON-RPC handler consistency
pub(crate) async fn science_substrate_discover() -> JsonRpcResult {
    let gpu_info = crate::gpu_system::query_gpu_devices();
    Ok(serde_json::json!({
        "substrates": {
            "gpu": gpu_info,
            "npu": [],
            "cpu": { "available": true },
        },
        "domain": "science",
    }))
}

pub(crate) async fn science_substrate_probe(params: Option<&serde_json::Value>) -> JsonRpcResult {
    let capability = params
        .and_then(|p| p.get("capability"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");

    Ok(serde_json::json!({
        "capability": capability,
        "available": true,
        "domain": "science",
        "note": "Probe delegates to runtime substrate detection",
    }))
}
