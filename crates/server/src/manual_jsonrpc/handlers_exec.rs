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

        let priority = params.get("priority").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let vram_hint = params
            .get("vram_required_mb")
            .and_then(|v| v.as_u64())
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
