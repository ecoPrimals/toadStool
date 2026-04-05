// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource estimation, validation, and optimization for JSON-RPC handler.

use crate::resource_estimator::ResourceEstimator;
use crate::resource_optimizer::ResourceOptimizer;
use crate::resource_validator::ResourceValidator;

use crate::pure_jsonrpc::types::JsonRpcError;

/// Handles resource workload operations (estimate, validate, optimize).
pub(super) struct ResourceHandler {
    pub(super) estimator: ResourceEstimator,
    pub(super) validator: ResourceValidator,
    pub(super) optimizer: ResourceOptimizer,
}

impl ResourceHandler {
    pub(super) fn new() -> Self {
        Self {
            estimator: ResourceEstimator::new(),
            validator: ResourceValidator::new(),
            optimizer: ResourceOptimizer::new(),
        }
    }

    pub(super) fn extract_graph(
        params: Option<&serde_json::Value>,
    ) -> Result<crate::graph_types::ExecutionGraph, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let graph_value = params
            .get("graph")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::from_value(graph_value)
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid graph parameter: {e}")))
    }

    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // JSON-RPC method dispatch; sync estimator.estimate()
    pub(super) async fn resources_estimate(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let graph = Self::extract_graph(params)?;
        self.estimator
            .estimate(&graph)
            .map_err(|e| JsonRpcError::internal_error(format!("Estimation failed: {e}")))
            .and_then(|estimate| {
                serde_json::to_value(estimate)
                    .map_err(|e| JsonRpcError::internal_error(format!("Serialization: {e}")))
            })
    }

    pub(super) async fn resources_validate_availability(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let graph = Self::extract_graph(params)?;
        self.validator
            .validate_availability(&graph)
            .await
            .map_err(|e| JsonRpcError::internal_error(format!("Validation failed: {e}")))
            .and_then(|result| {
                serde_json::to_value(result)
                    .map_err(|e| JsonRpcError::internal_error(format!("Serialization: {e}")))
            })
    }

    pub(super) async fn resources_suggest_optimizations(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let graph = Self::extract_graph(params)?;
        self.optimizer
            .suggest_optimizations(&graph)
            .await
            .map_err(|e| JsonRpcError::internal_error(format!("Optimization failed: {e}")))
            .and_then(|suggestions| {
                serde_json::to_value(suggestions)
                    .map_err(|e| JsonRpcError::internal_error(format!("Serialization: {e}")))
            })
    }
}
