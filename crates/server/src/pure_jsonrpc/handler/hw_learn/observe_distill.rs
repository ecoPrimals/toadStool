// SPDX-License-Identifier: AGPL-3.0-only
//! Observe and distill handlers — parse mmiotraces, diff traces, build init recipes.

use super::HwLearnHandler;
use super::helpers::observe_from_text;
use crate::pure_jsonrpc::types::JsonRpcError;

impl HwLearnHandler {
    /// `compute.hardware.observe` — Parse an mmiotrace into structured events.
    ///
    /// Params: `{ "trace_data": "<mmiotrace text>", "mode": "mmiotrace" }`
    /// Returns: `{ "events_count": N, "gpu_id": ..., "driver": ... }`
    ///
    /// # Errors
    ///
    /// Returns an error if `trace_data` is missing or mmiotrace parsing fails.
    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    pub async fn hw_learn_observe(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let trace_data = params
            .and_then(|p| p.get("trace_data"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                JsonRpcError::invalid_params("Missing required 'trace_data' string parameter")
            })?;

        let result = observe_from_text(trace_data, "trace")?;

        Ok(serde_json::json!({
            "domain": "compute.hardware",
            "operation": "observe",
            "events_count": result.events.len(),
            "gpu_id": result.gpu_id,
            "driver": result.driver,
            "compute_triggered": result.compute_triggered,
            "duration_us": result.duration_us,
        }))
    }

    /// `compute.hardware.distill` — Diff baseline vs compute traces, build init recipe.
    ///
    /// Params: `{ "baseline": "<mmiotrace>", "compute": "<mmiotrace>", "chip": "gv100" }`
    /// Returns: `{ "recipe": {...}, "diff_count": N }`
    ///
    /// # Errors
    ///
    /// Returns an error if `baseline`, `compute`, or `chip` params are missing,
    /// or if mmiotrace parsing fails for either trace.
    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    pub async fn hw_learn_distill(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params("Expected { baseline, compute, chip } parameters")
        })?;

        let baseline_text = p
            .get("baseline")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'baseline' trace data"))?;

        let compute_text = p
            .get("compute")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'compute' trace data"))?;

        let chip = p
            .get("chip")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'chip' codename"))?;

        let baseline = observe_from_text(baseline_text, "baseline")?;
        let compute = observe_from_text(compute_text, "compute")?;

        let diff = hw_learn::distiller::diff_traces(&baseline.events, &compute.events);
        let target_arch = hw_learn::distiller::GpuArch {
            vendor: hw_learn::distiller::Vendor::Nvidia,
            generation: String::new(),
            chip: chip.to_string(),
            compute_class: String::new(),
        };
        let recipe =
            hw_learn::distiller::RecipeDistiller::distill(&compute, Some(&baseline), target_arch);

        Ok(serde_json::json!({
            "domain": "compute.hardware",
            "operation": "distill",
            "chip": chip,
            "diff_count": diff.len(),
            "recipe_steps": recipe.steps.len(),
            "recipe": serde_json::to_value(&recipe).unwrap_or_default(),
        }))
    }
}
