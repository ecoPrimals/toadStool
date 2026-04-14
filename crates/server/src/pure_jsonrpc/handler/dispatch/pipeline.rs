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
#[path = "pipeline_tests.rs"]
mod tests;
