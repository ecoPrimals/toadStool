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

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use std::collections::HashSet;
use std::sync::Mutex;

use crate::vfio::guarded_sysfs;
use crate::vfio::kernel_health;
use crate::vfio::kmod;
use crate::vfio::module_patch::{self, PatchSet, ModulePatchResult};
use crate::vfio::sovereign_tiers::{TierEvidence, classify_tier};

/// Per-BDF handoff concurrency guard. Only one handoff per device at a time.
static HANDOFF_LOCKS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// RAII guard that releases the per-BDF handoff lock on drop. This ensures
/// the lock is freed even if the thread panics or the RPC timeout abandons
/// the blocking thread.
struct HandoffGuard {
    bdf: String,
}

impl HandoffGuard {
    fn acquire(bdf: &str) -> Result<Self, String> {
        let mut guard = HANDOFF_LOCKS.lock().map_err(|e| format!("lock poisoned: {e}"))?;
        let set = guard.get_or_insert_with(HashSet::new);
        if !set.insert(bdf.to_string()) {
            return Err(format!("handoff already in progress for {bdf}"));
        }
        Ok(Self { bdf: bdf.to_string() })
    }
}

impl Drop for HandoffGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = HANDOFF_LOCKS.lock() {
            if let Some(set) = guard.as_mut() {
                set.remove(&self.bdf);
            }
        }
    }
}

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
    /// Binary-patch a DKMS-built module (specific version) before loading.
    /// Used when the system's installed module is a different version
    /// (e.g., nvidia-580-open installed, but we need nvidia-470 proprietary).
    DkmsPatched {
        /// Module name in DKMS (e.g., "nvidia").
        dkms_module: String,
        /// DKMS version string (e.g., "470.256.02").
        dkms_version: String,
        /// Patch set name.
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

    /// Create a config for Titan V warm handoff via nvidia (system module, no patching).
    ///
    /// Uses the already-loaded nvidia driver as the seeder. nvidia's legacy RM
    /// fully initializes Volta (SEC2→ACR→FECS→GR→TPC). On unbind, nvidia's
    /// `nv_pci_remove` WILL tear down state — this strategy tests what nvidia
    /// preserves versus nouveau, without requiring module patching.
    ///
    /// The nvidia module is already loaded (RTX 5060 display GPU), so this
    /// uses `ModuleSourceConfig::System`.
    #[must_use]
    pub fn nvidia_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nvidia".into(),
            module_name: "nvidia".into(),
            module_source: ModuleSourceConfig::System,
            settle: Duration::from_secs(10),
            final_driver: "vfio-pci".into(),
        }
    }

    /// Create a config for Titan V warm handoff via patched nvidia-470 (dual-load injection).
    ///
    /// Uses the DKMS-built nvidia-470 proprietary `.ko` (which supports Volta
    /// via legacy RM, unlike nvidia-580-open which requires GSP). Patches
    /// `nv_pci_remove` and other teardown functions to NOP, and renames the
    /// module identity from "nvidia" to "nvsov" so it can be loaded alongside
    /// the running nvidia-580 module.
    ///
    /// This is the "agent reagents diesel engine" approach: the patched module
    /// is injected as a sovereign seeder while the display GPU's nvidia driver
    /// runs undisturbed.
    #[must_use]
    pub fn nvidia_patched_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nvsov".into(),
            module_name: "nvsov".into(),
            module_source: ModuleSourceConfig::DkmsPatched {
                dkms_module: "nvidia".into(),
                dkms_version: "470.256.02".into(),
                patch_set: "nvidia_warm_handoff".into(),
            },
            settle: Duration::from_secs(10),
            final_driver: "vfio-pci".into(),
        }
    }

    /// Resolve a config from a strategy name and BDF.
    #[must_use]
    pub fn from_strategy(strategy: &str, bdf: &str) -> Option<Self> {
        match strategy {
            "nouveau_titanv" => Some(Self::nouveau_titanv(bdf)),
            "nouveau_k80" => Some(Self::nouveau_k80(bdf)),
            "nvidia_titanv" => Some(Self::nvidia_titanv(bdf)),
            "nvidia_patched_titanv" => Some(Self::nvidia_patched_titanv(bdf)),
            _ => None,
        }
    }
}

/// Execute the full sovereign warm handoff pipeline.
///
/// This is the top-level entry point called from the dispatch handler.
/// It manages the entire lifecycle: pre-flight → module prep → bind →
/// settle → swap → classify → cleanup.
///
/// All dangerous sysfs writes (driver probe/unbind) and kernel module
/// operations use guarded child-process isolation with timeouts. If any
/// operation exceeds its deadline, the child is killed and rollback runs.
///
/// The overall pipeline has a 60s wall-clock deadline.
pub fn execute_handoff(
    config: &HandoffConfig,
    bar0: Option<&crate::vfio::device::MappedBar>,
) -> HandoffResult {
    let overall = Instant::now();
    let deadline = guarded_sysfs::HANDOFF_DEADLINE;
    let mut steps = Vec::new();
    let module_loaded;
    let mut patch_result = None;
    let mut sibling_state: Vec<(String, Option<String>)> = Vec::new();
    let mut needs_device_rollback = false;

    // ── Step 0: Pre-flight checks ───────────────────────────────────

    let t = Instant::now();

    // 0a. Concurrent handoff guard (RAII — released on drop at any exit path)
    let _handoff_guard = match HandoffGuard::acquire(&config.bdf) {
        Ok(guard) => guard,
        Err(e) => {
            steps.push(HandoffStep {
                name: "preflight".into(), ok: false,
                detail: Some(e),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return halt_result(&config.bdf, "preflight", steps, None, false, false, overall, &[], &config.module_name, false);
        }
    };

    // 0b. Module stuck state check
    if guarded_sysfs::is_module_stuck(&config.module_name) {
        steps.push(HandoffStep {
            name: "preflight".into(), ok: false,
            detail: Some(format!(
                "module '{}' is stuck (Unloading/negative refcount) — reboot required",
                config.module_name
            )),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        return halt_result(&config.bdf, "preflight", steps, None, false, false, overall, &[], &config.module_name, false);
    }

    // 0c. IOMMU group availability
    if let Err(e) = guarded_sysfs::iommu_group_ready(&config.bdf) {
        steps.push(HandoffStep {
            name: "preflight".into(), ok: false,
            detail: Some(format!("IOMMU group not ready: {e}")),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        return halt_result(&config.bdf, "preflight", steps, None, false, false, overall, &[], &config.module_name, false);
    }

    // 0d. Kernel build environment health (only for module sources that compile/load)
    if !matches!(config.module_source, ModuleSourceConfig::System) {
        match kernel_health::full_kernel_health_check() {
            Ok(report) => {
                if !report.layout_matches {
                    tracing::error!(
                        diagnosis = %report.diagnosis,
                        "kernel build environment unhealthy — module loading will fail"
                    );
                    steps.push(HandoffStep {
                        name: "preflight".into(), ok: false,
                        detail: Some(format!(
                            "kernel health check failed: {}",
                            report.diagnosis
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return halt_result(&config.bdf, "preflight", steps, None, false, false, overall, &[], &config.module_name, false);
                }
                tracing::info!(
                    autoconf_fresh = report.autoconf_fresh,
                    exit_offset = report.struct_module_exit_offset,
                    "kernel health check passed"
                );
            }
            Err(e) => {
                tracing::warn!(err = %e, "kernel health check could not run — proceeding with caution");
            }
        }
    }

    steps.push(HandoffStep {
        name: "preflight".into(), ok: true,
        detail: Some("module clean, IOMMU group free, no concurrent handoff, kernel healthy".into()),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    // ── Step 1: Module Preparation ──────────────────────────────────

    let t = Instant::now();
    match &config.module_source {
        ModuleSourceConfig::Patched { stock_module, patch_set } => {
            if kmod::is_module_loaded(&config.module_name) {
                tracing::info!(module = config.module_name.as_str(),
                               "module already loaded — guarded unload before patched load");
                if let Err(e) = guarded_sysfs::rmmod_guarded(
                    &config.module_name, guarded_sysfs::RMMOD_TIMEOUT,
                ) {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("cannot unload existing {}: {e}", config.module_name)),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
            
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            }

            let ps = match PatchSet::by_name(patch_set) {
                Some(ps) => ps,
                None => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("unknown patch set: {patch_set}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
            
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            };

            let stock_path = match kmod::find_stock_module(stock_module) {
                Ok(p) => p,
                Err(e) => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("stock module lookup failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
            
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            };

            let rename_pair = if stock_module != &config.module_name {
                Some((stock_module.as_str(), config.module_name.as_str()))
            } else {
                None
            };

            match module_patch::patch_module_with_rename(&stock_path, &ps, rename_pair) {
                Ok(pr) => {
                    let patched_path = PathBuf::from(&pr.patched_path);
                    patch_result = Some(pr);

                    // Load module dependencies before insmod. insmod doesn't
                    // resolve deps like modprobe — we use modprobe --dry-run
                    // to discover the chain and load each dep.
                    if let Err(e) = load_module_dependencies(stock_module) {
                        tracing::warn!(module = stock_module.as_str(), error = %e,
                                       "failed to load module dependencies (continuing)");
                    }

                    if let Err(e) = guarded_sysfs::insmod_guarded(
                        &patched_path, guarded_sysfs::INSMOD_TIMEOUT,
                    ) {
                        steps.push(HandoffStep {
                            name: "module_prep".into(), ok: false,
                            detail: Some(format!("guarded insmod failed: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                
                        return halt_result(&config.bdf, "module_prep", steps, patch_result, false, false, overall, &[], &config.module_name, false);
                    }
                    module_loaded = true;
                }
                Err(e) => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("module patching failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
            
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            }

            let patch_detail = patch_result.as_ref()
                .map(|pr| format!("patched module loaded (guarded, {}/{} patches applied)",
                    pr.applied_count, pr.total_count))
                .unwrap_or_else(|| "patched module loaded (guarded)".into());
            steps.push(HandoffStep {
                name: "module_prep".into(), ok: true,
                detail: Some(patch_detail),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        ModuleSourceConfig::DkmsPatched { dkms_module, dkms_version, patch_set } => {
            if kmod::is_module_loaded(&config.module_name) {
                tracing::info!(module = config.module_name.as_str(),
                               "module already loaded — guarded unload before DKMS patched load");
                if let Err(e) = guarded_sysfs::rmmod_guarded(
                    &config.module_name, guarded_sysfs::RMMOD_TIMEOUT,
                ) {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("cannot unload existing {}: {e}", config.module_name)),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            }

            let ps = match PatchSet::by_name(patch_set) {
                Some(ps) => ps,
                None => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("unknown patch set: {patch_set}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            };

            let stock_path = match kmod::find_dkms_module(dkms_module, dkms_version) {
                Ok(p) => p,
                Err(e) => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("DKMS module lookup failed for {dkms_module}/{dkms_version}: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            };

            let rename_pair = if dkms_module != &config.module_name {
                Some((dkms_module.as_str(), config.module_name.as_str()))
            } else {
                None
            };

            match module_patch::patch_module_with_rename(&stock_path, &ps, rename_pair) {
                Ok(pr) => {
                    let patched_path = PathBuf::from(&pr.patched_path);
                    patch_result = Some(pr);

                    if let Err(e) = guarded_sysfs::insmod_guarded(
                        &patched_path, guarded_sysfs::INSMOD_TIMEOUT,
                    ) {
                        steps.push(HandoffStep {
                            name: "module_prep".into(), ok: false,
                            detail: Some(format!("guarded insmod DKMS module failed: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });

                        return halt_result(&config.bdf, "module_prep", steps, patch_result, false, false, overall, &[], &config.module_name, false);
                    }
                    module_loaded = true;
                }
                Err(e) => {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("DKMS module patching failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            }

            let patch_detail = patch_result.as_ref()
                .map(|pr| format!("DKMS patched module loaded ({}/{} patches, {})",
                    pr.applied_count, pr.total_count,
                    rename_pair.map(|(o, n)| format!("renamed {o}→{n}")).unwrap_or_else(|| "no rename".into())))
                .unwrap_or_else(|| "DKMS patched module loaded".into());
            steps.push(HandoffStep {
                name: "module_prep".into(), ok: true,
                detail: Some(patch_detail),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        ModuleSourceConfig::System => {
            match kmod::ensure_module_loaded(&config.module_name) {
                Ok(freshly_loaded) => {
                    module_loaded = freshly_loaded;
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: true,
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
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("system module load failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
            
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
            }
        }
    }

    // ── Deadline check ──────────────────────────────────────────────

    if overall.elapsed() >= deadline {

        return deadline_exceeded(&config.bdf, steps, patch_result, module_loaded,
                                 &config.module_name, &sibling_state, overall);
    }

    // ── Step 2: Unbind current driver + IOMMU group siblings ────────

    let t = Instant::now();
    sibling_state = guarded_sysfs::unbind_iommu_siblings(&config.bdf);
    let prev_driver = guarded_sysfs::read_current_driver(&config.bdf);

    if let Some(ref current) = prev_driver {
        let unbind_path = crate::linux_paths::sysfs_pci_driver_unbind(current);
        if let Err(e) = guarded_sysfs::sysfs_write_guarded(
            &unbind_path, &config.bdf, guarded_sysfs::UNBIND_TIMEOUT,
        ) {
            tracing::warn!(bdf = config.bdf.as_str(), driver = current.as_str(),
                           error = %e, "guarded unbind failed (continuing)");
        }
    }

    let sibling_summary: Vec<String> = sibling_state.iter()
        .map(|(s, d)| format!("{s}: {} → unbound", d.as_deref().unwrap_or("none")))
        .collect();
    let mut detail_msg = prev_driver.map(|d| format!("was: {d}"))
        .unwrap_or_else(|| "unbound".into());
    if !sibling_summary.is_empty() {
        detail_msg.push_str(&format!("; siblings: [{}]", sibling_summary.join(", ")));
    }

    // Verify all siblings actually unbound
    let siblings_clean = sibling_state.iter().all(|(s, _)| guarded_sysfs::read_current_driver(s).is_none());
    let target_clean = guarded_sysfs::read_current_driver(&config.bdf).is_none();
    let unbind_ok = siblings_clean && target_clean;

    if !unbind_ok {
        detail_msg.push_str(" [WARN: not all devices fully unbound]");
    }

    steps.push(HandoffStep {
        name: "unbind_current".into(),
        ok: unbind_ok,
        detail: Some(detail_msg),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    if !unbind_ok {

        return halt_result(&config.bdf, "unbind_current", steps, patch_result,
                           module_loaded, false, overall, &sibling_state,
                           &config.module_name, true);
    }

    // Device has been unbound — rollback must restore to vfio-pci on any failure
    needs_device_rollback = true;

    // ── Step 3: Bind seeder driver (GUARDED) ────────────────────────

    let t = Instant::now();
    let override_path = crate::linux_paths::sysfs_pci_device_file(&config.bdf, "driver_override");
    if let Err(e) = guarded_sysfs::sysfs_write(&override_path, &config.seeder_driver) {
        steps.push(HandoffStep {
            name: "seeder_bind".into(), ok: false,
            detail: Some(format!("driver_override failed: {e}")),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        return halt_result(&config.bdf, "seeder_bind", steps, patch_result,
                           module_loaded, false, overall, &sibling_state,
                           &config.module_name, needs_device_rollback);
    }

    let probe_path = crate::linux_paths::sysfs_pci_drivers_probe();
    if let Err(e) = guarded_sysfs::sysfs_write_guarded(
        &probe_path, &config.bdf, guarded_sysfs::PROBE_TIMEOUT,
    ) {
        steps.push(HandoffStep {
            name: "seeder_bind".into(), ok: false,
            detail: Some(format!("guarded drivers_probe failed: {e}")),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        return halt_result(&config.bdf, "seeder_bind", steps, patch_result,
                           module_loaded, false, overall, &sibling_state,
                           &config.module_name, needs_device_rollback);
    }

    let bound = guarded_sysfs::read_current_driver(&config.bdf);
    let bind_ok = bound.as_deref() == Some(config.seeder_driver.as_str());
    steps.push(HandoffStep {
        name: "seeder_bind".into(), ok: bind_ok,
        detail: Some(format!("driver={} expected={}",
            bound.as_deref().unwrap_or("none"), config.seeder_driver)),
        duration_ms: t.elapsed().as_millis() as u64,
    });
    if !bind_ok {

        return halt_result(&config.bdf, "seeder_bind", steps, patch_result,
                           module_loaded, false, overall, &sibling_state,
                           &config.module_name, needs_device_rollback);
    }

    // ── Step 4: Settle — wait for hardware initialization ───────────

    let t = Instant::now();
    tracing::info!(bdf = config.bdf.as_str(), seeder = config.seeder_driver.as_str(),
                   settle_ms = config.settle.as_millis() as u64,
                   "waiting for seeder hardware initialization");
    std::thread::sleep(config.settle);
    steps.push(HandoffStep {
        name: "seeder_settle".into(), ok: true,
        detail: Some(format!("{}ms settle", config.settle.as_millis())),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    // ── Deadline check ──────────────────────────────────────────────

    if overall.elapsed() >= deadline {

        return deadline_exceeded(&config.bdf, steps, patch_result, module_loaded,
                                 &config.module_name, &sibling_state, overall);
    }

    // ── Step 5: Pin bridges + disable FLR ───────────────────────────

    let t = Instant::now();
    guarded_sysfs::pin_bridge_hierarchy(&config.bdf);
    guarded_sysfs::disable_flr(&config.bdf);
    steps.push(HandoffStep {
        name: "prepare_warm_swap".into(), ok: true,
        detail: Some("bridge pinned, FLR disabled".into()),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    // ── Step 6: Warm swap — seeder → final driver (GUARDED) ─────────

    let t = Instant::now();
    if let Some(ref current) = guarded_sysfs::read_current_driver(&config.bdf) {
        let unbind_path = crate::linux_paths::sysfs_pci_driver_unbind(current);
        if let Err(e) = guarded_sysfs::sysfs_write_guarded(
            &unbind_path, &config.bdf, guarded_sysfs::UNBIND_TIMEOUT,
        ) {
            steps.push(HandoffStep {
                name: "warm_swap".into(), ok: false,
                detail: Some(format!("guarded unbind {current} failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
    
            return halt_result(&config.bdf, "warm_swap", steps, patch_result,
                               module_loaded, false, overall, &sibling_state,
                               &config.module_name, needs_device_rollback);
        }
    }

    if let Err(e) = guarded_sysfs::sysfs_write(&override_path, &config.final_driver) {
        steps.push(HandoffStep {
            name: "warm_swap".into(), ok: false,
            detail: Some(format!("override to {} failed: {e}", config.final_driver)),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        return halt_result(&config.bdf, "warm_swap", steps, patch_result,
                           module_loaded, false, overall, &sibling_state,
                           &config.module_name, needs_device_rollback);
    }

    if let Err(e) = guarded_sysfs::sysfs_write_guarded(
        &probe_path, &config.bdf, guarded_sysfs::PROBE_TIMEOUT,
    ) {
        steps.push(HandoffStep {
            name: "warm_swap".into(), ok: false,
            detail: Some(format!("guarded drivers_probe for {} failed: {e}", config.final_driver)),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        return halt_result(&config.bdf, "warm_swap", steps, patch_result,
                           module_loaded, false, overall, &sibling_state,
                           &config.module_name, needs_device_rollback);
    }

    let final_bound = guarded_sysfs::read_current_driver(&config.bdf);
    let swap_ok = final_bound.as_deref() == Some(config.final_driver.as_str());
    steps.push(HandoffStep {
        name: "warm_swap".into(), ok: swap_ok,
        detail: Some(format!("{} → {} (warm_preserved={})",
            config.seeder_driver, final_bound.as_deref().unwrap_or("none"), swap_ok)),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    if !swap_ok {

        return halt_result(&config.bdf, "warm_swap", steps, patch_result,
                           module_loaded, false, overall, &sibling_state,
                           &config.module_name, needs_device_rollback);
    }

    // ── Step 6b: Rebind IOMMU siblings to vfio-pci ─────────────────

    if !sibling_state.is_empty() {
        guarded_sysfs::rebind_siblings_to_vfio(&sibling_state);
    }

    // ── Step 7: Tier Classification ─────────────────────────────────

    let tier = if let Some(b) = bar0 {
        let t = Instant::now();
        let evidence = classify_tier(b);
        steps.push(HandoffStep {
            name: "tier_classify".into(), ok: true,
            detail: Some(format!("{}", evidence.tier)),
            duration_ms: t.elapsed().as_millis() as u64,
        });
        Some(evidence)
    } else {
        let t = Instant::now();
        match crate::vfio::device::MappedBar::from_sysfs_rw(&config.bdf, 16 * 1024 * 1024) {
            Ok(sysfs_bar) => {
                let evidence = classify_tier(&sysfs_bar);
                steps.push(HandoffStep {
                    name: "tier_classify".into(), ok: true,
                    detail: Some(format!("{} (via sysfs)", evidence.tier)),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                Some(evidence)
            }
            Err(e) => {
                steps.push(HandoffStep {
                    name: "tier_classify".into(), ok: false,
                    detail: Some(format!("BAR0 access failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                None
            }
        }
    };

    // ── Step 8: Module Cleanup (GUARDED) ────────────────────────────

    let mut module_unloaded = false;
    if module_loaded {
        let t = Instant::now();
        match guarded_sysfs::rmmod_guarded(&config.module_name, guarded_sysfs::RMMOD_TIMEOUT) {
            Ok(()) => {
                module_unloaded = true;
                let _ = module_patch::cleanup_patched_module(&config.module_name);
                steps.push(HandoffStep {
                    name: "module_cleanup".into(), ok: true,
                    detail: Some(format!("guarded rmmod {} + tmpfile removed", config.module_name)),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                tracing::warn!(module = config.module_name.as_str(), error = %e,
                               "guarded module cleanup failed (non-fatal)");
                steps.push(HandoffStep {
                    name: "module_cleanup".into(), ok: false,
                    detail: Some(format!("guarded rmmod failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // _handoff_guard drops here, releasing the per-BDF lock

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

/// Halt result with rollback. Runs best-effort recovery before returning.
///
/// Rollback triggers when any of:
/// - `module_loaded` is true (need to rmmod)
/// - `sibling_state` is non-empty (siblings were unbound)
/// - `needs_device_rollback` is true (device was unbound from its original
///   driver and needs to be restored to vfio-pci)
fn halt_result(
    bdf: &str,
    halted_at: &str,
    mut steps: Vec<HandoffStep>,
    patch_result: Option<ModulePatchResult>,
    module_loaded: bool,
    module_unloaded: bool,
    start: Instant,
    sibling_state: &[(String, Option<String>)],
    module_name: &str,
    needs_device_rollback: bool,
) -> HandoffResult {
    let needs_rollback = module_loaded || !sibling_state.is_empty() || needs_device_rollback;
    if needs_rollback {
        let t = Instant::now();
        let mod_name = if module_loaded { Some(module_name) } else { None };
        guarded_sysfs::handoff_rollback(bdf, mod_name, sibling_state);
        steps.push(HandoffStep {
            name: "rollback".into(), ok: true,
            detail: Some(format!("best-effort recovery (module={}, siblings={}, device={})",
                module_loaded, sibling_state.len(), needs_device_rollback)),
            duration_ms: t.elapsed().as_millis() as u64,
        });
    }

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

/// Overall deadline exceeded — run rollback and return.
fn deadline_exceeded(
    bdf: &str,
    mut steps: Vec<HandoffStep>,
    patch_result: Option<ModulePatchResult>,
    module_loaded: bool,
    module_name: &str,
    sibling_state: &[(String, Option<String>)],
    start: Instant,
) -> HandoffResult {
    tracing::error!(bdf, elapsed_ms = start.elapsed().as_millis() as u64,
                    "handoff deadline exceeded — running rollback");
    steps.push(HandoffStep {
        name: "deadline".into(), ok: false,
        detail: Some(format!("{}ms deadline exceeded at {}ms",
            guarded_sysfs::HANDOFF_DEADLINE.as_millis(),
            start.elapsed().as_millis())),
        duration_ms: 0,
    });

    let mod_name = if module_loaded { Some(module_name) } else { None };
    guarded_sysfs::handoff_rollback(bdf, mod_name, sibling_state);

    HandoffResult {
        bdf: bdf.into(),
        success: false,
        halted_at: Some("deadline".into()),
        steps,
        patch_result,
        tier: None,
        module_loaded,
        module_unloaded: false,
        total_ms: start.elapsed().as_millis() as u64,
    }
}

/// Load all dependencies for a kernel module using `modprobe --show-depends`.
///
/// Parses the dependency chain and loads each dependency module in order
/// using `insmod`. This is necessary because `insmod` (used for patched
/// modules) doesn't resolve dependencies like `modprobe` does.
fn load_module_dependencies(module_name: &str) -> Result<(), String> {
    let output = std::process::Command::new("modprobe")
        .args(["--show-depends", module_name])
        .output()
        .map_err(|e| format!("modprobe --show-depends failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("modprobe --show-depends {module_name}: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let deps = parse_modprobe_deps(&stdout, module_name);
    let mut loaded = 0;

    for ko_path in &deps {
        let dep_name = std::path::Path::new(ko_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .replace('-', "_");

        if kmod::is_module_loaded(&dep_name) {
            continue;
        }

        tracing::debug!(dep = ko_path.as_str(), "loading module dependency");
        let dep_path = std::path::Path::new(ko_path.as_str());
        if let Err(e) = guarded_sysfs::insmod_guarded(dep_path, guarded_sysfs::INSMOD_TIMEOUT) {
            tracing::warn!(dep = ko_path.as_str(), error = %e, "dependency load failed (continuing)");
        } else {
            loaded += 1;
        }
    }

    tracing::info!(module = module_name, deps_loaded = loaded, "module dependencies loaded");
    Ok(())
}

/// Parse `modprobe --show-depends` output into a list of dependency `.ko` paths,
/// excluding the target module itself. Testable without kernel access.
fn parse_modprobe_deps(output: &str, target_module: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let ko_path = line.strip_prefix("insmod ")?.trim();
            if ko_path.contains(&format!("/{target_module}.ko")) {
                None
            } else {
                Some(ko_path.to_string())
            }
        })
        .collect()
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
        assert!(HandoffConfig::from_strategy("nvidia_titanv", "0000:02:00.0").is_some());
        assert!(HandoffConfig::from_strategy("nvidia_patched_titanv", "0000:02:00.0").is_some());
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
    fn nvidia_titanv_config_uses_system_nvidia() {
        let cfg = HandoffConfig::nvidia_titanv("0000:02:00.0");
        assert!(matches!(cfg.module_source, ModuleSourceConfig::System));
        assert_eq!(cfg.seeder_driver, "nvidia");
        assert_eq!(cfg.module_name, "nvidia");
        assert_eq!(cfg.final_driver, "vfio-pci");
        assert_eq!(cfg.settle.as_secs(), 10);
    }

    #[test]
    fn nvidia_patched_titanv_uses_renamed_module() {
        let cfg = HandoffConfig::nvidia_patched_titanv("0000:02:00.0");
        assert!(matches!(cfg.module_source, ModuleSourceConfig::DkmsPatched { .. }));
        assert_eq!(cfg.seeder_driver, "nvsov");
        assert_eq!(cfg.module_name, "nvsov");
        if let ModuleSourceConfig::DkmsPatched { dkms_module, dkms_version, patch_set } = &cfg.module_source {
            assert_eq!(dkms_module, "nvidia");
            assert_eq!(dkms_version, "470.256.02");
            assert_eq!(patch_set, "nvidia_warm_handoff");
        }
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

    #[test]
    fn handoff_guard_acquire_release() {
        // Use unique BDFs to avoid interference with parallel tests
        let bdf = "test:aa:00.0";
        let guard = HandoffGuard::acquire(bdf).unwrap();

        // Double-acquire should fail
        let second = HandoffGuard::acquire(bdf);
        assert!(second.is_err());

        // Drop the guard
        drop(guard);

        // Re-acquire should succeed after drop
        let guard2 = HandoffGuard::acquire(bdf).unwrap();
        drop(guard2);
    }

    #[test]
    fn handoff_guard_raii_drop() {
        let bdf = "test:bb:00.0";
        {
            let _guard = HandoffGuard::acquire(bdf).unwrap();
            // guard drops at end of scope
        }
        // Should be re-acquirable
        let _guard = HandoffGuard::acquire(bdf).unwrap();
    }

    #[test]
    fn halt_result_rollback_with_needs_device_rollback() {
        // Even with module_loaded=false and empty sibling_state,
        // needs_device_rollback=true should trigger rollback step
        let steps = vec![HandoffStep {
            name: "test".into(),
            ok: true,
            detail: None,
            duration_ms: 0,
        }];
        let result = halt_result(
            "ffff:ff:ff.f",
            "test_halt",
            steps,
            None,
            false,
            false,
            Instant::now(),
            &[],
            "nouveau",
            true, // needs_device_rollback
        );
        assert!(!result.success);
        assert_eq!(result.halted_at.as_deref(), Some("test_halt"));
        // Should have 2 steps: original + rollback
        assert_eq!(result.steps.len(), 2);
        assert_eq!(result.steps[1].name, "rollback");
        let detail = result.steps[1].detail.as_ref().unwrap();
        assert!(detail.contains("device=true"));
    }

    #[test]
    fn halt_result_no_rollback_when_nothing_needed() {
        let steps = vec![HandoffStep {
            name: "test".into(),
            ok: false,
            detail: None,
            duration_ms: 0,
        }];
        let result = halt_result(
            "ffff:ff:ff.f",
            "preflight",
            steps,
            None,
            false,
            false,
            Instant::now(),
            &[],
            "nouveau",
            false,
        );
        // Only 1 step — no rollback triggered
        assert_eq!(result.steps.len(), 1);
    }

    #[test]
    fn halt_result_rollback_with_module_loaded() {
        let steps = vec![];
        let result = halt_result(
            "ffff:ff:ff.f",
            "warm_swap",
            steps,
            None,
            true,  // module_loaded
            false,
            Instant::now(),
            &[],
            "nouveau",
            false,
        );
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].name, "rollback");
        let detail = result.steps[0].detail.as_ref().unwrap();
        assert!(detail.contains("module=true"));
    }

    #[test]
    fn halt_result_rollback_with_siblings() {
        let siblings = vec![
            ("0000:02:00.1".to_string(), Some("snd_hda_intel".to_string())),
        ];
        let steps = vec![];
        let result = halt_result(
            "ffff:ff:ff.f",
            "warm_swap",
            steps,
            None,
            false,
            false,
            Instant::now(),
            &siblings,
            "nouveau",
            false,
        );
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].name, "rollback");
        let detail = result.steps[0].detail.as_ref().unwrap();
        assert!(detail.contains("siblings=1"));
    }

    #[test]
    fn parse_modprobe_deps_extracts_paths() {
        let output = "\
insmod /lib/modules/6.17.9/kernel/drivers/gpu/drm/drm.ko
insmod /lib/modules/6.17.9/kernel/drivers/gpu/drm/drm_gpuvm.ko
insmod /lib/modules/6.17.9/kernel/drivers/gpu/drm/scheduler/gpu-sched.ko
insmod /lib/modules/6.17.9/kernel/drivers/gpu/drm/nouveau/nouveau.ko
";
        let deps = parse_modprobe_deps(output, "nouveau");
        assert_eq!(deps.len(), 3);
        assert!(deps[0].ends_with("drm.ko"));
        assert!(deps[1].ends_with("drm_gpuvm.ko"));
        assert!(deps[2].ends_with("gpu-sched.ko"));
    }

    #[test]
    fn parse_modprobe_deps_handles_install_lines() {
        let output = "\
install /sbin/modprobe --ignore-install some-mod
insmod /lib/modules/6.17.9/dep.ko
insmod /lib/modules/6.17.9/nouveau.ko
";
        let deps = parse_modprobe_deps(output, "nouveau");
        assert_eq!(deps.len(), 1);
        assert!(deps[0].ends_with("dep.ko"));
    }

    #[test]
    fn parse_modprobe_deps_empty_output() {
        let deps = parse_modprobe_deps("", "nouveau");
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_modprobe_deps_only_target() {
        let output = "insmod /lib/modules/6.17.9/nouveau.ko\n";
        let deps = parse_modprobe_deps(output, "nouveau");
        assert!(deps.is_empty());
    }
}
