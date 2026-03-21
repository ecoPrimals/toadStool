// SPDX-License-Identifier: AGPL-3.0-only

use super::super::job::JobHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

type JsonRpcResult = Result<serde_json::Value, JsonRpcError>;

pub(crate) async fn science_npu_dispatch(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_submit(params).await
}

#[allow(clippy::unused_async)] // async for JSON-RPC handler consistency
pub(crate) async fn science_npu_capabilities() -> JsonRpcResult {
    Ok(serde_json::json!({
        "available": false,
        "domain": "science",
        "supported_models": [],
        "note": "NPU capabilities discovered at runtime via NpuDispatch trait",
    }))
}
