// SPDX-License-Identifier: AGPL-3.0-or-later
mod nouveau;
mod nvidia;

use super::types::{PatchError, PatchSet};

impl PatchSet {
    /// Look up a predefined patch set by name.
    #[must_use]
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "volta_warm_handoff" => Some(Self::volta_warm_handoff()),
            "kepler_warm_handoff" => Some(Self::kepler_warm_handoff()),
            "nvidia_warm_handoff" => Some(Self::nvidia_warm_handoff()),
            "nvidia_catalyst_handoff" => Some(Self::nvidia_catalyst_handoff()),
            "nvidia_boot_services" => Some(Self::nvidia_boot_services()),
            _ => None,
        }
    }

    /// Dispatch a patch set from GPU chip family and driver version.
    ///
    /// Returns the compiled-in patch set for known combinations. Falls back
    /// to `by_name` when a TOML recipe has been loaded. This replaces
    /// magic string names with a structured dispatch path.
    #[must_use]
    pub fn by_profile(
        chip_family: crate::nv::gr_init::ChipFamily,
        driver: &str,
        strategy: &str,
    ) -> Option<Self> {
        use crate::nv::gr_init::ChipFamily;
        match (chip_family, driver, strategy) {
            (ChipFamily::Volta, "nvidia-470" | "470.256.02", "catalyst") => {
                Some(Self::nvidia_catalyst_handoff())
            }
            (ChipFamily::Volta, "nvidia-470" | "470.256.02", "warm") => {
                Some(Self::nvidia_warm_handoff())
            }
            (ChipFamily::Volta, "nvidia-470" | "470.256.02", "boot_services") => {
                Some(Self::nvidia_boot_services())
            }
            (ChipFamily::Volta, "nouveau", _) => {
                Some(Self::volta_warm_handoff())
            }
            (ChipFamily::Kepler, "nouveau", _) => {
                Some(Self::kepler_warm_handoff())
            }
            _ => Self::by_name(strategy),
        }
    }

    /// Deserialize a patch set from a JSON string.
    ///
    /// Enables runtime-defined patch sets — experiments can iterate on
    /// target lists and strategies without recompiling.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON for recipe persistence.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load a patch set from a catalyst recipe TOML file.
    ///
    /// Parses the `[[patches]]` array from recipes like
    /// `infra/catalysts/recipes/gv100_nvidia470.toml`. Each entry has
    /// `symbol` and `strategy` (string format). Enables new GPU+driver
    /// combos without recompiling cylinder.
    pub fn from_recipe_toml(toml_str: &str) -> Result<Self, PatchError> {
        let doc: toml::Value = toml::from_str(toml_str)?;

        let catalyst = doc
            .get("catalyst")
            .ok_or(PatchError::RecipeMissingSection("[catalyst]"))?;
        let name = catalyst
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("recipe")
            .to_string();

        let source = doc
            .get("source")
            .ok_or(PatchError::RecipeMissingSection("[source]"))?;
        let module_name = source
            .get("dkms_module")
            .and_then(|v| v.as_str())
            .unwrap_or("nvidia")
            .to_string();

        let patches = doc
            .get("patches")
            .and_then(|v| v.as_array())
            .ok_or(PatchError::RecipeMissingSection("[[patches]]"))?;

        let mut targets = Vec::new();
        for patch in patches {
            let symbol = patch
                .get("symbol")
                .and_then(|v| v.as_str())
                .ok_or(PatchError::RecipeInvalidPatch("patch missing 'symbol'"))?
                .to_string();
            let strategy_str = patch
                .get("strategy")
                .and_then(|v| v.as_str())
                .ok_or(PatchError::RecipeInvalidPatch("patch missing 'strategy'"))?;

            let strategy: super::types::PatchStrategy = strategy_str.parse().map_err(|e: String| {
                PatchError::InvalidPatchStrategy {
                    raw: strategy_str.to_string(),
                    detail: e,
                }
            })?;

            targets.push(super::types::PatchTarget { symbol, strategy });
        }

        Ok(Self {
            name,
            module_name,
            min_applied: 1,
            targets,
        })
    }
}
