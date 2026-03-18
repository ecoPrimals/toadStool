// SPDX-License-Identifier: AGPL-3.0-or-later
//! Share recipe handler — save, load, or list recipes.

use super::HwLearnHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

impl HwLearnHandler {
    /// `compute.hardware.share_recipe` — Save, load, or list recipes.
    ///
    /// Save: `{ "action": "save", "recipe_json": "..." }`
    /// Load: `{ "action": "load", "recipe_id": "..." }`
    /// List: `{ "action": "list" }`
    ///
    /// # Errors
    ///
    /// Returns an error if params are missing/invalid, recipe JSON is invalid
    /// (save), store fails to open/save/load, recipe not found (load), or
    /// action is unknown.
    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    pub async fn hw_learn_share_recipe(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params
            .ok_or_else(|| JsonRpcError::invalid_params("Expected { action, ... } parameter"))?;

        let action = p
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("list");

        match action {
            "save" => {
                let json_str = p
                    .get("recipe_json")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        JsonRpcError::invalid_params("Missing 'recipe_json' for save action")
                    })?;
                let recipe = hw_learn::knowledge::import_recipe(json_str).map_err(|e| {
                    JsonRpcError::invalid_params(format!("Invalid recipe JSON: {e}"))
                })?;
                let mut store = self.open_store()?;
                let id = store.store(&recipe).map_err(|e| {
                    JsonRpcError::internal_error(format!("Failed to save recipe: {e}"))
                })?;
                Ok(serde_json::json!({
                    "domain": "compute.hardware",
                    "operation": "share_recipe",
                    "action": "save",
                    "recipe_id": id,
                }))
            }
            "load" => {
                let id = p
                    .get("recipe_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        JsonRpcError::invalid_params("Missing 'recipe_id' for load action")
                    })?;
                let store = self.open_store()?;
                let recipe = store
                    .load(id)
                    .map_err(|e| {
                        JsonRpcError::invalid_params(format!("Failed to load recipe {id}: {e}"))
                    })?
                    .ok_or_else(|| {
                        JsonRpcError::invalid_params(format!("No recipe found for id '{id}'"))
                    })?;
                Ok(serde_json::json!({
                    "domain": "compute.hardware",
                    "operation": "share_recipe",
                    "action": "load",
                    "recipe_id": id,
                    "recipe": serde_json::to_value(&recipe).unwrap_or_default(),
                }))
            }
            "list" => {
                let store = self.open_store()?;
                let archs = store.architectures();
                let entries: Vec<serde_json::Value> = archs
                    .iter()
                    .map(|arch| {
                        serde_json::json!({
                            "arch": format!("{}", arch),
                        })
                    })
                    .collect();
                Ok(serde_json::json!({
                    "domain": "compute.hardware",
                    "operation": "share_recipe",
                    "action": "list",
                    "architectures": entries,
                    "count": entries.len(),
                }))
            }
            _ => Err(JsonRpcError::invalid_params(format!(
                "Unknown action '{action}'. Expected 'save', 'load', or 'list'"
            ))),
        }
    }
}
