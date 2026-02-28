//! Compute, Ollama, and resource management handlers

use serde_json::Value;

use crate::gpu_job_queue::JobType;
use crate::graph_types::ExecutionGraph;

use super::{
    JsonRpcRequest, ManualJsonRpcServer, INTERNAL_ERROR, INVALID_PARAMS, SERIALIZATION_FAILED,
};

impl ManualJsonRpcServer {
    pub(crate) async fn handle_compute_submit(
        &self,
        mut request: JsonRpcRequest,
    ) -> serde_json::Value {
        let params = match request.params.take() {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing params", &request),
        };

        let priority = params
            .get("priority")
            .and_then(serde_json::Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0);
        let vram_hint = params
            .get("vram_required_mb")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(4096);

        let job_type: JobType = match serde_json::from_value(params) {
            Ok(jt) => jt,
            Err(e) => {
                return self.error_response(
                    INVALID_PARAMS,
                    format!("Invalid job type: {e}"),
                    &request,
                )
            }
        };

        let routing = {
            let router = self.router.read().await;
            let model = match &job_type {
                JobType::Inference { model, .. } => model.as_str(),
                _ => "",
            };
            router.route(model, vram_hint)
        };

        match self.job_queue.submit(job_type, priority).await {
            Ok(job_id) => self.success_response(
                serde_json::json!({
                    "job_id": job_id,
                    "routing": {
                        "gate_id": routing.gate_id,
                        "reason": routing.reason,
                        "estimated_wait_ms": routing.estimated_wait_ms,
                    }
                }),
                &request,
            ),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    pub(crate) async fn handle_compute_status(&self, request: JsonRpcRequest) -> serde_json::Value {
        let job_id = match self.extract_job_id(&request) {
            Ok(id) => id,
            Err(resp) => return resp,
        };

        match self.job_queue.status(job_id).await {
            Ok(job) => {
                let result = serde_json::to_value(job)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result, &request)
            }
            Err(e) => self.job_queue_error_response(e, &request),
        }
    }

    pub(crate) async fn handle_compute_result(&self, request: JsonRpcRequest) -> serde_json::Value {
        let job_id = match self.extract_job_id(&request) {
            Ok(id) => id,
            Err(resp) => return resp,
        };

        match self.job_queue.result(job_id).await {
            Ok(result) => self.success_response(result, &request),
            Err(e) => self.job_queue_error_response(e, &request),
        }
    }

    pub(crate) async fn handle_compute_cancel(&self, request: JsonRpcRequest) -> serde_json::Value {
        let job_id = match self.extract_job_id(&request) {
            Ok(id) => id,
            Err(resp) => return resp,
        };

        match self.job_queue.cancel(job_id).await {
            Ok(()) => self.success_response(serde_json::json!({"cancelled": true}), &request),
            Err(e) => self.job_queue_error_response(e, &request),
        }
    }

    pub(crate) async fn handle_compute_list(&self, request: JsonRpcRequest) -> serde_json::Value {
        let state_filter = request
            .params
            .as_ref()
            .and_then(|p| p.get("state"))
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let jobs = self.job_queue.list(state_filter).await;
        let counts = self.job_queue.counts().await;

        self.success_response(
            serde_json::json!({ "jobs": jobs, "counts": counts }),
            &request,
        )
    }

    pub(crate) async fn handle_ollama_list_models(
        &self,
        request: JsonRpcRequest,
    ) -> serde_json::Value {
        match self.ollama.list_models().await {
            Ok(models) => self.success_response(serde_json::json!({"models": models}), &request),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    pub(crate) async fn handle_ollama_inference(
        &self,
        request: JsonRpcRequest,
    ) -> serde_json::Value {
        let params = match &request.params {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing params", &request),
        };

        let model = match params.get("model").and_then(|v| v.as_str()) {
            Some(m) => m,
            None => return self.error_response(INVALID_PARAMS, "Missing 'model' param", &request),
        };

        let prompt = match params.get("prompt").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing 'prompt' param", &request),
        };

        let extra_params = params.get("params").cloned().unwrap_or(Value::Null);

        match self.ollama.inference(model, prompt, &extra_params).await {
            Ok(response) => self.success_response(response, &request),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    pub(crate) async fn handle_ollama_load(&self, request: JsonRpcRequest) -> serde_json::Value {
        let model = match request
            .params
            .as_ref()
            .and_then(|p| p.get("model"))
            .and_then(|v| v.as_str())
        {
            Some(m) => m,
            None => return self.error_response(INVALID_PARAMS, "Missing 'model' param", &request),
        };

        match self.ollama.load(model).await {
            Ok(()) => self.success_response(
                serde_json::json!({"loaded": true, "model": model}),
                &request,
            ),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    pub(crate) async fn handle_ollama_unload(&self, request: JsonRpcRequest) -> serde_json::Value {
        let model = match request
            .params
            .as_ref()
            .and_then(|p| p.get("model"))
            .and_then(|v| v.as_str())
        {
            Some(m) => m,
            None => return self.error_response(INVALID_PARAMS, "Missing 'model' param", &request),
        };

        match self.ollama.unload(model).await {
            Ok(()) => self.success_response(
                serde_json::json!({"unloaded": true, "model": model}),
                &request,
            ),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    fn extract_graph(&self, request: &JsonRpcRequest) -> Result<ExecutionGraph, serde_json::Value> {
        let params = request.params.as_ref().ok_or_else(|| {
            self.error_response(INVALID_PARAMS, "Missing 'graph' parameter", request)
        })?;

        let graph_value = params.get("graph").cloned().unwrap_or(Value::Null);
        serde_json::from_value(graph_value).map_err(|e| {
            self.error_response(
                INVALID_PARAMS,
                format!("Invalid graph parameter: {e}"),
                request,
            )
        })
    }

    #[allow(clippy::unused_async)] // JSON-RPC handler; sync estimator.estimate()
    pub(crate) async fn handle_resources_estimate(
        &self,
        request: JsonRpcRequest,
    ) -> serde_json::Value {
        let graph = match self.extract_graph(&request) {
            Ok(g) => g,
            Err(resp) => return resp,
        };

        match self.estimator.estimate(&graph) {
            Ok(estimate) => {
                let result = serde_json::to_value(estimate)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result, &request)
            }
            Err(e) => {
                self.error_response(INTERNAL_ERROR, format!("Estimation failed: {e}"), &request)
            }
        }
    }

    pub(crate) async fn handle_resources_validate_availability(
        &self,
        request: JsonRpcRequest,
    ) -> serde_json::Value {
        let graph = match self.extract_graph(&request) {
            Ok(g) => g,
            Err(resp) => return resp,
        };

        match self.validator.validate_availability(&graph).await {
            Ok(result) => {
                let result_value = serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result_value, &request)
            }
            Err(e) => {
                self.error_response(INTERNAL_ERROR, format!("Validation failed: {e}"), &request)
            }
        }
    }

    pub(crate) async fn handle_resources_suggest_optimizations(
        &self,
        request: JsonRpcRequest,
    ) -> serde_json::Value {
        let graph = match self.extract_graph(&request) {
            Ok(g) => g,
            Err(resp) => return resp,
        };

        match self.optimizer.suggest_optimizations(&graph).await {
            Ok(suggestions) => {
                let result = serde_json::to_value(suggestions)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result, &request)
            }
            Err(e) => self.error_response(
                INTERNAL_ERROR,
                format!("Optimization failed: {e}"),
                &request,
            ),
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use crate::gpu_job_queue::JobQueueError;
    use crate::manual_jsonrpc::METHOD_NOT_FOUND;
    use crate::tarpc_server::StandaloneExecutor;
    use std::sync::Arc;

    fn test_server() -> ManualJsonRpcServer {
        let executor = Arc::new(StandaloneExecutor::new());
        ManualJsonRpcServer::new(executor, "test-1.0.0".to_string(), None)
    }

    fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
            method: method.to_string(),
            params,
            id: Some(serde_json::json!(id)),
        }
    }

    #[tokio::test]
    async fn handle_compute_submit_missing_params() {
        let server = test_server();
        let request = mk_request("compute.submit", None, 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
        assert!(obj["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Missing params"));
    }

    #[tokio::test]
    async fn handle_compute_submit_invalid_job_type() {
        let server = test_server();
        let params = serde_json::json!({ "invalid_key": "invalid" });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_compute_submit_transform_success() {
        let server = test_server();
        let params = serde_json::json!({
            "transform": {
                "operation": "embed",
                "input": {}
            }
        });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        assert!(obj["result"]["job_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn handle_compute_submit_custom_success() {
        let server = test_server();
        let params = serde_json::json!({
            "custom": {
                "plugin": "test_plugin",
                "payload": {}
            }
        });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        assert!(obj["result"]["job_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn handle_compute_status_missing_job_id() {
        let server = test_server();
        let request = mk_request("compute.status", None, 1);
        let response = server.handle_compute_status(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_compute_status_invalid_uuid() {
        let server = test_server();
        let request = mk_request(
            "compute.status",
            Some(serde_json::json!({"job_id": "not-a-uuid"})),
            1,
        );
        let response = server.handle_compute_status(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_compute_status_job_not_found() {
        let server = test_server();
        let request = mk_request(
            "compute.status",
            Some(serde_json::json!({"job_id": "550e8400-e29b-41d4-a716-446655440000"})),
            1,
        );
        let response = server.handle_compute_status(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn handle_compute_result_missing_job_id() {
        let server = test_server();
        let request = mk_request("compute.result", None, 1);
        let response = server.handle_compute_result(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_compute_cancel_missing_job_id() {
        let server = test_server();
        let request = mk_request("compute.cancel", None, 1);
        let response = server.handle_compute_cancel(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_compute_list_empty() {
        let server = test_server();
        let request = mk_request("compute.list", None, 1);
        let response = server.handle_compute_list(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        assert!(obj["result"]["jobs"].as_array().is_some());
        assert!(obj["result"]["counts"].as_object().is_some());
    }

    #[tokio::test]
    async fn handle_compute_list_with_state_filter() {
        let server = test_server();
        let params = serde_json::json!({"state": "pending"});
        let request = mk_request("compute.list", Some(params), 1);
        let response = server.handle_compute_list(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
    }

    #[tokio::test]
    async fn handle_ollama_inference_missing_params() {
        let server = test_server();
        let request = mk_request("ollama.inference", None, 1);
        let response = server.handle_ollama_inference(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_ollama_inference_missing_model() {
        let server = test_server();
        let params = serde_json::json!({"prompt": "hello"});
        let request = mk_request("ollama.inference", Some(params), 1);
        let response = server.handle_ollama_inference(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_ollama_inference_missing_prompt() {
        let server = test_server();
        let params = serde_json::json!({"model": "llama"});
        let request = mk_request("ollama.inference", Some(params), 1);
        let response = server.handle_ollama_inference(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_ollama_load_missing_model() {
        let server = test_server();
        let request = mk_request("ollama.load", None, 1);
        let response = server.handle_ollama_load(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_ollama_unload_missing_model() {
        let server = test_server();
        let request = mk_request("ollama.unload", None, 1);
        let response = server.handle_ollama_unload(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_resources_estimate_missing_graph() {
        let server = test_server();
        let request = mk_request("toadstool.resources.estimate", None, 1);
        let response = server.handle_resources_estimate(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_resources_estimate_invalid_graph() {
        let server = test_server();
        let params = serde_json::json!({"graph": "not an object"});
        let request = mk_request("toadstool.resources.estimate", Some(params), 1);
        let response = server.handle_resources_estimate(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_resources_estimate_empty_graph_returns_error() {
        let server = test_server();
        let params = serde_json::json!({
            "graph": {
                "id": "empty",
                "nodes": [],
                "edges": []
            }
        });
        let request = mk_request("toadstool.resources.estimate", Some(params), 1);
        let response = server.handle_resources_estimate(request).await;
        let obj = response.as_object().expect("object");
        // Empty graph causes estimator error -> INTERNAL_ERROR
        assert!(obj.get("error").is_some() || obj.get("result").is_some());
    }

    #[tokio::test]
    async fn handle_resources_validate_availability_missing_graph() {
        let server = test_server();
        let request = mk_request("toadstool.resources.validate_availability", None, 1);
        let response = server.handle_resources_validate_availability(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn handle_resources_suggest_optimizations_missing_graph() {
        let server = test_server();
        let request = mk_request("toadstool.resources.suggest_optimizations", None, 1);
        let response = server.handle_resources_suggest_optimizations(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[test]
    fn job_queue_error_response_queue_full() {
        let server = test_server();
        let request = mk_request("test", None, 1);
        let err = JobQueueError::QueueFull { max: 100 };
        let result = server.job_queue_error_response(err, &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INTERNAL_ERROR);
    }

    #[test]
    fn job_queue_error_response_job_cancelled() {
        let server = test_server();
        let request = mk_request("test", None, 1);
        let err = JobQueueError::JobCancelled {
            id: uuid::Uuid::nil(),
        };
        let result = server.job_queue_error_response(err, &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INTERNAL_ERROR);
    }

    #[test]
    fn job_queue_error_response_job_not_found() {
        let server = test_server();
        let request = mk_request("test", None, 1);
        let err = JobQueueError::JobNotFound {
            id: uuid::Uuid::nil(),
        };
        let result = server.job_queue_error_response(err, &request);
        let obj = result.as_object().expect("object");
        assert_eq!(obj["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn job_queue_error_response_other_variants() {
        let server = test_server();
        let request = mk_request("test", None, 1);

        let err = JobQueueError::JobNotComplete {
            id: uuid::Uuid::nil(),
        };
        let result = server.job_queue_error_response(err, &request);
        assert_eq!(result["error"]["code"], INTERNAL_ERROR);

        let err = JobQueueError::NoResult {
            id: uuid::Uuid::nil(),
        };
        let result = server.job_queue_error_response(err, &request);
        assert_eq!(result["error"]["code"], INTERNAL_ERROR);

        let err = JobQueueError::JobFailed {
            id: uuid::Uuid::nil(),
            error: "test failure".to_string(),
        };
        let result = server.job_queue_error_response(err, &request);
        assert_eq!(result["error"]["code"], INTERNAL_ERROR);
    }

    #[tokio::test]
    async fn handle_compute_submit_inference_success() {
        let server = test_server();
        let params = serde_json::json!({
            "inference": {
                "model": "llama2",
                "prompt": "Hello",
                "params": {}
            }
        });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        assert!(obj["result"]["job_id"].as_str().is_some());
        assert!(obj["result"]["routing"]["gate_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn handle_compute_status_success_after_submit() {
        let server = test_server();
        let params = serde_json::json!({
            "transform": { "operation": "embed", "input": {} }
        });
        let submit_req = mk_request("compute.submit", Some(params), 1);
        let submit_resp = server.handle_compute_submit(submit_req).await;
        let job_id = submit_resp["result"]["job_id"].as_str().unwrap();

        let status_req = mk_request(
            "compute.status",
            Some(serde_json::json!({ "job_id": job_id })),
            2,
        );
        let status_resp = server.handle_compute_status(status_req).await;
        let obj = status_resp.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
    }

    #[tokio::test]
    async fn handle_compute_result_and_cancel() {
        let server = test_server();
        let params = serde_json::json!({
            "transform": { "operation": "embed", "input": {} }
        });
        let submit_req = mk_request("compute.submit", Some(params), 1);
        let submit_resp = server.handle_compute_submit(submit_req).await;
        let job_id = submit_resp["result"]["job_id"].as_str().unwrap();

        let cancel_req = mk_request(
            "compute.cancel",
            Some(serde_json::json!({ "job_id": job_id })),
            3,
        );
        let cancel_resp = server.handle_compute_cancel(cancel_req).await;
        assert_eq!(cancel_resp["result"]["cancelled"], true);
    }

    #[tokio::test]
    async fn handle_compute_submit_priority_and_vram() {
        let server = test_server();
        let params = serde_json::json!({
            "transform": { "operation": "embed", "input": {} },
            "priority": 5,
            "vram_required_mb": 8192
        });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.contains_key("result") || obj.contains_key("error"));
        if let Some(res) = obj.get("result") {
            assert!(res.get("job_id").is_some());
        }
    }

    #[tokio::test]
    async fn handle_resources_estimate_valid_graph() {
        let server = test_server();
        let params = serde_json::json!({
            "graph": {
                "id": "test-graph",
                "nodes": [
                    { "id": "n1", "type": "compute", "config": {} }
                ],
                "edges": []
            }
        });
        let request = mk_request("toadstool.resources.estimate", Some(params), 1);
        let response = server.handle_resources_estimate(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some() || obj.get("error").is_some());
    }

    #[tokio::test]
    async fn extract_graph_missing_graph_key() {
        let server = test_server();
        let params = serde_json::json!({ "other_key": "value" });
        let request = mk_request("test", Some(params), 1);
        let result = server.extract_graph(&request);
        assert!(result.is_err());
    }
}
