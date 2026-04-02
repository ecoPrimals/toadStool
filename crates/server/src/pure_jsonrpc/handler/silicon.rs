// SPDX-License-Identifier: AGPL-3.0-only

//! Performance surface and multi-unit routing JSON-RPC handlers.
//!
//! Springs report measured throughput for `(operation, silicon_unit, precision)`
//! triples. toadStool stores these and uses them for tolerance-based routing
//! across all silicon units on the GPU die.

use tokio::sync::RwLock;

use toadstool_core::silicon::{
    MultiUnitRoutingPlan, PerformanceMeasurement, PerformanceSurfaceEntry, RoutedOperation,
    SiliconUnit,
};

use crate::pure_jsonrpc::types::JsonRpcError;

/// Handler for `compute.performance_surface.*` JSON-RPC methods.
///
/// Uses `tokio::sync::RwLock` so lock acquisition is async-safe and
/// cannot block the runtime under contention.
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

        let measurement: PerformanceMeasurement = serde_json::from_value(params.clone())
            .map_err(|e| JsonRpcError::invalid_params(format!("invalid measurement: {e}")))?;

        let unit_name = measurement.silicon_unit.as_str().to_string();
        let op_name = measurement.operation.clone();

        let mut store = self.measurements.write().await;

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

        let store = self.measurements.read().await;

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

        let best = match matching.iter().max_by(|a, b| {
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
            fallback_unit: fallback.map_or(SiliconUnit::ShaderCore, |f| f.silicon_unit),
            fallback_throughput_gflops: fallback.map_or(0.0, |f| f.throughput_gflops),
        };

        serde_json::to_value(&entry)
            .map_err(|e| JsonRpcError::internal_error(format!("serialize: {e}")))
    }

    /// `compute.performance_surface.list` — list all measurements and available operations.
    pub async fn list(&self) -> Result<serde_json::Value, JsonRpcError> {
        let store = self.measurements.read().await;

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

    /// `compute.route.multi_unit` — build a routing plan for a compound workload.
    ///
    /// Each operation in the workload specifies a tolerance. The routing engine
    /// consults the performance surface to find the highest-throughput unit
    /// that meets tolerance, with shader-core fallback for every decision.
    pub async fn route_multi_unit(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("missing params"))?;

        let workload = params
            .get("workload")
            .and_then(|v| v.as_array())
            .ok_or_else(|| JsonRpcError::invalid_params("missing 'workload' array"))?;

        if workload.is_empty() {
            return Err(JsonRpcError::invalid_params("'workload' array is empty"));
        }

        let gpu_target = params
            .get("gpu")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let store = self.measurements.read().await;

        let mut routed_ops = Vec::with_capacity(workload.len());
        let mut total_throughput = 0.0_f64;

        for item in workload {
            let op = item
                .get("op")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError::invalid_params("workload item missing 'op'"))?;

            let tolerance = item
                .get("tolerance")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| JsonRpcError::invalid_params("workload item missing 'tolerance'"))?;

            let routed = route_single_op(&store, op, tolerance);
            total_throughput += routed.estimated_throughput_gflops;
            routed_ops.push(routed);
        }

        let plan = MultiUnitRoutingPlan {
            operations: routed_ops,
            total_estimated_throughput_gflops: total_throughput,
            gpu_target: gpu_target.to_string(),
        };

        serde_json::to_value(&plan)
            .map_err(|e| JsonRpcError::internal_error(format!("serialize: {e}")))
    }
}

/// Route a single operation by consulting the performance surface.
///
/// Strategy:
/// 1. Find all measurements for this op where tolerance is met
/// 2. Pick the highest-throughput unit as primary
/// 3. Pick shader-core measurement as fallback (always available)
/// 4. If no measurements exist, fall back to heuristic defaults
fn route_single_op(store: &[PerformanceMeasurement], op: &str, tolerance: f64) -> RoutedOperation {
    let matching: Vec<&PerformanceMeasurement> = store
        .iter()
        .filter(|m| m.operation == op && m.tolerance_achieved <= tolerance)
        .collect();

    if matching.is_empty() {
        return route_heuristic(op, tolerance);
    }

    let Some(best) = matching.iter().max_by(|a, b| {
        a.throughput_gflops
            .partial_cmp(&b.throughput_gflops)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) else {
        return route_heuristic(op, tolerance);
    };

    let shader_fallback = matching
        .iter()
        .filter(|m| m.silicon_unit == SiliconUnit::ShaderCore)
        .max_by(|a, b| {
            a.throughput_gflops
                .partial_cmp(&b.throughput_gflops)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    let fallback = if best.silicon_unit != SiliconUnit::ShaderCore {
        Some(Box::new(RoutedOperation {
            operation: op.to_string(),
            silicon_unit: shader_fallback.map_or(SiliconUnit::ShaderCore, |f| f.silicon_unit),
            precision_mode: shader_fallback.map_or_else(
                || precision_for_tolerance(tolerance),
                |f| f.precision_mode.clone(),
            ),
            estimated_throughput_gflops: shader_fallback.map_or(0.0, |f| f.throughput_gflops),
            reason: String::from("shader core fallback"),
            fallback: None,
        }))
    } else {
        None
    };

    RoutedOperation {
        operation: op.to_string(),
        silicon_unit: best.silicon_unit,
        precision_mode: best.precision_mode.clone(),
        estimated_throughput_gflops: best.throughput_gflops,
        reason: format!(
            "{} at {} achieves {:.0} GFLOPS within tolerance {:.0e}",
            best.silicon_unit, best.precision_mode, best.throughput_gflops, tolerance
        ),
        fallback,
    }
}

/// Heuristic routing when no performance surface data exists for an operation.
///
/// Uses tolerance thresholds to pick reasonable defaults based on the
/// silicon budget table from specs/README.md.
fn route_heuristic(op: &str, tolerance: f64) -> RoutedOperation {
    let op_lower = op.to_lowercase();

    let (unit, precision, throughput_est, reason) = if op_lower.contains("neighbor")
        || op_lower.contains("spatial")
        || op_lower.contains("bvh")
    {
        (
            SiliconUnit::RtCore,
            "fp32",
            5400.0,
            "spatial query heuristic — RT cores for BVH traversal",
        )
    } else if op_lower.contains("histogram")
        || op_lower.contains("scatter")
        || op_lower.contains("deposit")
    {
        (
            SiliconUnit::Rop,
            "fp32",
            2700.0,
            "scatter/additive heuristic — ROPs for per-pixel atomic ops",
        )
    } else if op_lower.contains("lookup") || op_lower.contains("table") || op_lower.contains("eos")
    {
        (
            SiliconUnit::TextureUnit,
            "fp32",
            4000.0,
            "table lookup heuristic — TMUs for interpolated reads",
        )
    } else if op_lower.contains("matmul")
        || op_lower.contains("mma")
        || op_lower.contains("cg_solve")
    {
        if tolerance >= 1e-4 {
            (
                SiliconUnit::TensorCore,
                "fp16",
                142_000.0,
                "matrix heuristic — tensor cores at FP16 for loose tolerance",
            )
        } else {
            (
                SiliconUnit::ShaderCore,
                &*precision_for_tolerance(tolerance),
                throughput_for_tolerance(tolerance),
                "matrix heuristic — shader cores for tight tolerance",
            )
        }
    } else {
        (
            SiliconUnit::ShaderCore,
            &*precision_for_tolerance(tolerance),
            throughput_for_tolerance(tolerance),
            "default heuristic — shader cores",
        )
    };

    let fallback = if unit != SiliconUnit::ShaderCore {
        Some(Box::new(RoutedOperation {
            operation: op.to_string(),
            silicon_unit: SiliconUnit::ShaderCore,
            precision_mode: precision_for_tolerance(tolerance),
            estimated_throughput_gflops: throughput_for_tolerance(tolerance),
            reason: String::from("shader core fallback (heuristic)"),
            fallback: None,
        }))
    } else {
        None
    };

    RoutedOperation {
        operation: op.to_string(),
        silicon_unit: unit,
        precision_mode: precision.to_string(),
        estimated_throughput_gflops: throughput_est,
        reason: reason.to_string(),
        fallback,
    }
}

/// Select precision mode based on tolerance threshold.
fn precision_for_tolerance(tolerance: f64) -> String {
    if tolerance <= 1e-14 {
        String::from("df64")
    } else if tolerance <= 1e-7 {
        String::from("fp32")
    } else {
        String::from("fp16")
    }
}

/// Estimate shader-core throughput (GFLOPS) based on tolerance/precision.
///
/// Reference numbers from RTX 3090 silicon budget (specs/README.md).
fn throughput_for_tolerance(tolerance: f64) -> f64 {
    if tolerance <= 1e-14 {
        3240.0 // DF64 on FP32 ALUs
    } else if tolerance <= 1e-7 {
        35_580.0 // FP32 native
    } else {
        71_160.0 // FP16 on shader cores
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

    #[tokio::test]
    async fn route_multi_unit_with_surface_data() {
        let handler = SiliconHandler::new();

        handler
            .report(Some(&make_measurement(
                "neighbor_search",
                SiliconUnit::RtCore,
                5400.0,
                1e-3,
            )))
            .await
            .unwrap();
        handler
            .report(Some(&make_measurement(
                "neighbor_search",
                SiliconUnit::ShaderCore,
                540.0,
                1e-7,
            )))
            .await
            .unwrap();
        handler
            .report(Some(&make_measurement(
                "force_eval",
                SiliconUnit::ShaderCore,
                3240.0,
                1e-14,
            )))
            .await
            .unwrap();
        handler
            .report(Some(&serde_json::json!({
                "operation": "accumulation",
                "silicon_unit": "rop",
                "precision_mode": "fp32",
                "throughput_gflops": 2700.0,
                "tolerance_achieved": 1e-7,
                "gpu_model": "RTX 3090",
                "measured_by": "test",
                "timestamp": 1_710_000_000_u64
            })))
            .await
            .unwrap();

        let params = serde_json::json!({
            "workload": [
                { "op": "neighbor_search", "tolerance": 1e-2 },
                { "op": "force_eval", "tolerance": 1e-14 },
                { "op": "accumulation", "tolerance": 1e-7 }
            ],
            "gpu": "RTX 3090"
        });

        let result = handler.route_multi_unit(Some(&params)).await.unwrap();

        assert_eq!(result["gpu_target"], "RTX 3090");
        let ops = result["operations"].as_array().unwrap();
        assert_eq!(ops.len(), 3);

        assert_eq!(ops[0]["silicon_unit"], "rt_core");
        assert_eq!(ops[0]["operation"], "neighbor_search");
        assert!(ops[0]["fallback"].is_object());
        assert_eq!(ops[0]["fallback"]["silicon_unit"], "shader_core");

        assert_eq!(ops[1]["silicon_unit"], "shader_core");
        assert_eq!(ops[1]["operation"], "force_eval");

        assert_eq!(ops[2]["silicon_unit"], "rop");
        assert_eq!(ops[2]["operation"], "accumulation");

        let total = result["total_estimated_throughput_gflops"]
            .as_f64()
            .unwrap();
        assert!(total > 10_000.0);
    }

    #[tokio::test]
    async fn route_multi_unit_heuristic_fallback() {
        let handler = SiliconHandler::new();

        let params = serde_json::json!({
            "workload": [
                { "op": "neighbor_search", "tolerance": 1e-2 },
                { "op": "unknown_op", "tolerance": 1e-7 }
            ]
        });

        let result = handler.route_multi_unit(Some(&params)).await.unwrap();
        let ops = result["operations"].as_array().unwrap();

        assert_eq!(ops[0]["silicon_unit"], "rt_core");
        assert!(ops[0]["reason"].as_str().unwrap().contains("heuristic"));

        assert_eq!(ops[1]["silicon_unit"], "shader_core");
        assert!(ops[1]["reason"].as_str().unwrap().contains("heuristic"));
    }

    #[tokio::test]
    async fn route_multi_unit_empty_workload() {
        let handler = SiliconHandler::new();
        let params = serde_json::json!({ "workload": [] });
        let err = handler.route_multi_unit(Some(&params)).await.unwrap_err();
        assert_eq!(err.code, -32602);
    }

    #[tokio::test]
    async fn route_multi_unit_missing_params() {
        let handler = SiliconHandler::new();
        let err = handler.route_multi_unit(None).await.unwrap_err();
        assert_eq!(err.code, -32602);
    }
}
