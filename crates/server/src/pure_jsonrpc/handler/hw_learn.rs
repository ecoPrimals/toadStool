// SPDX-License-Identifier: AGPL-3.0-only
//! Hardware learning JSON-RPC handlers.
//!
//! Exposes the hw-learn pipeline (observe → distill → apply → share) as
//! JSON-RPC methods under the `compute.hardware.*` domain, matching the
//! biomeOS v2.30 capability registration.

use crate::pure_jsonrpc::types::JsonRpcError;
use std::path::PathBuf;

/// Handler for `compute.hardware.*` JSON-RPC methods.
///
/// Wraps the hw-learn pipeline and nvpmu firmware inventory.
/// All methods are stateless — the `KnowledgeStore` persists recipes to disk.
pub struct HwLearnHandler {
    store_dir: PathBuf,
}

impl HwLearnHandler {
    #[must_use]
    pub fn new() -> Self {
        let store_dir = dirs_for_store();
        Self { store_dir }
    }

    fn open_store(&self) -> Result<hw_learn::knowledge::KnowledgeStore, JsonRpcError> {
        hw_learn::knowledge::KnowledgeStore::open(&self.store_dir)
            .map_err(|e| JsonRpcError::internal_error(format!("Failed to open recipe store: {e}")))
    }

    /// `compute.hardware.observe` — Parse an mmiotrace into structured events.
    ///
    /// Params: `{ "trace_data": "<mmiotrace text>", "mode": "mmiotrace" }`
    /// Returns: `{ "events_count": N, "gpu_id": ..., "driver": ... }`
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

    /// `compute.hardware.share_recipe` — Save, load, or list recipes.
    ///
    /// Save: `{ "action": "save", "recipe_json": "..." }`
    /// Load: `{ "action": "load", "recipe_id": "..." }`
    /// List: `{ "action": "list" }`
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

    /// `compute.hardware.status` — Report hw-learn pipeline state and firmware inventory.
    ///
    /// Params: `{ "chip": "gv100" }` (optional)
    /// Returns: `{ "pipeline": ..., "firmware": ..., "recipes": ... }`
    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    pub async fn hw_learn_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let store = self.open_store().ok();
        let arch_count = store.as_ref().map(|s| s.architectures().len()).unwrap_or(0);

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
                "phase": "3 — BAR0 live apply available",
                "stages": ["observe", "distill", "apply", "share_recipe"],
                "bar0_live_apply": true,
                "bar0_requires": "gpu-mmio group or root (run setup-gpu-sovereign.sh)",
            },
            "recipes": {
                "stored_architectures": arch_count,
            },
            "firmware": firmware,
            "gpus_detected": gpu_count,
        }))
    }
}

impl HwLearnHandler {
    /// `gpu.telemetry` — Report thermal and power data for all detected GPUs.
    ///
    /// Returns per-GPU temperature, power, safety status.
    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    pub async fn gpu_telemetry(
        &self,
        _params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let sysmon_gpus = toadstool_sysmon::discover_gpus();

        let mut gpu_entries = Vec::new();
        for gpu in &sysmon_gpus {
            let telemetry = gpu.telemetry();
            let safety = check_thermal_for_bdf(&gpu.pci_slot)
                .unwrap_or(nvpmu::SafetyStatus::Unknown);

            gpu_entries.push(serde_json::json!({
                "card_index": gpu.card_index,
                "driver": gpu.driver,
                "pci_slot": gpu.pci_slot,
                "vendor": format!("{:?}", gpu.vendor),
                "device_id": format!("{:#06x}", gpu.device_id),
                "temperature_celsius": telemetry.temperature_celsius,
                "power_watts": telemetry.power_watts,
                "power_cap_watts": telemetry.power_cap_watts,
                "core_clock_mhz": telemetry.core_clock_mhz,
                "fan_rpm": telemetry.fan_rpm,
                "utilization_percent": telemetry.utilization_percent,
                "vram_total_bytes": telemetry.vram_total_bytes,
                "vram_used_bytes": telemetry.vram_used_bytes,
                "safety_status": format!("{:?}", safety),
                "compute_safe": safety.compute_safe(),
            }));
        }

        Ok(serde_json::json!({
            "domain": "gpu",
            "operation": "telemetry",
            "gpus": gpu_entries,
            "gpu_count": gpu_entries.len(),
        }))
    }

    /// `compute.hardware.auto_init` — Auto-detect GPU, find best recipe, apply.
    ///
    /// This wires Gap 5 end-to-end: discover GPU → knowledge store → BAR0 apply.
    ///
    /// Params: `{ "bdf": "..." }` (optional, auto-detects if omitted)
    ///         `{ "dry_run": true }` (optional, default false)
    /// Returns: `{ "gpu": ..., "recipe_id": ..., "result": ... }`
    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    pub async fn hw_learn_auto_init(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let gpus = nvpmu::pci::discover_gpus()
            .map_err(|e| JsonRpcError::internal_error(format!("GPU discovery failed: {e}")))?;

        if gpus.is_empty() {
            return Err(JsonRpcError::internal_error("No NVIDIA GPUs found"));
        }

        let bdf = params
            .and_then(|p| p.get("bdf"))
            .and_then(serde_json::Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| gpus[0].bdf.clone());

        let gpu = gpus.iter().find(|g| g.bdf == bdf).ok_or_else(|| {
            JsonRpcError::invalid_params(format!("GPU {bdf} not found in PCI scan"))
        })?;

        let chip = gpu.chip.as_deref().unwrap_or("unknown");
        let target_arch = hw_learn::distiller::GpuArch {
            vendor: hw_learn::distiller::Vendor::Nvidia,
            generation: String::new(),
            chip: chip.to_string(),
            compute_class: String::new(),
        };

        let store = self.open_store()?;
        let recipe_id = store.best_recipe(&target_arch).ok_or_else(|| {
            JsonRpcError::internal_error(format!(
                "No recipe found for {chip}. Run compute.hardware.distill first \
                 to create a recipe from mmiotraces."
            ))
        })?;

        let recipe = store
            .load(&recipe_id)
            .map_err(|e| {
                JsonRpcError::internal_error(format!("Failed to load recipe {recipe_id}: {e}"))
            })?
            .ok_or_else(|| {
                JsonRpcError::internal_error(format!("Recipe {recipe_id} disappeared from store"))
            })?;

        let dry_run = params
            .and_then(|p| p.get("dry_run"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let card_path = params
            .and_then(|p| p.get("card_path"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("/dev/dri/card0");

        if dry_run {
            let mut applicator = hw_learn::RecipeApplicator::new(true);
            let result = applicator.apply(&recipe, card_path);

            return Ok(serde_json::json!({
                "domain": "compute.hardware",
                "operation": "auto_init",
                "mode": "dry_run",
                "gpu": { "bdf": bdf, "chip": chip },
                "recipe_id": recipe_id,
                "verdict": format!("{:?}", result.verdict),
                "steps_total": result.steps_total,
            }));
        }

        let thermal = check_thermal_for_bdf(&bdf);
        if let Some(ref status) = thermal {
            if !status.compute_safe() {
                return Err(JsonRpcError::internal_error(format!(
                    "GPU {bdf} thermal status {:?} — refusing auto_init",
                    status
                )));
            }
        }

        let mut bar0 = nvpmu::Bar0Access::open(&bdf).map_err(|e| {
            JsonRpcError::internal_error(format!(
                "BAR0 open failed for {bdf}: {e}. Run setup-gpu-sovereign.sh."
            ))
        })?;

        let snapshot = nvpmu::RegisterSnapshot::capture(&recipe, &bar0);

        let mut applicator = hw_learn::RecipeApplicator::new(false).with_register_access(&mut bar0);
        let result = applicator.apply(&recipe, card_path);

        let confidence = match result.verdict {
            hw_learn::applicator::ApplyVerdict::Success => 1.0,
            hw_learn::applicator::ApplyVerdict::PartialSuccess => 0.5,
            _ => 0.0,
        };

        let mut rollback_info = serde_json::json!(null);
        if result.verdict != hw_learn::applicator::ApplyVerdict::Success
            && !snapshot.is_empty()
        {
            tracing::warn!(bdf = %bdf, "auto_init failed — attempting rollback");
            let rollback_ok = snapshot.rollback(&mut bar0);
            rollback_info = serde_json::json!({
                "attempted": true,
                "succeeded": rollback_ok,
                "registers": snapshot.len(),
            });
        }

        if let Ok(mut store) = self.open_store() {
            let _ = store.update_confidence(&recipe_id, confidence);
        }

        Ok(serde_json::json!({
            "domain": "compute.hardware",
            "operation": "auto_init",
            "mode": "live",
            "gpu": { "bdf": bdf, "chip": chip, "driver": gpu.driver },
            "recipe_id": recipe_id,
            "thermal_checked": thermal.is_some(),
            "verdict": format!("{:?}", result.verdict),
            "steps_executed": result.steps_executed,
            "steps_total": result.steps_total,
            "confidence_updated": confidence,
            "rollback": rollback_info,
            "result": serde_json::to_value(&result).unwrap_or_default(),
        }))
    }
}

/// Resolve a PCI BDF address — either from params or auto-detect the first NVIDIA GPU.
fn resolve_bdf(params: &serde_json::Value) -> Result<String, JsonRpcError> {
    if let Some(bdf) = params.get("bdf").and_then(serde_json::Value::as_str) {
        return Ok(bdf.to_string());
    }

    let gpus = nvpmu::pci::discover_gpus()
        .map_err(|e| JsonRpcError::internal_error(format!("GPU discovery failed: {e}")))?;

    gpus.first()
        .map(|g| g.bdf.clone())
        .ok_or_else(|| JsonRpcError::internal_error("No NVIDIA GPUs found for BAR0 access"))
}

fn observe_from_text(
    text: &str,
    label: &str,
) -> Result<hw_learn::observer::ObserveResult, JsonRpcError> {
    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("hw-learn-{label}-{}.txt", std::process::id()));
    std::fs::write(&tmp_path, text)
        .map_err(|e| JsonRpcError::internal_error(format!("write temp: {e}")))?;

    let config = hw_learn::observer::ObserveConfig {
        mode: hw_learn::observer::TraceMode::MmioTrace,
        trace_path: Some(tmp_path.clone()),
        gpu_selector: hw_learn::observer::GpuSelector::Auto,
        trigger_compute: false,
    };

    let result = hw_learn::TraceObserver::observe(&config)
        .map_err(|e| JsonRpcError::invalid_params(format!("Failed to parse {label} trace: {e}")));

    let _ = std::fs::remove_file(&tmp_path);
    result
}

/// Check thermal safety for a GPU before dispatch.
///
/// Attempts to read hwmon sensors for the given BDF. Returns `None` if
/// sensors are unavailable (e.g. no hwmon, proprietary driver).
fn check_thermal_for_bdf(bdf: &str) -> Option<nvpmu::SafetyStatus> {
    let device_path = std::path::PathBuf::from(format!("/sys/bus/pci/devices/{bdf}"));
    let config = nvpmu::MonitorConfig::default();
    match nvpmu::monitor::sample(&device_path, &config) {
        Ok(sample) => {
            if sample.status != nvpmu::SafetyStatus::Normal {
                tracing::warn!(
                    bdf,
                    status = ?sample.status,
                    "GPU thermal status is not Normal"
                );
            }
            Some(sample.status)
        }
        Err(_) => {
            tracing::debug!(bdf, "hwmon sensors unavailable — skipping thermal check");
            None
        }
    }
}

fn dirs_for_store() -> PathBuf {
    if let Ok(dir) = std::env::var("TOADSTOOL_HW_LEARN_STORE") {
        return PathBuf::from(dir);
    }

    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg)
            .join("toadstool")
            .join("hw-learn-recipes");
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("toadstool")
            .join("hw-learn-recipes");
    }

    PathBuf::from("hw-learn-recipes")
}
