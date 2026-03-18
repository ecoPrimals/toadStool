// SPDX-License-Identifier: AGPL-3.0-or-later

use std::borrow::Cow;
use std::sync::Arc;

use crate::pure_jsonrpc::handler::JsonRpcHandler;
use crate::tarpc_server::StandaloneExecutor;

pub(super) fn test_handler() -> JsonRpcHandler {
    let executor = Arc::new(StandaloneExecutor::new());
    JsonRpcHandler::new(executor, "test-1.0.0".to_string(), None)
}

pub(super) fn mk_request(
    method: &str,
    params: Option<serde_json::Value>,
    id: i32,
) -> crate::pure_jsonrpc::types::JsonRpcRequest<'static> {
    crate::pure_jsonrpc::types::JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Owned(method.to_string()),
        params,
        id: Some(serde_json::json!(id)),
    }
}
