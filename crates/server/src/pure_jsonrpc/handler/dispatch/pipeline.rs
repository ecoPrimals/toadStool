// SPDX-License-Identifier: AGPL-3.0-or-later
//! `compute.dispatch.pipeline.*` — ordered multi-stage dispatch for ML inference
//! and other DAG-structured compute workloads.
//!
//! Resolves primalSpring upstream gap: neuralSpring needs ordered multi-stage
//! dispatch (tokenize → attention → FFN) over IPC. This handler accepts a DAG
//! of stages, validates the graph, and executes stages in topological order —
//! feeding each stage's result into downstream stages via `previous_results`.

use std::collections::HashMap;

use super::DispatchHandler;
use super::dag::{parse_edges, topological_sort};
use super::types::{PipelineJob, PipelineStageRequest, PipelineStageResult, PipelineStatus};
use crate::pure_jsonrpc::types::JsonRpcError;
use std::sync::atomic::Ordering;

impl DispatchHandler {
    /// Handle `compute.dispatch.pipeline.submit`.
    ///
    /// Accepts a named pipeline with ordered stages and dependency edges,
    /// validates the DAG, then executes stages in topological order.
    ///
    /// # Params
    ///
    /// ```json
    /// {
    ///   "name": "inference_pipeline",
    ///   "stages": [
    ///     { "id": "tokenize", "method": "compute.dispatch.submit", "params": {...}, "substrate": "gpu_preferred" },
    ///     { "id": "attention", "method": "compute.dispatch.submit", "params": {...}, "substrate": "gpu_only" },
    ///     { "id": "ffn", "method": "compute.dispatch.submit", "params": {...}, "substrate": "gpu_only" }
    ///   ],
    ///   "edges": [["tokenize", "attention"], ["attention", "ffn"]]
    /// }
    /// ```
    pub async fn pipeline_submit(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params(
                "Expected { name, stages: [...], edges: [[from, to], ...] }",
            )
        })?;

        let name = p
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unnamed_pipeline")
            .to_string();

        let stages: Vec<PipelineStageRequest> = p
            .get("stages")
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'stages' array"))
            .and_then(|v| {
                serde_json::from_value(v.clone())
                    .map_err(|e| JsonRpcError::invalid_params(format!("Invalid stages: {e}")))
            })?;

        if stages.is_empty() {
            return Err(JsonRpcError::invalid_params(
                "Pipeline must have at least one stage",
            ));
        }

        let edges = parse_edges(p)?;
        let execution_order = match topological_sort(&stages, &edges) {
            Ok(order) => order,
            Err(graph_err) => {
                let pipeline_id = uuid::Uuid::new_v4().to_string();
                let pipeline_job = PipelineJob {
                    id: pipeline_id.clone(),
                    name: name.clone(),
                    status: PipelineStatus::Failed(graph_err.message.to_string()),
                    submitted_at: std::time::Instant::now(),
                    stage_count: stages.len(),
                    stages_completed: 0,
                    stage_results: Vec::new(),
                };
                let mut pipelines = self.pipelines.write().await;
                pipelines.insert(pipeline_id.clone(), pipeline_job);
                return Ok(serde_json::json!({
                    "domain": "compute.dispatch",
                    "operation": "pipeline.submit",
                    "job_id": pipeline_id,
                    "status": "failed",
                    "output": null,
                    "error": graph_err.message.as_ref(),
                    "metadata": {
                        "name": name,
                        "stage_count": stages.len(),
                    },
                }));
            }
        };

        let pipeline_id = uuid::Uuid::new_v4().to_string();
        let pipeline_job = PipelineJob {
            id: pipeline_id.clone(),
            name: name.clone(),
            status: PipelineStatus::Submitted,
            submitted_at: std::time::Instant::now(),
            stage_count: stages.len(),
            stages_completed: 0,
            stage_results: Vec::with_capacity(stages.len()),
        };

        {
            let mut pipelines = self.pipelines.write().await;
            pipelines.insert(pipeline_id.clone(), pipeline_job);
        }

        let stage_map: HashMap<&str, &PipelineStageRequest> =
            stages.iter().map(|s| (s.id.as_str(), s)).collect();

        let mut stage_results: Vec<PipelineStageResult> = Vec::with_capacity(stages.len());
        let mut completed_results: HashMap<String, serde_json::Value> = HashMap::new();

        for stage_id in &execution_order {
            let stage = stage_map[stage_id.as_str()];

            {
                let mut pipelines = self.pipelines.write().await;
                if let Some(pj) = pipelines.get_mut(&pipeline_id) {
                    pj.status = PipelineStatus::Running {
                        current_stage: stage_id.clone(),
                    };
                }
            }

            let mut stage_params = stage.params.clone();
            if !completed_results.is_empty()
                && let Some(obj) = stage_params.as_object_mut()
            {
                obj.insert(
                    "previous_results".to_string(),
                    serde_json::to_value(&completed_results).unwrap_or_default(),
                );
            }

            let start = std::time::Instant::now();
            let result = self
                .execute_stage_method(&stage.method, &stage_params)
                .await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(value) => {
                    completed_results.insert(stage_id.clone(), value.clone());
                    stage_results.push(PipelineStageResult {
                        stage_id: stage_id.clone(),
                        method: stage.method.clone(),
                        substrate: stage.substrate,
                        status: "completed".to_string(),
                        elapsed_ms,
                        result: Some(value),
                        error: None,
                    });
                }
                Err(e) => {
                    let error_msg = e.message.to_string();
                    stage_results.push(PipelineStageResult {
                        stage_id: stage_id.clone(),
                        method: stage.method.clone(),
                        substrate: stage.substrate,
                        status: "failed".to_string(),
                        elapsed_ms,
                        result: None,
                        error: Some(error_msg.clone()),
                    });

                    let completed = stage_results.iter().filter(|r| r.error.is_none()).count();

                    {
                        let mut pipelines = self.pipelines.write().await;
                        if let Some(pj) = pipelines.get_mut(&pipeline_id) {
                            pj.status = PipelineStatus::PartialFailure {
                                completed,
                                failed_stage: stage_id.clone(),
                                error: error_msg.clone(),
                            };
                            pj.stages_completed = completed;
                            pj.stage_results.clone_from(&stage_results);
                        }
                    }

                    self.dispatch_count.fetch_add(1, Ordering::Relaxed);

                    return Ok(serde_json::json!({
                        "domain": "compute.dispatch",
                        "operation": "pipeline.submit",
                        "job_id": pipeline_id,
                        "status": "partial_failure",
                        "output": { "stage_results": stage_results },
                        "error": error_msg,
                        "metadata": {
                            "name": name,
                            "stage_count": stages.len(),
                            "stages_completed": completed,
                            "failed_stage": stage_id,
                        },
                    }));
                }
            }
        }

        let total_elapsed_ms: u64 = stage_results.iter().map(|r| r.elapsed_ms).sum();

        {
            let mut pipelines = self.pipelines.write().await;
            if let Some(pj) = pipelines.get_mut(&pipeline_id) {
                pj.status = PipelineStatus::Completed;
                pj.stages_completed = stages.len();
                pj.stage_results.clone_from(&stage_results);
            }
        }

        self.dispatch_count.fetch_add(1, Ordering::Relaxed);

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "pipeline.submit",
            "job_id": pipeline_id,
            "status": "completed",
            "output": { "stage_results": stage_results },
            "error": null,
            "metadata": {
                "name": name,
                "stage_count": stages.len(),
                "stages_completed": stages.len(),
                "total_elapsed_ms": total_elapsed_ms,
            },
        }))
    }

    /// Handle `compute.dispatch.pipeline.status`.
    pub async fn pipeline_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let pipeline_id = params
            .and_then(|p| p.get("pipeline_id"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'pipeline_id'"))?;

        let pipelines = self.pipelines.read().await;
        let pj = pipelines.get(pipeline_id).ok_or_else(|| {
            JsonRpcError::internal_error(format!("Pipeline {pipeline_id} not found"))
        })?;

        let (status_str, error_str) = match &pj.status {
            PipelineStatus::Failed(msg) => ("failed", Some(msg.as_str())),
            PipelineStatus::PartialFailure { error, .. } => ("partial_failure", Some(error.as_str())),
            other => (other.as_str(), None),
        };

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "pipeline.status",
            "job_id": pipeline_id,
            "status": status_str,
            "output": { "stage_results": pj.stage_results },
            "error": error_str,
            "metadata": {
                "name": pj.name,
                "stage_count": pj.stage_count,
                "stages_completed": pj.stages_completed,
                "elapsed_ms": pj.submitted_at.elapsed().as_millis() as u64,
            },
        }))
    }

    /// Dispatch a stage to the appropriate internal handler by method name.
    async fn execute_stage_method(
        &self,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match method {
            "compute.dispatch.submit" => self.dispatch_submit(Some(params)).await,
            "shader.dispatch" => self.shader_dispatch(Some(params)).await,
            _ => Err(JsonRpcError::invalid_params(format!(
                "Unsupported pipeline stage method: {method} \
                 (supported: compute.dispatch.submit, shader.dispatch)"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn pipeline_submit_empty_stages_rejected() {
        let handler = super::super::DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
        );
        let params = serde_json::json!({
            "name": "empty",
            "stages": [],
            "edges": []
        });
        let err = handler.pipeline_submit(Some(&params)).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn pipeline_submit_single_stage_passthrough() {
        let handler = super::super::DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
        );
        let params = serde_json::json!({
            "name": "single_stage",
            "stages": [{
                "id": "dispatch",
                "method": "compute.dispatch.submit",
                "params": {
                    "binary": [1, 2, 3],
                    "bdf": "0000:03:00.0",
                    "dispatch_mode": "passthrough"
                }
            }]
        });
        let result = handler.pipeline_submit(Some(&params)).await.unwrap();
        assert_eq!(result["domain"], "compute.dispatch");
        assert_eq!(result["operation"], "pipeline.submit");
        assert!(result["job_id"].as_str().is_some());
        assert_eq!(result["metadata"]["stage_count"], 1);
        assert_eq!(result["metadata"]["stages_completed"], 1);
    }

    #[tokio::test]
    async fn pipeline_submit_multi_stage_ordered() {
        let handler = super::super::DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
        );
        let params = serde_json::json!({
            "name": "inference_pipeline",
            "stages": [
                {
                    "id": "tokenize",
                    "method": "compute.dispatch.submit",
                    "params": {"binary": [1, 2], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"}
                },
                {
                    "id": "attention",
                    "method": "compute.dispatch.submit",
                    "params": {"binary": [3, 4], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"},
                    "substrate": "gpu_only"
                },
                {
                    "id": "ffn",
                    "method": "compute.dispatch.submit",
                    "params": {"binary": [5, 6], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"},
                    "substrate": "gpu_only"
                }
            ],
            "edges": [["tokenize", "attention"], ["attention", "ffn"]]
        });
        let result = handler.pipeline_submit(Some(&params)).await.unwrap();
        assert_eq!(result["status"], "completed");
        assert_eq!(result["metadata"]["stage_count"], 3);
        assert_eq!(result["metadata"]["stages_completed"], 3);

        let stage_results = result["output"]["stage_results"].as_array().unwrap();
        assert_eq!(stage_results.len(), 3);
        assert_eq!(stage_results[0]["stage_id"], "tokenize");
        assert_eq!(stage_results[1]["stage_id"], "attention");
        assert_eq!(stage_results[2]["stage_id"], "ffn");
    }

    #[tokio::test]
    async fn pipeline_status_returns_tracked_pipeline() {
        let handler = super::super::DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
        );
        let submit_params = serde_json::json!({
            "name": "tracked",
            "stages": [{
                "id": "s1",
                "method": "compute.dispatch.submit",
                "params": {"binary": [1], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"}
            }]
        });
        let submit_result = handler.pipeline_submit(Some(&submit_params)).await.unwrap();
        let pipeline_id = submit_result["job_id"].as_str().unwrap();

        let status_params = serde_json::json!({"pipeline_id": pipeline_id});
        let status = handler.pipeline_status(Some(&status_params)).await.unwrap();
        assert_eq!(status["domain"], "compute.dispatch");
        assert_eq!(status["operation"], "pipeline.status");
        assert_eq!(status["job_id"], pipeline_id);
        assert_eq!(status["status"], "completed");
    }

    #[tokio::test]
    async fn pipeline_status_not_found() {
        let handler = super::super::DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
        );
        let params = serde_json::json!({"pipeline_id": "nonexistent"});
        let err = handler.pipeline_status(Some(&params)).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn pipeline_submit_cycle_rejected() {
        let handler = super::super::DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
        );
        let params = serde_json::json!({
            "name": "cyclic",
            "stages": [
                {"id": "a", "method": "compute.dispatch.submit", "params": {"binary": [1], "bdf": "x", "dispatch_mode": "passthrough"}},
                {"id": "b", "method": "compute.dispatch.submit", "params": {"binary": [2], "bdf": "x", "dispatch_mode": "passthrough"}}
            ],
            "edges": [["a", "b"], ["b", "a"]]
        });
        let result = handler.pipeline_submit(Some(&params)).await.unwrap();
        assert_eq!(result["status"], "failed");
        assert!(result["error"].as_str().unwrap().contains("cycle"));
    }

    #[tokio::test]
    async fn pipeline_submit_unsupported_method_fails() {
        let handler = super::super::DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
        );
        let params = serde_json::json!({
            "name": "bad_method",
            "stages": [{
                "id": "s1",
                "method": "unknown.method",
                "params": {}
            }]
        });
        let result = handler.pipeline_submit(Some(&params)).await.unwrap();
        assert_eq!(result["status"], "partial_failure");
        let err = result["error"].as_str().unwrap();
        assert!(err.contains("Unsupported"));
    }

    #[tokio::test]
    async fn pipeline_submit_downstream_receives_previous_results() {
        let handler = super::super::DispatchHandler::new(
            crate::visualization_client::create_visualization_client(),
        );
        let params = serde_json::json!({
            "name": "chain",
            "stages": [
                {
                    "id": "first",
                    "method": "compute.dispatch.submit",
                    "params": {"binary": [1], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"}
                },
                {
                    "id": "second",
                    "method": "compute.dispatch.submit",
                    "params": {"binary": [2], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"}
                }
            ],
            "edges": [["first", "second"]]
        });
        let result = handler.pipeline_submit(Some(&params)).await.unwrap();
        assert_eq!(result["status"], "completed");
        assert_eq!(result["metadata"]["stages_completed"], 2);
    }
}
