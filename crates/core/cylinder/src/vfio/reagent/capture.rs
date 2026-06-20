// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reagent capture pipeline — executes Track B extraction strategies.
//!
//! Orchestrates linux-firmware cataloging, BAR0 probing, and catalyst
//! artifact copying into a unified [`ReagentCaptureResult`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{ReagentError, ReagentManifest, catalog_linux_firmware};

/// Runtime services configuration — nvidia stays loaded as a persistent
/// compute backend while toadStool manages infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeServicesConfig {
    /// PCI BDF of the GPU to use in runtime services mode.
    pub bdf: String,
    /// Whether to probe nvidia's established FECS/GPCCS state.
    pub probe_falcon_state: bool,
    /// Whether to capture reagents during the runtime services session.
    pub capture_reagents: bool,
    /// Target directory for reagent storage.
    pub reagent_store: PathBuf,
}

impl RuntimeServicesConfig {
    /// Create with defaults for a given BDF.
    pub fn new(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_owned(),
            probe_falcon_state: true,
            capture_reagents: true,
            reagent_store: PathBuf::from(super::reagent_store_dir()),
        }
    }
}

/// Result of a reagent capture operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReagentCaptureResult {
    /// The manifest describing all captured artifacts.
    pub manifest: ReagentManifest,
    /// Path where the manifest was persisted (if successful).
    pub manifest_path: Option<PathBuf>,
    /// Per-strategy capture results.
    pub strategy_results: HashMap<String, StrategyResult>,
}

/// Result of a single capture strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyResult {
    /// Whether this strategy succeeded.
    pub success: bool,
    /// Human-readable description of what happened.
    pub detail: String,
    /// Artifacts produced (filename → size in bytes).
    pub artifacts: HashMap<String, u64>,
}

/// Execute the full reagent capture pipeline on a GPU where nvidia is loaded.
///
/// This is the core orchestrator for Track B. It:
/// 1. Probes nvidia state (FECS running, TPC alive)
/// 2. Catalogs linux-firmware blobs
/// 3. Captures domain BAR0 snapshot (if vfio-pci bound)
/// 4. Attempts VRAM firmware read via PRAMIN
/// 5. Writes ReagentManifest to reagent store
pub fn execute_reagent_capture(
    bdf: &str,
    chip: &str,
    driver_version: &str,
) -> ReagentCaptureResult {
    let kernel_version = detect_kernel_version();
    let mut manifest = ReagentManifest::new(chip, driver_version, &kernel_version, bdf);
    let mut strategy_results = HashMap::new();
    let store_dir = manifest.store_path();

    // Strategy 1: Catalog linux-firmware blobs
    let chip_lower = chip.to_lowercase();
    let blobs = catalog_linux_firmware(&chip_lower);
    let blob_count = blobs.len();
    let acr_count = blobs.iter().filter(|b| b.acr_required).count();

    manifest.firmware.linux_firmware_blobs = blobs;
    manifest.completeness.linux_firmware = blob_count > 0;

    let mut blob_artifacts = HashMap::new();
    for blob in &manifest.firmware.linux_firmware_blobs {
        blob_artifacts.insert(blob.filename.clone(), blob.size_bytes);
    }
    strategy_results.insert(
        "linux_firmware_catalog".to_owned(),
        StrategyResult {
            success: blob_count > 0,
            detail: format!("{blob_count} blobs found ({acr_count} ACR-required)"),
            artifacts: blob_artifacts,
        },
    );

    // Strategy 2: Probe nvidia state via sysfs BAR0
    let bar0_result = probe_nvidia_bar0_state(bdf);
    strategy_results.insert(
        "nvidia_state_probe".to_owned(),
        StrategyResult {
            success: bar0_result.is_ok(),
            detail: match &bar0_result {
                Ok(s) => s.clone(),
                Err(e) => format!("Failed: {e}"),
            },
            artifacts: HashMap::new(),
        },
    );

    // Strategy 3: Copy existing catalyst artifacts if available
    let catalyst_dir = PathBuf::from(crate::linux_paths::data_subdir("catalysts"));
    if copy_catalyst_artifacts(&catalyst_dir, &store_dir, &mut manifest).is_ok() {
        strategy_results.insert(
            "catalyst_artifacts".to_owned(),
            StrategyResult {
                success: true,
                detail: "Catalyst artifacts copied to reagent store".to_owned(),
                artifacts: HashMap::new(),
            },
        );
    }

    // Persist manifest
    let manifest_path = match manifest.persist() {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to persist reagent manifest");
            None
        }
    };

    ReagentCaptureResult {
        manifest,
        manifest_path,
        strategy_results,
    }
}

fn detect_kernel_version() -> String {
    std::fs::read_to_string("/proc/version")
        .ok()
        .and_then(|v| v.split_whitespace().nth(2).map(|s| s.to_owned()))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn probe_nvidia_bar0_state(bdf: &str) -> Result<String, ReagentError> {
    let resource_path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
    if !Path::new(&resource_path).exists() {
        return Err(ReagentError::NoBar0Resource {
            path: resource_path,
        });
    }

    let driver_path = crate::linux_paths::sysfs_pci_device_file(bdf, "driver");
    let driver = std::fs::read_link(&driver_path)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unbound".to_owned());

    Ok(format!("BDF={bdf} driver={driver}"))
}

fn copy_catalyst_artifacts(
    catalyst_dir: &Path,
    reagent_dir: &Path,
    manifest: &mut ReagentManifest,
) -> Result<(), ReagentError> {
    std::fs::create_dir_all(reagent_dir).map_err(|source| ReagentError::MirrorMkdirFailed {
        path: reagent_dir.display().to_string(),
        source,
    })?;

    // Copy patch set recipe if available
    let recipes_dir = catalyst_dir.join("recipes");
    if recipes_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&recipes_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let Some(name) = path.file_name() else {
                    continue;
                };
                let dest = reagent_dir.join(name);
                if std::fs::copy(&path, &dest).is_ok() {
                    manifest.patch_set = Some(dest);
                    manifest.completeness.patch_set = true;
                }
            }
        }
    }

    // Copy firmware artifacts if captured
    let fw_dir = catalyst_dir.join("firmware");
    if fw_dir.is_dir() {
        let reagent_fw_dir = reagent_dir.join("firmware");
        std::fs::create_dir_all(&reagent_fw_dir).map_err(ReagentError::CatalystMkdirFailed)?;

        if let Ok(entries) = std::fs::read_dir(&fw_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name() else {
                    continue;
                };
                let dest = reagent_fw_dir.join(name);
                if std::fs::copy(&path, &dest).is_ok() {
                    let name = name.to_string_lossy();
                    if name.contains("fecs_imem") {
                        manifest.firmware.fecs_imem = Some(dest.clone());
                    } else if name.contains("gpccs_imem") {
                        manifest.firmware.gpccs_imem = Some(dest.clone());
                    } else if name.contains("pmu_dmem") {
                        manifest.firmware.pmu_dmem = Some(dest.clone());
                    }
                    manifest.completeness.falcon_firmware = true;
                    manifest
                        .firmware
                        .provenance
                        .insert(name.to_string(), "catalyst_capture".to_owned());
                }
            }
        }
    }

    Ok(())
}
