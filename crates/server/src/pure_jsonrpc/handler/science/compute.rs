// SPDX-License-Identifier: AGPL-3.0-only

use super::super::job::JobHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

type JsonRpcResult = Result<serde_json::Value, JsonRpcError>;

pub(crate) async fn science_compute_submit(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_submit(params).await
}

pub(crate) async fn science_compute_status(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_status(params).await
}

pub(crate) async fn science_compute_result(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_result(params).await
}

pub(crate) async fn science_compute_cancel(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_cancel(params).await
}
