// SPDX-License-Identifier: AGPL-3.0-only
//! Apply handler — apply recipes (dry-run or live BAR0).

use super::helpers::{check_thermal_for_bdf, resolve_bdf};
use super::HwLearnHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

impl HwLearnHandler {
    /// `compute.hardware.apply` — Apply a recipe (dry-run or live BAR0).
    ///
    /// Params: `{ "recipe_json": "..." }` or `{ "recipe_id": "..." }`
    ///         Optional: `{ "live": true, "bdf": "0000:65:00.0" }`
    ///
    /// When `live` is true and BAR0 is accessible, the recipe is applied
    /// directly to the GPU via MMIO register writes. Without `live`, the
    /// applicator performs a dry-run simulation.
    ///
    /// Returns: `{ "result": {...}, "mode": "live"|"dry_run" }`
    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    pub async fn hw_learn_apply(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params("Expected { recipe_json } or { recipe_id } parameter")
        })?;

        let recipe = if let Some(json_str) =
            p.get("recipe_json").and_then(serde_json::Value::as_str)
        {
            hw_learn::knowledge::import_recipe(json_str)
                .map_err(|e| JsonRpcError::invalid_params(format!("Invalid recipe JSON: {e}")))?
        } else if let Some(id) = p.get("recipe_id").and_then(serde_json::Value::as_str) {
            let store = self.open_store()?;
            store
                .load(id)
                .map_err(|e| {
                    JsonRpcError::invalid_params(format!("Failed to load recipe {id}: {e}"))
                })?
                .ok_or_else(|| {
                    JsonRpcError::invalid_params(format!("No recipe found for id '{id}'"))
                })?
        } else {
            return Err(JsonRpcError::invalid_params(
                "Provide 'recipe_json' string or 'recipe_id' to load from store",
            ));
        };

        let live = p
            .get("live")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let card_path = p
            .get("card_path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("/dev/dri/card0");

        if live {
            let bdf = resolve_bdf(p)?;
            let mut bar0 = nvpmu::Bar0Access::open(&bdf).map_err(|e| {
                JsonRpcError::internal_error(format!(
                    "Failed to open BAR0 for {bdf}: {e}. \
                     Run setup-gpu-sovereign.sh or use sudo."
                ))
            })?;

            let thermal = check_thermal_for_bdf(&bdf);
            if let Some(ref status) = thermal {
                if !status.compute_safe() {
                    return Err(JsonRpcError::internal_error(format!(
                        "GPU {bdf} thermal status {:?} — refusing live apply",
                        status
                    )));
                }
            }

            let mut applicator =
                hw_learn::RecipeApplicator::new(false).with_register_access(&mut bar0);
            let result = applicator.apply(&recipe, card_path);

            if result.verdict == hw_learn::applicator::ApplyVerdict::Success {
                if let Ok(mut store) = self.open_store() {
                    let _ = store.store(&recipe);
                }
            }

            Ok(serde_json::json!({
                "domain": "compute.hardware",
                "operation": "apply",
                "mode": "live",
                "bdf": bdf,
                "thermal_checked": thermal.is_some(),
                "verdict": format!("{:?}", result.verdict),
                "steps_executed": result.steps_executed,
                "steps_total": result.steps_total,
                "result": serde_json::to_value(&result).unwrap_or_default(),
            }))
        } else {
            let mut applicator = hw_learn::RecipeApplicator::new(true);
            let result = applicator.apply(&recipe, card_path);

            Ok(serde_json::json!({
                "domain": "compute.hardware",
                "operation": "apply",
                "mode": "dry_run",
                "verdict": format!("{:?}", result.verdict),
                "steps_executed": result.steps_executed,
                "steps_total": result.steps_total,
                "result": serde_json::to_value(&result).unwrap_or_default(),
                "note": "Pass 'live: true' with BAR0 access for real register writes",
            }))
        }
    }
}
