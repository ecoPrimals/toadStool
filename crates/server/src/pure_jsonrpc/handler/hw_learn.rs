// SPDX-License-Identifier: AGPL-3.0-only
//! Hardware learning JSON-RPC handlers.
//!
//! Exposes the hw-learn pipeline (observe → distill → apply → share) as
//! JSON-RPC methods under the `compute.hardware.*` domain, matching the
//! biomeOS v2.30 capability registration.

use crate::pure_jsonrpc::types::JsonRpcError;

/// Handler for `compute.hardware.*` JSON-RPC methods.
///
/// Wraps the hw-learn pipeline and nvpmu firmware inventory.
/// All methods are stateless — the `RecipeStore` persists recipes to disk.
pub struct HwLearnHandler {
    recipe_store: hw_learn::RecipeStore,
}

impl HwLearnHandler {
    pub fn new() -> Self {
        Self {
            recipe_store: hw_learn::RecipeStore::default_location(),
        }
    }

    /// `compute.hardware.observe` — Parse an mmiotrace into structured MMIO accesses.
    ///
    /// Params: `{ "trace_data": "<mmiotrace text>" }`
    /// Returns: `{ "accesses": [...], "base_address": ... }`
    #[expect(clippy::unused_async, reason = "async for JSON-RPC handler trait consistency")]
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

        let trace = hw_learn::MmioTrace::parse(trace_data.as_bytes()).map_err(|e| {
            JsonRpcError::invalid_params(format!("Failed to parse mmiotrace: {e}"))
        })?;

        Ok(serde_json::json!({
            "domain": "compute.hardware",
            "operation": "observe",
            "accesses_count": trace.accesses.len(),
            "base_address": trace.base_address,
            "trace": serde_json::to_value(&trace).unwrap_or_default(),
        }))
    }

    /// `compute.hardware.distill` — Diff baseline vs compute traces, build init recipe.
    ///
    /// Params: `{ "baseline": "<mmiotrace>", "compute": "<mmiotrace>", "chip": "gv100", "base_address": 0 }`
    /// Returns: `{ "recipe": {...}, "diff_count": N }`
    #[expect(clippy::unused_async, reason = "async for JSON-RPC handler trait consistency")]
    pub async fn hw_learn_distill(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params(
                "Expected { baseline, compute, chip, base_address } parameters",
            )
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

        let base_address = p
            .get("base_address")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);

        let baseline = hw_learn::MmioTrace::parse(baseline_text.as_bytes()).map_err(|e| {
            JsonRpcError::invalid_params(format!("Failed to parse baseline trace: {e}"))
        })?;
        let compute = hw_learn::MmioTrace::parse(compute_text.as_bytes()).map_err(|e| {
            JsonRpcError::invalid_params(format!("Failed to parse compute trace: {e}"))
        })?;
        let diff = hw_learn::distiller::diff_traces(&baseline, &compute);
        let recipe = hw_learn::distiller::build_recipe(chip, &diff, base_address);

        Ok(serde_json::json!({
            "domain": "compute.hardware",
            "operation": "distill",
            "chip": chip,
            "diff_count": diff.len(),
            "recipe_steps": recipe.len(),
            "recipe": serde_json::to_value(&recipe).unwrap_or_default(),
        }))
    }

    /// `compute.hardware.apply` — Dry-run a recipe (BAR0 apply requires root + explicit opt-in).
    ///
    /// Params: `{ "recipe_json": "..." }` or `{ "chip": "gv100" }` (loads from store)
    /// Returns: `{ "result": {...} }`
    #[expect(clippy::unused_async, reason = "async for JSON-RPC handler trait consistency")]
    pub async fn hw_learn_apply(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params("Expected { recipe_json } or { chip } parameter")
        })?;

        let recipe = if let Some(json_str) = p.get("recipe_json").and_then(serde_json::Value::as_str)
        {
            hw_learn::distiller::InitRecipe::from_json(json_str).map_err(|e| {
                JsonRpcError::invalid_params(format!("Invalid recipe JSON: {e}"))
            })?
        } else if let Some(chip) = p.get("chip").and_then(serde_json::Value::as_str) {
            self.recipe_store.load(chip).map_err(|e| {
                JsonRpcError::invalid_params(format!("Failed to load recipe for {chip}: {e}"))
            })?
        } else {
            return Err(JsonRpcError::invalid_params(
                "Provide 'recipe_json' string or 'chip' name to load from store",
            ));
        };

        let result = hw_learn::applicator::dry_run(&recipe);

        Ok(serde_json::json!({
            "domain": "compute.hardware",
            "operation": "apply",
            "mode": "dry_run",
            "chip": recipe.chip,
            "success": result.success(),
            "steps_applied": result.steps_applied,
            "steps_failed": result.steps_failed,
            "verify_passed": result.verify_results.iter().all(|v| v.passed),
            "result": serde_json::to_value(&result).unwrap_or_default(),
            "note": "BAR0 live apply requires root privileges and explicit 'live: true' opt-in (not yet wired)",
        }))
    }

    /// `compute.hardware.share_recipe` — Save or load a recipe from the recipe store.
    ///
    /// Save: `{ "action": "save", "recipe_json": "..." }`
    /// Load: `{ "action": "load", "chip": "gv100" }`
    /// List: `{ "action": "list" }`
    #[expect(clippy::unused_async, reason = "async for JSON-RPC handler trait consistency")]
    pub async fn hw_learn_share_recipe(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params("Expected { action, ... } parameter")
        })?;

        let action = p
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("list");

        match action {
            "save" => {
                let json_str =
                    p.get("recipe_json")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            JsonRpcError::invalid_params("Missing 'recipe_json' for save action")
                        })?;
                let recipe =
                    hw_learn::distiller::InitRecipe::from_json(json_str).map_err(|e| {
                        JsonRpcError::invalid_params(format!("Invalid recipe JSON: {e}"))
                    })?;
                let path = self.recipe_store.save(&recipe).map_err(|e| {
                    JsonRpcError::internal_error(format!("Failed to save recipe: {e}"))
                })?;
                Ok(serde_json::json!({
                    "domain": "compute.hardware",
                    "operation": "share_recipe",
                    "action": "save",
                    "chip": recipe.chip,
                    "path": path.display().to_string(),
                }))
            }
            "load" => {
                let chip =
                    p.get("chip")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            JsonRpcError::invalid_params("Missing 'chip' for load action")
                        })?;
                let recipe = self.recipe_store.load(chip).map_err(|e| {
                    JsonRpcError::invalid_params(format!(
                        "Failed to load recipe for {chip}: {e}"
                    ))
                })?;
                Ok(serde_json::json!({
                    "domain": "compute.hardware",
                    "operation": "share_recipe",
                    "action": "load",
                    "chip": chip,
                    "recipe": serde_json::to_value(&recipe).unwrap_or_default(),
                }))
            }
            "list" => {
                let recipes = self.recipe_store.list().map_err(|e| {
                    JsonRpcError::internal_error(format!("Failed to list recipes: {e}"))
                })?;
                let entries: Vec<serde_json::Value> = recipes
                    .iter()
                    .map(|(chip, path)| {
                        serde_json::json!({
                            "chip": chip,
                            "path": path.display().to_string(),
                        })
                    })
                    .collect();
                Ok(serde_json::json!({
                    "domain": "compute.hardware",
                    "operation": "share_recipe",
                    "action": "list",
                    "recipes": entries,
                    "count": entries.len(),
                }))
            }
            _ => Err(JsonRpcError::invalid_params(format!(
                "Unknown action '{action}'. Expected 'save', 'load', or 'list'"
            ))),
        }
    }

    /// `compute.hardware.status` — Report hw-learn pipeline state and firmware inventory.
    ///
    /// Params: `{ "chip": "gv100" }` (optional — probes firmware for that chip)
    /// Returns: `{ "pipeline": ..., "firmware": ..., "recipes": ... }`
    #[expect(clippy::unused_async, reason = "async for JSON-RPC handler trait consistency")]
    pub async fn hw_learn_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let recipes = self.recipe_store.list().unwrap_or_default();
        let recipe_chips: Vec<&String> = recipes.keys().collect();

        let chip = params
            .and_then(|p| p.get("chip"))
            .and_then(serde_json::Value::as_str);

        let firmware = chip.map(|c| {
            let inv = nvpmu::FirmwareInventory::probe(c);
            serde_json::json!({
                "chip": c,
                "compute_viable": inv.compute_viable(),
                "compute_blockers": inv.compute_blockers(),
                "needs_software_pmu": inv.needs_software_pmu(),
                "inventory": serde_json::to_value(&inv).unwrap_or_default(),
            })
        });

        let sysmon_gpus = toadstool_sysmon::discover_gpus();
        let gpu_count = sysmon_gpus.len();

        Ok(serde_json::json!({
            "domain": "compute.hardware",
            "operation": "status",
            "pipeline": {
                "phase": "0 — trace-based (no live hardware access)",
                "stages": ["observe", "distill", "apply", "share_recipe"],
                "bar0_live_apply": false,
            },
            "recipes": {
                "stored_chips": recipe_chips,
                "count": recipe_chips.len(),
            },
            "firmware": firmware,
            "gpus_detected": gpu_count,
        }))
    }
}
