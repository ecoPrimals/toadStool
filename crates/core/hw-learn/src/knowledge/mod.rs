// SPDX-License-Identifier: AGPL-3.0-only
//! Cross-vendor knowledge store for GPU init recipes.
//!
//! Stores validated recipes indexed by architecture, making them
//! available for application to target GPUs and distribution via
//! biomeOS Plasmodium federation.

pub mod amd_baseline;
pub mod arch_map;

use crate::distiller::{GpuArch, InitRecipe, Vendor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Unique identifier for a stored recipe.
pub type RecipeId = String;

/// Architecture identifier for indexing recipes.
pub type ArchId = String;

/// Persistent recipe store backed by a directory of JSON files.
///
/// Layout:
/// ```text
/// store_dir/
///   nvidia/
///     sm70_volta.json
///     sm86_ampere.json
///   amd/
///     gfx1030_navi21.json
///   intel/
///     gen12_dg2.json
///   index.json
/// ```
pub struct KnowledgeStore {
    store_dir: PathBuf,
    index: StoreIndex,
}

/// In-memory index of stored recipes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoreIndex {
    recipes: HashMap<ArchId, Vec<RecipeEntry>>,
}

/// Index entry for a single recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeEntry {
    id: RecipeId,
    source_arch: GpuArch,
    target_arch: GpuArch,
    confidence: f64,
    path: PathBuf,
}

impl KnowledgeStore {
    /// Open or create a knowledge store at the given directory.
    ///
    /// # Errors
    /// Returns `Err` if the directory cannot be created or the index file cannot be read.
    pub fn open(store_dir: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let store_dir = store_dir.into();
        std::fs::create_dir_all(&store_dir)?;

        let index_path = store_dir.join("index.json");
        let index = if index_path.exists() {
            let data = std::fs::read_to_string(&index_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            StoreIndex::default()
        };

        Ok(Self { store_dir, index })
    }

    /// Store a new recipe, returning its ID.
    ///
    /// # Errors
    /// Returns `Err` if the vendor directory cannot be created, the recipe JSON cannot be serialized, or the file cannot be written.
    pub fn store(&mut self, recipe: &InitRecipe) -> Result<RecipeId, std::io::Error> {
        let arch_id = arch_to_id(&recipe.target_arch);
        let vendor_dir = self
            .store_dir
            .join(vendor_dirname(recipe.target_arch.vendor));
        std::fs::create_dir_all(&vendor_dir)?;

        let id = format!("{}_{}", arch_id, recipe_hash(recipe));
        let filename = format!("{id}.json");
        let path = vendor_dir.join(&filename);

        let json = serde_json::to_string_pretty(recipe).map_err(std::io::Error::other)?;
        std::fs::write(&path, json)?;

        let entry = RecipeEntry {
            id: id.clone(),
            source_arch: recipe.source_arch.clone(),
            target_arch: recipe.target_arch.clone(),
            confidence: recipe.confidence,
            path: path
                .strip_prefix(&self.store_dir)
                .unwrap_or(&path)
                .to_path_buf(),
        };

        self.index.recipes.entry(arch_id).or_default().push(entry);
        self.save_index()?;

        Ok(id)
    }

    /// Look up recipes for a target architecture.
    #[must_use]
    pub fn lookup(&self, target: &GpuArch) -> Vec<&RecipeEntry> {
        let arch_id = arch_to_id(target);
        self.index
            .recipes
            .get(&arch_id)
            .map(|entries| entries.iter().collect())
            .unwrap_or_default()
    }

    /// Load a recipe by ID.
    ///
    /// # Errors
    /// Returns `Err` if the recipe file cannot be read or the JSON cannot be parsed.
    pub fn load(&self, id: &str) -> Result<Option<InitRecipe>, std::io::Error> {
        for entries in self.index.recipes.values() {
            for entry in entries {
                if entry.id == id {
                    let path = self.store_dir.join(&entry.path);
                    let data = std::fs::read_to_string(path)?;
                    let recipe: InitRecipe = serde_json::from_str(&data)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    return Ok(Some(recipe));
                }
            }
        }
        Ok(None)
    }

    /// Get the best recipe for a target (highest confidence).
    #[must_use]
    pub fn best_recipe(&self, target: &GpuArch) -> Option<RecipeId> {
        let entries = self.lookup(target);
        entries
            .into_iter()
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| e.id.clone())
    }

    /// Update confidence score after validation.
    ///
    /// # Errors
    /// Returns `Err` if the index cannot be saved to disk.
    pub fn update_confidence(&mut self, id: &str, confidence: f64) -> Result<(), std::io::Error> {
        for entries in self.index.recipes.values_mut() {
            for entry in entries.iter_mut() {
                if entry.id == id {
                    entry.confidence = confidence;
                }
            }
        }
        self.save_index()
    }

    /// List all stored architectures.
    #[must_use]
    pub fn architectures(&self) -> Vec<&ArchId> {
        self.index.recipes.keys().collect()
    }

    /// Export the full index as JSON for Plasmodium federation sharing.
    ///
    /// # Errors
    /// Returns `Err` if the index cannot be serialized to JSON.
    pub fn export_index(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.index)
    }

    fn save_index(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self.index).map_err(std::io::Error::other)?;
        std::fs::write(self.store_dir.join("index.json"), json)
    }
}

fn arch_to_id(arch: &GpuArch) -> ArchId {
    format!("{}_{}", arch.compute_class, arch.chip).to_lowercase()
}

const fn vendor_dirname(vendor: Vendor) -> &'static str {
    match vendor {
        Vendor::Amd => "amd",
        Vendor::Intel => "intel",
        Vendor::Nvidia => "nvidia",
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "hash truncated to 32 bits for short recipe ID"
)]
fn recipe_hash(recipe: &InitRecipe) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    recipe.description.hash(&mut hasher);
    recipe.steps.len().hash(&mut hasher);
    format!("{:08x}", hasher.finish() as u32)
}

/// Export a recipe to a portable format for cross-machine sharing.
///
/// # Errors
/// Returns `Err` if the recipe cannot be serialized to JSON.
pub fn export_recipe(recipe: &InitRecipe) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(recipe)
}

/// Import a recipe from portable JSON format.
///
/// # Errors
/// Returns `Err` if the JSON cannot be parsed or does not represent a valid `InitRecipe`.
pub fn import_recipe(json: &str) -> Result<InitRecipe, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distiller::{InitStep, RegFunction, VerifyCheck};

    fn test_arch() -> GpuArch {
        GpuArch {
            vendor: Vendor::Nvidia,
            generation: "Volta".into(),
            chip: "GV100".into(),
            compute_class: "sm70".into(),
        }
    }

    fn test_recipe() -> InitRecipe {
        InitRecipe {
            source_arch: test_arch(),
            source_driver: crate::distiller::DriverKind::Nouveau,
            target_arch: test_arch(),
            steps: vec![
                InitStep::RegisterWrite {
                    offset: 0x20000,
                    value: 1,
                    function: RegFunction::PowerGate,
                },
                InitStep::Verify {
                    check: VerifyCheck::ComputeReadback,
                },
            ],
            confidence: 0.0,
            description: "test recipe".into(),
        }
    }

    #[test]
    fn roundtrip_store_load() {
        let dir = std::env::temp_dir().join("hw_learn_test_store");
        let _ = std::fs::remove_dir_all(&dir);

        let mut store = KnowledgeStore::open(&dir).unwrap();
        let recipe = test_recipe();
        let id = store.store(&recipe).unwrap();

        let loaded = store.load(&id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().description, "test recipe");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn best_recipe_selects_highest_confidence() {
        let dir = std::env::temp_dir().join("hw_learn_test_best");
        let _ = std::fs::remove_dir_all(&dir);

        let mut store = KnowledgeStore::open(&dir).unwrap();
        let mut r1 = test_recipe();
        r1.confidence = 0.3;
        r1.description = "low confidence".into();
        let _id1 = store.store(&r1).unwrap();

        let mut r2 = test_recipe();
        r2.confidence = 0.9;
        r2.description = "high confidence".into();
        let id2 = store.store(&r2).unwrap();

        let best = store.best_recipe(&test_arch());
        assert_eq!(best.unwrap(), id2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_import_roundtrip() {
        let recipe = test_recipe();
        let json = export_recipe(&recipe).unwrap();
        let imported = import_recipe(&json).unwrap();
        assert_eq!(imported.steps.len(), recipe.steps.len());
    }
}
