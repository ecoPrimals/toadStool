// SPDX-License-Identifier: AGPL-3.0-or-later
mod nouveau;
mod nvidia;

use super::types::PatchSet;

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
}
