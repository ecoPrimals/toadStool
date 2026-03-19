// SPDX-License-Identifier: AGPL-3.0-or-later

//! Performance surface JSON-RPC handlers.
//!
//! Springs report measured throughput for `(operation, silicon_unit, precision)`
//! triples. toadStool stores these and uses them for tolerance-based routing.

use std::sync::RwLock;

use toadstool_core::silicon::{PerformanceMeasurement, PerformanceSurfaceEntry, SiliconUnit};

use crate::pure_jsonrpc::types::JsonRpcError;

/// Handler for `compute.performance_surface.*` JSON-RPC methods.
pub struct SiliconHandler {
    measurements: RwLock<Vec<PerformanceMeasurement>>,
}

impl SiliconHandler {
    /// Create a new silicon handler with an empty performance surface.
    pub fn new() -> Self {
        Self {
            measurements: RwLock::new(Vec::new()),
        }
    }

    /// `compute.performance_surface.report` — record a spring experiment measurement.
    pub async fn report(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("missing params"))?;

        let measurement: PerformanceMeasurement =
            serde_json::from_value(params.clone()).map_err(|e| {
                JsonRpcError::invalid_params(format!("invalid measurement: {e}"))
            })?;

        let unit_name = measurement.silicon_unit.as_str().to_string();
        let op_name = measurement.operation.clone();

        let mut store = self
            .measurements
            .write()
            .map_err(|_| JsonRpcError::internal_error("performance surface lock poisoned"))?;

        store.push(measurement);
        let total = store.len();

        Ok(serde_json::json!({
            "status": "recorded",
            "operation": op_name,
            "silicon_unit": unit_name,
            "total_measurements": total
        }))
    }

    /// `compute.performance_surface.query` — find the best unit for an operation+tolerance.
    pub async fn query(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("missing params"))?;

        let operation = params
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("missing 'operation'"))?;

        let tolerance = params
            .get("tolerance_required")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| JsonRpcError::invalid_params("missing 'tolerance_required'"))?;

        let store = self
            .measurements
            .read()
            .map_err(|_| JsonRpcError::internal_error("performance surface lock poisoned"))?;

        let matching: Vec<&PerformanceMeasurement> = store
            .iter()
            .filter(|m| m.operation == operation && m.tolerance_achieved <= tolerance)
            .collect();

        if matching.is_empty() {
            return Ok(serde_json::json!({
                "operation": operation,
                "tolerance_required": tolerance,
                "recommendation": null,
                "message": "no measurements found for this operation and tolerance"
            }));
        }

        let best = match matching
            .iter()
            .max_by(|a, b| {
                a.throughput_gflops
                    .partial_cmp(&b.throughput_gflops)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
            Some(m) => m,
            None => return Err(JsonRpcError::internal_error("no matching measurement")),
        };

        let fallback = matching
            .iter()
            .filter(|m| m.silicon_unit == SiliconUnit::ShaderCore)
            .max_by(|a, b| {
                a.throughput_gflops
                    .partial_cmp(&b.throughput_gflops)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let entry = PerformanceSurfaceEntry {
            operation: operation.to_string(),
            tolerance_required: tolerance,
            recommended_unit: best.silicon_unit,
            recommended_precision: best.precision_mode.clone(),
            estimated_throughput_gflops: best.throughput_gflops,
            fallback_unit: fallback
                .map_or(SiliconUnit::ShaderCore, |f| f.silicon_unit),
            fallback_throughput_gflops: fallback.map_or(0.0, |f| f.throughput_gflops),
        };

        serde_json::to_value(&entry)
            .map_err(|e| JsonRpcError::internal_error(format!("serialize: {e}")))
    }

    /// `compute.performance_surface.list` — list all measurements and available operations.
    pub async fn list(&self) -> Result<serde_json::Value, JsonRpcError> {
        let store = self
            .measurements
            .read()
            .map_err(|_| JsonRpcError::internal_error("performance surface lock poisoned"))?;

        let operations: Vec<&str> = {
            let mut ops: Vec<&str> = store.iter().map(|m| m.operation.as_str()).collect();
            ops.sort_unstable();
            ops.dedup();
            ops
        };

        let gpu_models: Vec<&str> = {
            let mut models: Vec<&str> = store.iter().map(|m| m.gpu_model.as_str()).collect();
            models.sort_unstable();
            models.dedup();
            models
        };

        let silicon_units: Vec<&str> = {
            let mut units: Vec<&str> = store.iter().map(|m| m.silicon_unit.as_str()).collect();
            units.sort_unstable();
            units.dedup();
            units
        };

        Ok(serde_json::json!({
            "total_measurements": store.len(),
            "operations": operations,
            "gpu_models": gpu_models,
            "silicon_units": silicon_units,
            "all_known_units": SiliconUnit::ALL.iter().map(|u| u.as_str()).collect::<Vec<_>>()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_measurement(op: &str, unit: SiliconUnit, tflops: f64, tol: f64) -> serde_json::Value {
        serde_json::json!({
            "operation": op,
            "silicon_unit": unit.as_str(),
            "precision_mode": "fp32",
            "throughput_gflops": tflops,
            "tolerance_achieved": tol,
            "gpu_model": "RTX 3090",
            "measured_by": "test",
            "timestamp": 1_710_000_000_u64
        })
    }

    #[tokio::test]
    async fn report_and_list() {
        let handler = SiliconHandler::new();
        let m = make_measurement("math.pairwise.yukawa", SiliconUnit::RtCore, 5400.0, 1e-7);
        let result = handler.report(Some(&m)).await.unwrap();
        assert_eq!(result["status"], "recorded");
        assert_eq!(result["total_measurements"], 1);

        let list = handler.list().await.unwrap();
        assert_eq!(list["total_measurements"], 1);
        assert_eq!(list["operations"][0], "math.pairwise.yukawa");
    }

    #[tokio::test]
    async fn query_finds_best_unit() {
        let handler = SiliconHandler::new();

        let m1 = make_measurement("neighbor_search", SiliconUnit::ShaderCore, 540.0, 1e-7);
        let m2 = make_measurement("neighbor_search", SiliconUnit::RtCore, 5400.0, 1e-3);

        handler.report(Some(&m1)).await.unwrap();
        handler.report(Some(&m2)).await.unwrap();

        let query = serde_json::json!({
            "operation": "neighbor_search",
            "tolerance_required": 1e-2
        });
        let result = handler.query(Some(&query)).await.unwrap();
        assert_eq!(result["recommended_unit"], "rt_core");
        assert_eq!(result["fallback_unit"], "shader_core");
    }

    #[tokio::test]
    async fn query_no_matches() {
        let handler = SiliconHandler::new();
        let query = serde_json::json!({
            "operation": "unknown_op",
            "tolerance_required": 1e-7
        });
        let result = handler.query(Some(&query)).await.unwrap();
        assert!(result["recommendation"].is_null());
    }

    #[tokio::test]
    async fn list_empty() {
        let handler = SiliconHandler::new();
        let result = handler.list().await.unwrap();
        assert_eq!(result["total_measurements"], 0);
        assert_eq!(result["all_known_units"].as_array().unwrap().len(), 9);
    }

    #[tokio::test]
    async fn report_missing_params() {
        let handler = SiliconHandler::new();
        let err = handler.report(None).await.unwrap_err();
        assert_eq!(err.code, -32602);
    }
}
