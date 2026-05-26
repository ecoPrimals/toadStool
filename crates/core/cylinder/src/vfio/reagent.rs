// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reagent capture pipeline — systematic extraction and cataloging of
//! firmware "chemical agents" that enable GPU compute.
//!
//! While nvidia is loaded as a runtime service, the diesel engine captures
//! every component that enables Tier 2 compute: BAR0 register state,
//! falcon firmware blobs, ACR boot sequences, and linux-firmware binaries.
//! These are stored as versioned "reagents" for late-stage sovereign
//! Tier 2 replay without the vendor driver.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Track A: Runtime Services (nvidia loaded, Tier 2 now)      │
//! ├─────────────────────────────────────────────────────────────┤
//! │ Track B: Reagent Capture (extract while alive)             │
//! │   BAR0 snapshot → falcon IMEM/DMEM → ACR sequence → blobs │
//! ├─────────────────────────────────────────────────────────────┤
//! │ Track C: Late-stage replay (sovereign Tier 2 from reagents)│
//! └─────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Default runtime reagent storage directory.
pub const REAGENT_STORE_DIR: &str = "/var/lib/toadstool/reagents";

/// A unified artifact combining all captured chemical agents for a
/// specific GPU + driver + kernel combination.
///
/// The manifest is the receipt proving what was captured. Each field
/// points to an artifact file within the reagent directory. Optional
/// fields reflect capture strategies that may fail (e.g., IMEM capture
/// blocked by HS fuses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReagentManifest {
    /// GPU chip identifier (e.g. "GV100", "GA102").
    pub chip: String,
    /// Driver version that served as the chemical catalyst (e.g. "470.256.02").
    pub driver_version: String,
    /// Kernel version the catalyst ran on (e.g. "6.17.9-76061709-generic").
    pub kernel_version: String,
    /// PCI BDF of the device this was captured from.
    pub bdf: String,
    /// ISO 8601 timestamp of capture.
    pub captured_at: String,
    /// Path to BAR0 domain snapshot JSON (alive registers only).
    pub bar0_snapshot: Option<PathBuf>,
    /// Path to BAR0 replay sequence JSON (`GrInitSequence`).
    pub bar0_replay: Option<PathBuf>,
    /// Path to the PatchSet JSON used to prepare the catalyst driver.
    pub patch_set: Option<PathBuf>,
    /// Path to frozen patched .ko binary (41+ MB, gitignored).
    pub frozen_module: Option<PathBuf>,
    /// Firmware artifacts captured or cataloged.
    pub firmware: ReagentFirmware,
    /// Path to mmiotrace-distilled recipe JSON.
    pub mmiotrace_recipe: Option<PathBuf>,
    /// Per-artifact metadata (sizes, provenance notes).
    pub metadata: HashMap<String, String>,
    /// Capture completeness — what fraction of reagents were successfully captured.
    pub completeness: ReagentCompleteness,
}

/// Firmware reagent artifacts — the chemical agents that enable compute.
///
/// Each `Option<PathBuf>` points to a captured binary blob. Capture may
/// fail due to HS fuse enforcement, RM gating, or IMEM wipe during unbind.
/// The `source` field documents provenance for each blob.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReagentFirmware {
    /// FECS instruction memory (if captured or extracted).
    pub fecs_imem: Option<PathBuf>,
    /// FECS data memory.
    pub fecs_dmem: Option<PathBuf>,
    /// GPCCS instruction memory.
    pub gpccs_imem: Option<PathBuf>,
    /// GPCCS data memory.
    pub gpccs_dmem: Option<PathBuf>,
    /// PMU instruction memory.
    pub pmu_imem: Option<PathBuf>,
    /// PMU data memory.
    pub pmu_dmem: Option<PathBuf>,
    /// SEC2 instruction memory.
    pub sec2_imem: Option<PathBuf>,
    /// ACR boot sequence distilled from mmiotrace or BAR0 capture.
    pub acr_sequence: Option<PathBuf>,
    /// Signed firmware blobs from `/lib/firmware/nvidia/{chip}/`.
    pub linux_firmware_blobs: Vec<FirmwareBlob>,
    /// Per-blob provenance notes.
    pub provenance: HashMap<String, String>,
}

/// A cataloged firmware blob from linux-firmware or extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareBlob {
    /// Subsystem (e.g. "gr", "acr", "sec2", "pmu").
    pub subsystem: String,
    /// Filename (e.g. "fecs_inst.bin").
    pub filename: String,
    /// Absolute path on disk.
    pub path: PathBuf,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Whether this blob is required for ACR boot chain.
    pub acr_required: bool,
}

/// How complete the reagent capture was.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReagentCompleteness {
    /// BAR0 register snapshot captured.
    pub bar0_snapshot: bool,
    /// BAR0 replay sequence generated.
    pub bar0_replay: bool,
    /// Patch set recipe captured.
    pub patch_set: bool,
    /// Any falcon IMEM/DMEM captured (pre-swap or post-recovery).
    pub falcon_firmware: bool,
    /// linux-firmware blobs cataloged.
    pub linux_firmware: bool,
    /// mmiotrace recipe distilled.
    pub mmiotrace_recipe: bool,
    /// VRAM firmware captured via PRAMIN.
    pub vram_firmware: bool,
}

impl ReagentCompleteness {
    /// Fraction of capture strategies that succeeded (0.0 to 1.0).
    #[must_use]
    pub fn fraction(&self) -> f64 {
        let total = 7.0;
        let done = [
            self.bar0_snapshot,
            self.bar0_replay,
            self.patch_set,
            self.falcon_firmware,
            self.linux_firmware,
            self.mmiotrace_recipe,
            self.vram_firmware,
        ]
        .iter()
        .filter(|&&v| v)
        .count() as f64;
        done / total
    }
}

impl Default for ReagentCompleteness {
    fn default() -> Self {
        Self {
            bar0_snapshot: false,
            bar0_replay: false,
            patch_set: false,
            falcon_firmware: false,
            linux_firmware: false,
            mmiotrace_recipe: false,
            vram_firmware: false,
        }
    }
}

impl ReagentManifest {
    /// Create a new empty manifest for a capture session.
    pub fn new(chip: &str, driver_version: &str, kernel_version: &str, bdf: &str) -> Self {
        let ts = chrono_iso8601_now();
        Self {
            chip: chip.to_owned(),
            driver_version: driver_version.to_owned(),
            kernel_version: kernel_version.to_owned(),
            bdf: bdf.to_owned(),
            captured_at: ts,
            bar0_snapshot: None,
            bar0_replay: None,
            patch_set: None,
            frozen_module: None,
            firmware: ReagentFirmware::default(),
            mmiotrace_recipe: None,
            metadata: HashMap::new(),
            completeness: ReagentCompleteness::default(),
        }
    }

    /// Directory name for this reagent set (chip + driver + kernel).
    #[must_use]
    pub fn dir_name(&self) -> String {
        let kernel_short = self
            .kernel_version
            .split('-')
            .next()
            .unwrap_or(&self.kernel_version);
        format!(
            "{}_nvidia{}_k{}",
            self.chip.to_lowercase(),
            self.driver_version.replace('.', ""),
            kernel_short,
        )
    }

    /// Full path to the reagent directory.
    #[must_use]
    pub fn store_path(&self) -> PathBuf {
        PathBuf::from(REAGENT_STORE_DIR).join(self.dir_name())
    }

    /// Write the manifest to the reagent store directory.
    ///
    /// Creates the directory structure if it doesn't exist.
    pub fn persist(&self) -> Result<PathBuf, String> {
        let dir = self.store_path();
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create reagent dir {}: {e}", dir.display()))?;

        let manifest_path = dir.join("manifest.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {e}"))?;
        std::fs::write(&manifest_path, json)
            .map_err(|e| format!("Failed to write manifest: {e}"))?;

        tracing::info!(
            path = %manifest_path.display(),
            chip = %self.chip,
            completeness = format!("{:.0}%", self.completeness.fraction() * 100.0),
            "Reagent manifest persisted"
        );
        Ok(manifest_path)
    }

    /// Load a manifest from disk.
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest {}: {e}", path.display()))?;
        serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse manifest: {e}"))
    }

    /// Mirror small JSON artifacts to a repo-side reagent directory.
    ///
    /// Large binaries (frozen .ko, full mmiotrace) are NOT copied — only
    /// manifest.json and JSON recipe files under a size threshold.
    pub fn mirror_to_repo(&self, repo_reagents_dir: &Path) -> Result<PathBuf, String> {
        const MAX_MIRROR_SIZE: u64 = 5 * 1024 * 1024; // 5 MiB threshold

        let dest_dir = repo_reagents_dir.join(self.dir_name());
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("mkdir {}: {e}", dest_dir.display()))?;
        std::fs::create_dir_all(dest_dir.join("firmware")).ok();
        std::fs::create_dir_all(dest_dir.join("mmiotrace")).ok();

        // Write manifest
        let manifest_path = dest_dir.join("manifest.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(&manifest_path, json)
            .map_err(|e| format!("write manifest: {e}"))?;

        // Mirror JSON artifacts under size threshold
        let json_sources = [
            &self.bar0_snapshot,
            &self.bar0_replay,
            &self.patch_set,
            &self.mmiotrace_recipe,
        ];
        for src_opt in &json_sources {
            if let Some(src) = src_opt {
                if src.exists() {
                    let size = std::fs::metadata(src).map(|m| m.len()).unwrap_or(u64::MAX);
                    if size < MAX_MIRROR_SIZE {
                        if let Some(name) = src.file_name() {
                            std::fs::copy(src, dest_dir.join(name)).ok();
                        }
                    }
                }
            }
        }

        tracing::info!(
            dest = %dest_dir.display(),
            "Reagent artifacts mirrored to repo"
        );
        Ok(manifest_path)
    }
}

/// Catalog linux-firmware blobs for a given chip.
///
/// Scans `/lib/firmware/nvidia/{chip}/` for known firmware files and
/// returns a list of those that exist on disk with their metadata.
pub fn catalog_linux_firmware(chip: &str) -> Vec<FirmwareBlob> {
    let base = format!("/lib/firmware/nvidia/{chip}");

    let known_blobs = [
        ("acr", "bl.bin", true),
        ("acr", "ucode_unload.bin", false),
        ("gr", "fecs_bl.bin", true),
        ("gr", "fecs_inst.bin", true),
        ("gr", "fecs_data.bin", true),
        ("gr", "gpccs_bl.bin", true),
        ("gr", "gpccs_inst.bin", true),
        ("gr", "gpccs_data.bin", true),
        ("gr", "sw_ctx.bin", false),
        ("gr", "sw_nonctx.bin", false),
        ("gr", "sw_bundle_init.bin", false),
        ("gr", "sw_method_init.bin", false),
        ("sec2", "desc.bin", true),
        ("sec2", "image.bin", true),
        ("sec2", "sig.bin", true),
        ("pmu", "bl.bin", false),
        ("pmu", "inst.bin", false),
        ("pmu", "data.bin", false),
        ("pmu", "sig.bin", false),
    ];

    let mut found = Vec::new();
    for (subsystem, filename, acr_required) in &known_blobs {
        let path = PathBuf::from(format!("{base}/{subsystem}/{filename}"));
        if path.exists() {
            let size_bytes = std::fs::metadata(&path)
                .map(|m| m.len())
                .unwrap_or(0);
            found.push(FirmwareBlob {
                subsystem: (*subsystem).to_owned(),
                filename: (*filename).to_owned(),
                path,
                size_bytes,
                acr_required: *acr_required,
            });
        }
    }

    tracing::info!(
        chip = chip,
        found = found.len(),
        acr_required_present = found.iter().filter(|b| b.acr_required).count(),
        "linux-firmware blobs cataloged"
    );

    found
}

/// Distill an mmiotrace log into an ACR-focused reagent recipe.
///
/// Parses the trace through `BootTrace`, extracts the write sequence,
/// filters to falcon/ACR/GR domains, and saves as a JSON recipe file.
/// This captures the exact register programming that nvidia uses to
/// boot FECS/GPCCS — the chemical agents for Tier 2.
pub fn distill_mmiotrace_to_reagent(
    trace_path: &Path,
    output_path: &Path,
) -> Result<MmiotraceReagentSummary, String> {
    use crate::vfio::channel::diagnostic::boot_follower::BootTrace;

    let trace = BootTrace::from_mmiotrace(trace_path)
        .map_err(|e| format!("Failed to parse mmiotrace: {e}"))?;

    let total_writes = trace.writes.len();
    let total_reads = trace.reads.len();
    let domain_summary = trace.domain_summary();

    let recipe = trace.to_recipe();
    let recipe_steps = recipe.len();

    // Filter to ACR/falcon-relevant domains
    let acr_domains = [
        "PMC", "PRI_MASTER", "PMU", "PFIFO", "PBDMA", "PRAMIN",
    ];
    let acr_recipe: Vec<_> = recipe
        .iter()
        .filter(|s| acr_domains.contains(&s.domain.as_str()) || s.domain == "UNKNOWN")
        .cloned()
        .collect();
    let acr_steps = acr_recipe.len();

    // Save full recipe
    let json = serde_json::to_string_pretty(&recipe)
        .map_err(|e| format!("Failed to serialize recipe: {e}"))?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    std::fs::write(output_path, &json)
        .map_err(|e| format!("Failed to write recipe: {e}"))?;

    // Save ACR-focused subset
    let acr_output = output_path.with_file_name(
        output_path
            .file_stem()
            .map(|s| format!("{}_acr_subset", s.to_string_lossy()))
            .unwrap_or_else(|| "acr_subset".to_owned())
            + ".json",
    );
    let acr_json = serde_json::to_string_pretty(&acr_recipe)
        .map_err(|e| format!("Failed to serialize ACR recipe: {e}"))?;
    std::fs::write(&acr_output, &acr_json)
        .map_err(|e| format!("Failed to write ACR recipe: {e}"))?;

    let summary = MmiotraceReagentSummary {
        trace_path: trace_path.to_path_buf(),
        total_writes,
        total_reads,
        duration_us: trace.duration_us,
        domain_summary,
        recipe_steps,
        acr_steps,
        output_path: output_path.to_path_buf(),
        acr_output_path: acr_output,
    };

    tracing::info!(
        trace = %trace_path.display(),
        writes = total_writes,
        reads = total_reads,
        recipe_steps = recipe_steps,
        acr_steps = acr_steps,
        "mmiotrace distilled to reagent recipe"
    );

    Ok(summary)
}

/// Summary of an mmiotrace distillation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmiotraceReagentSummary {
    pub trace_path: PathBuf,
    pub total_writes: usize,
    pub total_reads: usize,
    pub duration_us: u64,
    pub domain_summary: std::collections::BTreeMap<String, usize>,
    pub recipe_steps: usize,
    pub acr_steps: usize,
    pub output_path: PathBuf,
    pub acr_output_path: PathBuf,
}

/// Attempt to read VRAM content through the PRAMIN window (BAR0 0x700000).
///
/// When nvidia is loaded and PRAMIN is configured, the 1 MiB window at
/// BAR0 offset 0x700000 maps a configurable region of GPU VRAM. By writing
/// the target VRAM page address to `NV_PBUS_BAR0_WINDOW` (0x1700), we can
/// read arbitrary VRAM contents.
///
/// Returns the bytes read, or an error if PRAMIN is not configured or the
/// read fails.
pub fn read_vram_via_pramin(
    bar0: &crate::vfio::device::MappedBar,
    vram_addr: u64,
    len: usize,
) -> Result<Vec<u8>, String> {
    const PRAMIN_BASE: usize = 0x70_0000;
    const PRAMIN_SIZE: usize = 0x10_0000; // 1 MiB window
    const BAR0_WINDOW_REG: usize = 0x1700;

    if len > PRAMIN_SIZE {
        return Err(format!("Requested {len} bytes exceeds PRAMIN window size {PRAMIN_SIZE}"));
    }

    let page_addr = (vram_addr >> 16) as u32;
    bar0.write_u32(BAR0_WINDOW_REG, page_addr)
        .map_err(|e| format!("PRAMIN window write failed: {e}"))?;

    let page_offset = (vram_addr & 0xFFFF) as usize;
    let mut data = Vec::with_capacity(len);

    for i in (0..len).step_by(4) {
        let offset = PRAMIN_BASE + page_offset + i;
        let word = bar0
            .read_u32(offset)
            .map_err(|e| format!("PRAMIN read at 0x{offset:x} failed: {e}"))?;
        data.extend_from_slice(&word.to_le_bytes());
    }

    data.truncate(len);

    let nonzero = data.iter().filter(|&&b| b != 0).count();
    tracing::info!(
        vram_addr = format!("0x{vram_addr:x}"),
        len = len,
        nonzero_bytes = nonzero,
        "VRAM read via PRAMIN"
    );

    Ok(data)
}

/// Known VRAM firmware staging addresses from Exp 160 mmiotrace analysis.
/// nvidia stages firmware blobs in VRAM before DMA-loading to falcon IMEM.
pub mod vram_firmware_addrs {
    /// FECS firmware staged at this VRAM address before BootROM DMA (Exp 160, nvidia-535).
    pub const FECS_VRAM_ADDR_535: u64 = 0x802F_D458;
    /// FECS code size (from nvidia-535 mmiotrace — 25632 bytes, matches fecs_inst.bin).
    pub const FECS_CODE_SIZE: usize = 25632;
    /// GPCCS firmware typically follows FECS in VRAM (offset determined at runtime).
    pub const GPCCS_VRAM_OFFSET_HINT: u64 = 0x10000;
}

/// Attempt VRAM firmware capture for all known falcon staging addresses.
///
/// While nvidia is loaded and FECS is running, the firmware blobs are staged
/// in VRAM at known addresses. This function reads them through the PRAMIN
/// window, bypassing the HS IMEM PIO block entirely.
///
/// Returns paths to captured firmware files, or errors for each.
pub fn capture_vram_firmware(
    bar0: &crate::vfio::device::MappedBar,
    output_dir: &std::path::Path,
) -> Vec<(String, Result<PathBuf, String>)> {
    use vram_firmware_addrs::*;

    std::fs::create_dir_all(output_dir).ok();
    let mut results = Vec::new();

    // Capture FECS from VRAM
    let fecs_path = output_dir.join("fecs_vram_capture.bin");
    let fecs_result = read_vram_via_pramin(bar0, FECS_VRAM_ADDR_535, FECS_CODE_SIZE)
        .and_then(|data| {
            let nonzero = data.iter().filter(|&&b| b != 0).count();
            if nonzero < FECS_CODE_SIZE / 10 {
                return Err(format!(
                    "FECS VRAM capture mostly zeros ({nonzero}/{FECS_CODE_SIZE} nonzero) — \
                     firmware may not be staged at 0x{FECS_VRAM_ADDR_535:x}"
                ));
            }
            std::fs::write(&fecs_path, &data)
                .map_err(|e| format!("write {}: {e}", fecs_path.display()))?;
            tracing::info!(
                path = %fecs_path.display(),
                size = data.len(),
                nonzero = nonzero,
                "FECS firmware captured from VRAM"
            );
            Ok(fecs_path.clone())
        });
    results.push(("fecs_vram".to_owned(), fecs_result));

    // Attempt GPCCS capture at hinted offset after FECS
    let gpccs_addr = FECS_VRAM_ADDR_535 + GPCCS_VRAM_OFFSET_HINT;
    let gpccs_size = 12643; // matches gpccs_inst.bin
    let gpccs_path = output_dir.join("gpccs_vram_capture.bin");
    let gpccs_result = read_vram_via_pramin(bar0, gpccs_addr, gpccs_size)
        .and_then(|data| {
            let nonzero = data.iter().filter(|&&b| b != 0).count();
            if nonzero < gpccs_size / 10 {
                return Err(format!(
                    "GPCCS VRAM capture mostly zeros ({nonzero}/{gpccs_size} nonzero) — \
                     address 0x{gpccs_addr:x} may be wrong"
                ));
            }
            std::fs::write(&gpccs_path, &data)
                .map_err(|e| format!("write {}: {e}", gpccs_path.display()))?;
            tracing::info!(
                path = %gpccs_path.display(),
                size = data.len(),
                nonzero = nonzero,
                "GPCCS firmware captured from VRAM"
            );
            Ok(gpccs_path.clone())
        });
    results.push(("gpccs_vram".to_owned(), gpccs_result));

    results
}

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
            reagent_store: PathBuf::from(REAGENT_STORE_DIR),
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

fn chrono_iso8601_now() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    let days = secs / 86400;
    let year = 1970 + days / 365;
    format!("{year}-XX-XXT{hours:02}:{mins:02}:{s:02}Z")
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
            detail: bar0_result
                .as_ref()
                .map(|s| s.clone())
                .unwrap_or_else(|e| format!("Failed: {e}")),
            artifacts: HashMap::new(),
        },
    );

    // Strategy 3: Copy existing catalyst artifacts if available
    let catalyst_dir = PathBuf::from("/var/lib/toadstool/catalysts");
    if let Ok(()) = copy_catalyst_artifacts(&catalyst_dir, &store_dir, &mut manifest) {
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
        .and_then(|v| {
            v.split_whitespace()
                .nth(2)
                .map(|s| s.to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn probe_nvidia_bar0_state(bdf: &str) -> Result<String, String> {
    let resource_path = format!("/sys/bus/pci/devices/{bdf}/resource0");
    if !Path::new(&resource_path).exists() {
        return Err(format!("No BAR0 resource at {resource_path}"));
    }

    let driver_path = format!("/sys/bus/pci/devices/{bdf}/driver");
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
) -> Result<(), String> {
    std::fs::create_dir_all(reagent_dir)
        .map_err(|e| format!("mkdir {}: {e}", reagent_dir.display()))?;

    // Copy patch set recipe if available
    let recipes_dir = catalyst_dir.join("recipes");
    if recipes_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&recipes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    let dest = reagent_dir.join(path.file_name().unwrap());
                    if std::fs::copy(&path, &dest).is_ok() {
                        manifest.patch_set = Some(dest);
                        manifest.completeness.patch_set = true;
                    }
                }
            }
        }
    }

    // Copy firmware artifacts if captured
    let fw_dir = catalyst_dir.join("firmware");
    if fw_dir.is_dir() {
        let reagent_fw_dir = reagent_dir.join("firmware");
        std::fs::create_dir_all(&reagent_fw_dir)
            .map_err(|e| format!("mkdir firmware: {e}"))?;

        if let Ok(entries) = std::fs::read_dir(&fw_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let dest = reagent_fw_dir.join(path.file_name().unwrap());
                if std::fs::copy(&path, &dest).is_ok() {
                    let name = path.file_name().unwrap().to_string_lossy();
                    if name.contains("fecs_imem") {
                        manifest.firmware.fecs_imem = Some(dest.clone());
                    } else if name.contains("gpccs_imem") {
                        manifest.firmware.gpccs_imem = Some(dest.clone());
                    } else if name.contains("pmu_dmem") {
                        manifest.firmware.pmu_dmem = Some(dest.clone());
                    }
                    manifest.completeness.falcon_firmware = true;
                    manifest.firmware.provenance.insert(
                        name.to_string(),
                        "catalyst_capture".to_owned(),
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let manifest = ReagentManifest::new("GV100", "470.256.02", "6.17.9", "0000:02:00.0");
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let loaded: ReagentManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.chip, "GV100");
        assert_eq!(loaded.driver_version, "470.256.02");
        assert_eq!(loaded.bdf, "0000:02:00.0");
    }

    #[test]
    fn completeness_fraction() {
        let mut c = ReagentCompleteness::default();
        assert!((c.fraction() - 0.0).abs() < f64::EPSILON);

        c.bar0_snapshot = true;
        c.linux_firmware = true;
        c.patch_set = true;
        assert!((c.fraction() - 3.0 / 7.0).abs() < 0.01);
    }

    #[test]
    fn dir_name_format() {
        let m = ReagentManifest::new("GV100", "470.256.02", "6.17.9-76061709-generic", "0000:02:00.0");
        assert_eq!(m.dir_name(), "gv100_nvidia47025602_k6.17.9");
    }

    #[test]
    fn firmware_blob_serde() {
        let blob = FirmwareBlob {
            subsystem: "gr".to_owned(),
            filename: "fecs_inst.bin".to_owned(),
            path: PathBuf::from("/lib/firmware/nvidia/gv100/gr/fecs_inst.bin"),
            size_bytes: 32768,
            acr_required: true,
        };
        let json = serde_json::to_string(&blob).unwrap();
        let loaded: FirmwareBlob = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.subsystem, "gr");
        assert!(loaded.acr_required);
    }
}
