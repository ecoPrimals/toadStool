// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign warm handoff orchestrator — the diesel engine's driver rotation pipeline.
//!
//! Composes kernel module management, binary patching, sysfs driver
//! bind/unbind, and tier classification into a single operation. The
//! operator makes one RPC call; the daemon handles everything.
//!
//! # Pipeline
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │ 1. Module Preparation                                               │
//! │    Patched: find stock .ko → binary-patch → insmod                 │
//! │    System:  verify module loaded (or load it)                       │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 2. Seeder Bind                                                      │
//! │    unbind current driver → driver_override → drivers_probe         │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 3. Settle                                                           │
//! │    Wait for seeder hardware initialization                          │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 4. Bridge Pin + FLR Disable                                         │
//! │    Pin ancestor bridge power, disable FLR for warm swap             │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 5. Warm Swap                                                        │
//! │    unbind seeder (teardown NOP'd) → driver_override → bind vfio    │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 6. Tier Classification                                              │
//! │    BAR0 register probes → SovereignTier determination               │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ 7. Module Cleanup                                                   │
//! │    rmmod patched module (if we loaded it), delete /tmp/.ko          │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::vfio::kmod;
use crate::vfio::module_patch::{self, PatchSet, ModulePatchResult};
use crate::vfio::sovereign_tiers::{TierEvidence, classify_tier};

/// Configuration for a sovereign warm handoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffConfig {
    /// Target PCI BDF (e.g., "0000:02:00.0").
    pub bdf: String,

    /// Seeder driver name for sysfs bind (e.g., "nouveau").
    pub seeder_driver: String,

    /// Kernel module name (e.g., "nouveau").
    pub module_name: String,

    /// Module source strategy.
    pub module_source: ModuleSourceConfig,

    /// How long to wait after seeder binds before warm-swapping.
    pub settle: Duration,

    /// Final driver target (e.g., "vfio-pci").
    pub final_driver: String,
}

/// Module source configuration (cylinder-side, no glowplug dependency).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleSourceConfig {
    /// Module already loaded or loadable via the system.
    System,
    /// Binary-patch a stock module before loading.
    Patched {
        /// Stock module name for `modinfo -n` lookup.
        stock_module: String,
        /// Patch set name (resolved by `PatchSet::by_name`).
        patch_set: String,
    },
}

/// Result of a sovereign warm handoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffResult {
    /// Target BDF.
    pub bdf: String,
    /// Whether the full pipeline succeeded.
    pub success: bool,
    /// Which step halted the pipeline (if any).
    pub halted_at: Option<String>,
    /// Per-step outcomes.
    pub steps: Vec<HandoffStep>,
    /// Module patch result (if patching was used).
    pub patch_result: Option<ModulePatchResult>,
    /// Tier classification after handoff (if we got far enough).
    pub tier: Option<TierEvidence>,
    /// Whether a module was loaded by this handoff.
    pub module_loaded: bool,
    /// Whether the module was successfully unloaded after handoff.
    pub module_unloaded: bool,
    /// Total wall-clock time in milliseconds.
    pub total_ms: u64,
}

/// One step in the handoff pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffStep {
    pub name: String,
    pub ok: bool,
    pub detail: Option<String>,
    pub duration_ms: u64,
}

impl HandoffConfig {
    /// Create a config for Titan V warm handoff via patched nouveau.
    #[must_use]
    pub fn nouveau_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nouveau".into(),
            module_name: "nouveau".into(),
            module_source: ModuleSourceConfig::Patched {
                stock_module: "nouveau".into(),
                patch_set: "volta_warm_handoff".into(),
            },
            settle: Duration::from_secs(5),
            final_driver: "vfio-pci".into(),
        }
    }

    /// Create a config for K80 warm handoff via stock nouveau.
    #[must_use]
    pub fn nouveau_k80(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nouveau".into(),
            module_name: "nouveau".into(),
            module_source: ModuleSourceConfig::System,
            settle: Duration::from_secs(5),
            final_driver: "vfio-pci".into(),
        }
    }

    /// Resolve a config from a strategy name and BDF.
    #[must_use]
    pub fn from_strategy(strategy: &str, bdf: &str) -> Option<Self> {
        match strategy {
            "nouveau_titanv" => Some(Self::nouveau_titanv(bdf)),
            "nouveau_k80" => Some(Self::nouveau_k80(bdf)),
            _ => None,
        }
    }
}

/// Execute the full sovereign warm handoff pipeline.
///
/// This is the top-level entry point called from the dispatch handler.
/// It manages the entire lifecycle: module prep → bind → settle →
/// swap → classify → cleanup.
///
/// The optional `bar0` parameter is used for post-handoff tier
/// classification. If `None`, tier classification is skipped.
pub fn execute_handoff(
    config: &HandoffConfig,
    bar0: Option<&crate::vfio::device::MappedBar>,
) -> HandoffResult {
    let overall = Instant::now();
    let mut steps = Vec::new();
    let module_loaded;
    let mut patch_result = None;

    // ── Step 1: Module Preparation ──────────────────────────────────

    let t = Instant::now();
    match &config.module_source {
        ModuleSourceConfig::Patched { stock_module, patch_set } => {
            // If module is already loaded (e.g., from a previous run), unload first
            if kmod::is_module_loaded(&config.module_name) {
                tracing::info!(
                    module = config.module_name.as_str(),
                    "module already loaded — unloading before patched load"
                );
                if let Err(e) = kmod::unload_module(&config.module_name) {
                    steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("cannot unload existing {}: {e}", config.module_name)),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall);
                }
            }

            let ps = match PatchSet::by_name(patch_set) {
                Some(ps) => ps,
                None => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("unknown patch set: {patch_set}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall);
                }
            };

            let stock_path = match kmod::find_stock_module(stock_module) {
                Ok(p) => p,
                Err(e) => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("stock module lookup failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall);
                }
            };

            match module_patch::patch_module(&stock_path, &ps) {
                Ok(pr) => {
                    let patched_path = PathBuf::from(&pr.patched_path);
                    patch_result = Some(pr);

                    if let Err(e) = kmod::load_module(&patched_path) {
                        steps.push(HandoffStep {
                            name: "module_prep".into(),
                            ok: false,
                            detail: Some(format!("insmod patched module failed: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return halt_result(&config.bdf, "module_prep", steps, patch_result, false, false, overall);
                    }
                    module_loaded = true;
                }
                Err(e) => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("module patching failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall);
                }
            }

            steps.push(HandoffStep {
                name: "module_prep".into(),
                ok: true,
                detail: Some("patched module loaded".into()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        ModuleSourceConfig::System => {
            match kmod::ensure_module_loaded(&config.module_name) {
                Ok(freshly_loaded) => {
                    module_loaded = freshly_loaded;
                    steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: true,
                        detail: Some(if freshly_loaded {
                            "system module loaded".into()
                        } else {
                            "system module already present".into()
                        }),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("system module load failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall);
                }
            }
        }
    }

    // ── Step 2: Unbind current driver ───────────────────────────────

    let t = Instant::now();
    let prev_driver = read_current_driver(&config.bdf);
    if let Some(ref current) = prev_driver {
        let unbind_path = format!(
            "{}/bus/pci/drivers/{current}/unbind",
            crate::linux_paths::sysfs_root()
        );
        if let Err(e) = sysfs_write(&unbind_path, &config.bdf) {
            tracing::warn!(bdf = config.bdf.as_str(), driver = current.as_str(), error = %e, "unbind failed (continuing)");
        }
    }
    steps.push(HandoffStep {
        name: "unbind_current".into(),
        ok: true,
        detail: prev_driver.map(|d| format!("was: {d}")),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    // ── Step 3: Bind seeder driver ──────────────────────────────────

    let t = Instant::now();
    let override_path = crate::linux_paths::sysfs_pci_device_file(&config.bdf, "driver_override");
    if let Err(e) = sysfs_write(&override_path, &config.seeder_driver) {
        steps.push(HandoffStep {
            name: "seeder_bind".into(),
            ok: false,
            detail: Some(format!("driver_override failed: {e}")),
            duration_ms: t.elapsed().as_millis() as u64,
        });
        return halt_result(&config.bdf, "seeder_bind", steps, patch_result, module_loaded, false, overall);
    }

    let probe_path = crate::linux_paths::sysfs_pci_drivers_probe();
    if let Err(e) = sysfs_write(&probe_path, &config.bdf) {
        steps.push(HandoffStep {
            name: "seeder_bind".into(),
            ok: false,
            detail: Some(format!("drivers_probe failed: {e}")),
            duration_ms: t.elapsed().as_millis() as u64,
        });
        return halt_result(&config.bdf, "seeder_bind", steps, patch_result, module_loaded, false, overall);
    }

    let bound = read_current_driver(&config.bdf);
    let bind_ok = bound.as_deref() == Some(config.seeder_driver.as_str());
    steps.push(HandoffStep {
        name: "seeder_bind".into(),
        ok: bind_ok,
        detail: Some(format!(
            "driver={} expected={}",
            bound.as_deref().unwrap_or("none"),
            config.seeder_driver
        )),
        duration_ms: t.elapsed().as_millis() as u64,
    });
    if !bind_ok {
        return halt_result(&config.bdf, "seeder_bind", steps, patch_result, module_loaded, false, overall);
    }

    // ── Step 4: Settle — wait for hardware initialization ───────────

    let t = Instant::now();
    tracing::info!(
        bdf = config.bdf.as_str(),
        seeder = config.seeder_driver.as_str(),
        settle_ms = config.settle.as_millis() as u64,
        "waiting for seeder hardware initialization"
    );
    std::thread::sleep(config.settle);
    steps.push(HandoffStep {
        name: "seeder_settle".into(),
        ok: true,
        detail: Some(format!("{}ms settle", config.settle.as_millis())),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    // ── Step 5: Pin bridges + disable FLR ───────────────────────────

    let t = Instant::now();
    pin_bridge_hierarchy(&config.bdf);
    disable_flr(&config.bdf);
    steps.push(HandoffStep {
        name: "prepare_warm_swap".into(),
        ok: true,
        detail: Some("bridge pinned, FLR disabled".into()),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    // ── Step 6: Warm swap — seeder → final driver ───────────────────

    let t = Instant::now();
    if let Some(ref current) = read_current_driver(&config.bdf) {
        let unbind_path = format!(
            "{}/bus/pci/drivers/{current}/unbind",
            crate::linux_paths::sysfs_root()
        );
        if let Err(e) = sysfs_write(&unbind_path, &config.bdf) {
            steps.push(HandoffStep {
                name: "warm_swap".into(),
                ok: false,
                detail: Some(format!("unbind {current} failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return halt_result(&config.bdf, "warm_swap", steps, patch_result, module_loaded, false, overall);
        }
    }

    if let Err(e) = sysfs_write(&override_path, &config.final_driver) {
        steps.push(HandoffStep {
            name: "warm_swap".into(),
            ok: false,
            detail: Some(format!("override to {} failed: {e}", config.final_driver)),
            duration_ms: t.elapsed().as_millis() as u64,
        });
        return halt_result(&config.bdf, "warm_swap", steps, patch_result, module_loaded, false, overall);
    }
    let _ = sysfs_write(&probe_path, &config.bdf);

    let final_bound = read_current_driver(&config.bdf);
    let swap_ok = final_bound.as_deref() == Some(config.final_driver.as_str());
    steps.push(HandoffStep {
        name: "warm_swap".into(),
        ok: swap_ok,
        detail: Some(format!(
            "{} → {} (warm_preserved={})",
            config.seeder_driver,
            final_bound.as_deref().unwrap_or("none"),
            swap_ok
        )),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    if !swap_ok {
        return halt_result(&config.bdf, "warm_swap", steps, patch_result, module_loaded, false, overall);
    }

    // ── Step 7: Tier Classification ─────────────────────────────────

    let tier = if let Some(b) = bar0 {
        let t = Instant::now();
        let evidence = classify_tier(b);
        steps.push(HandoffStep {
            name: "tier_classify".into(),
            ok: true,
            detail: Some(format!("{}", evidence.tier)),
            duration_ms: t.elapsed().as_millis() as u64,
        });
        Some(evidence)
    } else {
        // Try sysfs BAR0 fallback
        let t = Instant::now();
        match crate::vfio::device::MappedBar::from_sysfs_rw(&config.bdf, 16 * 1024 * 1024) {
            Ok(sysfs_bar) => {
                let evidence = classify_tier(&sysfs_bar);
                steps.push(HandoffStep {
                    name: "tier_classify".into(),
                    ok: true,
                    detail: Some(format!("{} (via sysfs)", evidence.tier)),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                Some(evidence)
            }
            Err(e) => {
                steps.push(HandoffStep {
                    name: "tier_classify".into(),
                    ok: false,
                    detail: Some(format!("BAR0 access failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                None
            }
        }
    };

    // ── Step 8: Module Cleanup ──────────────────────────────────────

    let mut module_unloaded = false;
    if module_loaded {
        let t = Instant::now();
        match kmod::unload_module(&config.module_name) {
            Ok(()) => {
                module_unloaded = true;
                let _ = module_patch::cleanup_patched_module(&config.module_name);
                steps.push(HandoffStep {
                    name: "module_cleanup".into(),
                    ok: true,
                    detail: Some(format!("rmmod {} + tmpfile removed", config.module_name)),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                tracing::warn!(
                    module = config.module_name.as_str(),
                    error = %e,
                    "module cleanup failed (non-fatal)"
                );
                steps.push(HandoffStep {
                    name: "module_cleanup".into(),
                    ok: false,
                    detail: Some(format!("rmmod failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    HandoffResult {
        bdf: config.bdf.clone(),
        success: true,
        halted_at: None,
        steps,
        patch_result,
        tier,
        module_loaded,
        module_unloaded,
        total_ms: overall.elapsed().as_millis() as u64,
    }
}

fn halt_result(
    bdf: &str,
    halted_at: &str,
    steps: Vec<HandoffStep>,
    patch_result: Option<ModulePatchResult>,
    module_loaded: bool,
    module_unloaded: bool,
    start: Instant,
) -> HandoffResult {
    HandoffResult {
        bdf: bdf.into(),
        success: false,
        halted_at: Some(halted_at.into()),
        steps,
        patch_result,
        tier: None,
        module_loaded,
        module_unloaded,
        total_ms: start.elapsed().as_millis() as u64,
    }
}

// ── Sysfs helpers (minimal, no glowplug dependency) ─────────────────

fn read_current_driver(bdf: &str) -> Option<String> {
    let link = crate::linux_paths::sysfs_pci_device_file(bdf, "driver");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

fn sysfs_write(path: &str, value: &str) -> Result<(), String> {
    std::fs::write(path, value).map_err(|e| format!("{path}: {e}"))
}

/// Walk the sysfs device path upward, pinning `power/control=on` and
/// `d3cold_allowed=0` on every ancestor PCI bridge.
fn pin_bridge_hierarchy(bdf: &str) {
    let device_link = crate::linux_paths::sysfs_pci_device_path(bdf);
    let Ok(canonical) = std::fs::canonicalize(&device_link) else {
        return;
    };

    let mut current = canonical.as_path();
    while let Some(parent) = current.parent() {
        let power_control = parent.join("power/control");
        if power_control.exists() {
            let _ = std::fs::write(&power_control, "on");
        }
        let d3cold = parent.join("d3cold_allowed");
        if d3cold.exists() {
            let _ = std::fs::write(&d3cold, "0");
        }

        if !parent.join("vendor").exists() {
            break;
        }
        current = parent;
    }
}

/// Disable Function Level Reset for warm-preserving swaps.
fn disable_flr(bdf: &str) {
    let reset_path = crate::linux_paths::sysfs_pci_device_file(bdf, "reset_method");
    if Path::new(&reset_path).exists() {
        let _ = std::fs::write(&reset_path, "");
    }
}

impl std::fmt::Display for HandoffResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.success {
            let tier_str = self
                .tier
                .as_ref()
                .map(|t| format!(" → {}", t.tier))
                .unwrap_or_default();
            write!(
                f,
                "HANDOFF OK ({}{}, {}ms)",
                self.bdf, tier_str, self.total_ms
            )
        } else {
            write!(
                f,
                "HANDOFF HALTED@{} ({}, {}ms)",
                self.halted_at.as_deref().unwrap_or("?"),
                self.bdf,
                self.total_ms
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_strategy_resolves_known() {
        assert!(HandoffConfig::from_strategy("nouveau_titanv", "0000:02:00.0").is_some());
        assert!(HandoffConfig::from_strategy("nouveau_k80", "0000:49:00.0").is_some());
        assert!(HandoffConfig::from_strategy("unknown", "0000:02:00.0").is_none());
    }

    #[test]
    fn titanv_config_uses_patched_source() {
        let cfg = HandoffConfig::nouveau_titanv("0000:02:00.0");
        assert!(matches!(cfg.module_source, ModuleSourceConfig::Patched { .. }));
        assert_eq!(cfg.seeder_driver, "nouveau");
        assert_eq!(cfg.final_driver, "vfio-pci");
    }

    #[test]
    fn k80_config_uses_system_source() {
        let cfg = HandoffConfig::nouveau_k80("0000:49:00.0");
        assert!(matches!(cfg.module_source, ModuleSourceConfig::System));
    }

    #[test]
    fn handoff_result_display_success() {
        let r = HandoffResult {
            bdf: "0000:02:00.0".into(),
            success: true,
            halted_at: None,
            steps: vec![],
            patch_result: None,
            tier: Some(TierEvidence {
                tier: crate::vfio::sovereign_tiers::SovereignTier::WarmInfrastructure,
                pmc_enable: 0xFFFF_FFFF,
                pmc_popcount: 32,
                pramin_accessible: true,
                fecs_pc: Some(0),
                gpc_enables: None,
                ce_status: None,
                gr_status: None,
                pbdma_intr: None,
                ce_runlist: None,
            }),
            module_loaded: true,
            module_unloaded: true,
            total_ms: 6000,
        };
        let s = r.to_string();
        assert!(s.contains("HANDOFF OK"));
        assert!(s.contains("Tier 1"));
    }

    #[test]
    fn handoff_result_display_halted() {
        let r = HandoffResult {
            bdf: "0000:02:00.0".into(),
            success: false,
            halted_at: Some("seeder_bind".into()),
            steps: vec![],
            patch_result: None,
            tier: None,
            module_loaded: false,
            module_unloaded: false,
            total_ms: 100,
        };
        let s = r.to_string();
        assert!(s.contains("HALTED@seeder_bind"));
    }

    #[test]
    fn handoff_result_serde_roundtrip() {
        let r = HandoffResult {
            bdf: "0000:02:00.0".into(),
            success: true,
            halted_at: None,
            steps: vec![HandoffStep {
                name: "module_prep".into(),
                ok: true,
                detail: Some("patched module loaded".into()),
                duration_ms: 200,
            }],
            patch_result: None,
            tier: None,
            module_loaded: true,
            module_unloaded: true,
            total_ms: 5000,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: HandoffResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bdf, "0000:02:00.0");
        assert!(back.success);
        assert_eq!(back.steps.len(), 1);
    }
}
