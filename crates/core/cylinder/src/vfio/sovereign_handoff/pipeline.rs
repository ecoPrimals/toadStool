// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::time::{Duration, Instant};

use std::fmt::Write;

use crate::nv::registers::{falcon, gpc, pmc, pri};
use crate::vfio::guarded_sysfs;
use crate::vfio::kernel_health;
use crate::vfio::kmod;
use crate::vfio::module_patch::{self, PatchSet};
use crate::vfio::sovereign_tiers::{TierEvidence, classify_tier};
use crate::vfio::warm_capture::Bar0Snapshot;
use toadstool_ember::pri_ring_anchor::{BootServiceEvidence, PriRingAnchor, PriRingHealth};

use super::lock::HandoffGuard;
use super::module_deps::load_module_dependencies;
use super::pri_recovery::recover_pri_ring;
use super::rollback::{deadline_exceeded, halt_result, halt_result_poisoned};
use super::rm_trigger::trigger_rm_init;
use super::types::{HandoffConfig, HandoffResult, HandoffStep, ModuleSourceConfig};

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
    execute_handoff_inner(config, bar0, None)
}

/// Execute a sovereign handoff with an optional heartbeat callback.
///
/// `heartbeat_fn` is called at each major pipeline step boundary to reset
/// external watchdog timers (diesel engine safety net). The callback is
/// invoked from the blocking handoff thread context.
pub fn execute_handoff_with_heartbeat(
    config: &HandoffConfig,
    bar0: Option<&crate::vfio::device::MappedBar>,
    heartbeat_fn: impl Fn() + Send + 'static,
) -> HandoffResult {
    execute_handoff_inner(config, bar0, Some(Box::new(heartbeat_fn)))
}

#[allow(clippy::too_many_lines, reason = "sovereign handoff is a linear hardware init pipeline — splitting would obscure the sequencing")]
fn execute_handoff_inner(
    config: &HandoffConfig,
    bar0: Option<&crate::vfio::device::MappedBar>,
    heartbeat_fn: Option<Box<dyn Fn() + Send>>,
) -> HandoffResult {
    let overall = Instant::now();
    let deadline = guarded_sysfs::HANDOFF_DEADLINE;
    let hw = super::types::HandoffCapabilityProfile::for_sm(config.sm_version.unwrap_or(70));
    let heartbeat = || { if let Some(ref f) = heartbeat_fn { f(); } };
    let mut steps = Vec::new();
    let mut module_loaded = false;
    let mut patch_result = None;
    let mut sibling_state: Vec<(String, Option<String>)> = Vec::new();
    let mut catalyst_snapshot_path: Option<String> = None;
    let mut catalyst_alive_count: Option<usize> = None;
    let mut catalyst_tier: Option<TierEvidence> = None;
    let mut boot_evidence: Option<BootServiceEvidence> = None;

    heartbeat();
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

    if config.skip_preflight {
        tracing::warn!("skip_preflight=true — skipping module stuck, IOMMU, and kernel health checks");
    } else {
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
    }

    steps.push(HandoffStep {
        name: "preflight".into(), ok: true,
        detail: Some(if config.skip_preflight {
            "preflight skipped (skip_preflight=true)".into()
        } else {
            "module clean, IOMMU group free, no concurrent handoff, kernel healthy".into()
        }),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    // Detect catalyst strategies early — used for anchor-release guard
    // and later for pre-swap capture + diagnostics.
    let is_catalyst = matches!(
        &config.module_source,
        ModuleSourceConfig::DkmsPatched { patch_set, .. }
            if patch_set == "nvidia_catalyst_handoff"
                || patch_set == "nvidia_boot_services"
                || patch_set == "nvidia_warm_handoff"
    );

    // ── Step 0e: Verify GPU survived anchor release (Exp 225 guard) ──
    //
    // The RPC handler drops the VfioAnchor before calling execute_handoff.
    // If FLR was not suppressed, vfio_pci_core_release() resets the GPU
    // and PMC_ENABLE drops from ~23 engines to ~2. Detect this early
    // rather than wasting 60s on a doomed catalyst settle.
    //
    // Exp 229: On cold-start (post-reboot), the GPU was never warm — the
    // catalyst pipeline's purpose is to warm it via nvidia driver load.
    // Only halt if the GPU was warm and regressed; cold-start is expected.
    if is_catalyst {
        let t = Instant::now();
        let pmc_check = crate::vfio::device::MappedBar::from_sysfs_rw(
            &config.bdf, 4096,
        ).map(|bar| {
            let pmc = bar.read_u32(pmc::ENABLE as usize).unwrap_or(0);
            let popcount = pmc.count_ones();
            (pmc, popcount)
        });
        match pmc_check {
            Ok((pmc, popcount)) => {
                tracing::info!(
                    bdf = config.bdf.as_str(),
                    pmc = format_args!("0x{pmc:08x}"),
                    popcount,
                    "anchor release guard: PMC_ENABLE health check"
                );
                if popcount < hw.pmc_warm_threshold {
                    tracing::warn!(
                        bdf = config.bdf.as_str(),
                        pmc = format_args!("0x{pmc:08x}"),
                        popcount,
                        "GPU cold at anchor release — catalyst will warm it"
                    );
                }
                steps.push(HandoffStep {
                    name: "anchor_release_guard".into(), ok: true,
                    detail: Some(format!(
                        "PMC_ENABLE=0x{pmc:08x} (popcount={popcount}){}",
                        if popcount < hw.pmc_warm_threshold { " — cold start, catalyst will warm" } else { " — GPU warm" }
                    )),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                tracing::warn!(
                    bdf = config.bdf.as_str(),
                    error = %e,
                    "anchor release guard: cannot read PMC_ENABLE — proceeding"
                );
            }
        }
    }

    heartbeat();
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

            let ps = if let Some(ref json) = config.patch_set_override {
                match PatchSet::from_json(json) {
                    Ok(ps) => ps,
                    Err(e) => {
                        steps.push(HandoffStep {
                            name: "module_prep".into(), ok: false,
                            detail: Some(format!("invalid patch_set_override JSON: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                    }
                }
            } else {
                match PatchSet::by_name(patch_set) {
                    Some(ps) => ps,
                    None => {
                        steps.push(HandoffStep {
                            name: "module_prep".into(), ok: false,
                            detail: Some(format!("unknown patch set: {patch_set}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                    }
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

            let ps = if let Some(ref json) = config.patch_set_override {
                match PatchSet::from_json(json) {
                    Ok(ps) => ps,
                    Err(e) => {
                        steps.push(HandoffStep {
                            name: "module_prep".into(), ok: false,
                            detail: Some(format!("invalid patch_set_override JSON: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                    }
                }
            } else {
                match PatchSet::by_name(patch_set) {
                    Some(ps) => ps,
                    None => {
                        steps.push(HandoffStep {
                            name: "module_prep".into(), ok: false,
                            detail: Some(format!("unknown patch set: {patch_set}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                    }
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

            // For dual-load (renamed) modules, run objcopy BEFORE patching.
            // This strips __ksymtab export sections that cause "duplicate
            // symbol" errors, and ensures that all subsequent ELF
            // manipulation (normalization, NOPs, relocation nullification)
            // operates on the final ELF layout.
            let patch_source = if rename_pair.is_some() {
                let staging = PathBuf::from(format!(
                    "/tmp/toadstool-staging-{}.ko", config.module_name
                ));
                if let Err(e) = std::fs::copy(&stock_path, &staging) {
                    steps.push(HandoffStep {
                        name: "module_prep".into(), ok: false,
                        detail: Some(format!("failed to copy DKMS module to staging: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return halt_result(&config.bdf, "module_prep", steps, None, false, false, overall, &[], &config.module_name, false);
                }
                match module_patch::strip_ksymtab_sections(&staging, &staging) {
                    Ok(()) => {
                        tracing::info!(
                            path = %staging.display(),
                            "pre-patch: stripped ksymtab export sections (pure Rust)"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "ksymtab strip failed (continuing)"
                        );
                    }
                }
                staging
            } else {
                stock_path.clone()
            };

            // For dual-load (renamed) modules, the nvidia proprietary
            // driver probes PCI during init. The target GPU must be unbound
            // from vfio-pci BEFORE insmod so the driver can find it.
            // We defer insmod to after the unbind step below.
            let deferred_insmod = rename_pair.is_some();

            match module_patch::patch_module_with_rename(&patch_source, &ps, rename_pair) {
                Ok(pr) => {
                    // Clean up staging file
                    if rename_pair.is_some() {
                        let staging = PathBuf::from(format!(
                            "/tmp/toadstool-staging-{}.ko", config.module_name
                        ));
                        let _ = std::fs::remove_file(&staging);
                    }

                    if !deferred_insmod {
                        let patched_path = PathBuf::from(&pr.patched_path);
                        if let Err(e) = guarded_sysfs::insmod_guarded(
                            &patched_path, guarded_sysfs::INSMOD_TIMEOUT,
                        ) {
                            steps.push(HandoffStep {
                                name: "module_prep".into(), ok: false,
                                detail: Some(format!("guarded insmod DKMS module failed: {e}")),
                                duration_ms: t.elapsed().as_millis() as u64,
                            });
                            patch_result = Some(pr);
                            return halt_result(&config.bdf, "module_prep", steps, patch_result, false, false, overall, &[], &config.module_name, false);
                        }
                        module_loaded = true;
                    }
                    patch_result = Some(pr);
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
                .map(|pr| format!("DKMS patched module {} ({}/{} patches, {})",
                    if deferred_insmod { "prepared" } else { "loaded" },
                    pr.applied_count, pr.total_count,
                    rename_pair.map(|(o, n)| format!("renamed {o}→{n}")).unwrap_or_else(|| "no rename".into())))
                .unwrap_or_else(|| "DKMS patched module prepared".into());
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

    heartbeat();
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
        let _ = write!(detail_msg, "; siblings: [{}]", sibling_summary.join(", "));
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
    let needs_device_rollback = true;

    // ── Deferred insmod for dual-load ───────────────────────────────
    // The GPU is now unbound from vfio-pci. Set driver_override to our
    // renamed module name so the kernel binds this device to our module
    // (not the host nvidia) when we insmod.
    if let ModuleSourceConfig::DkmsPatched { .. } = &config.module_source
        && !module_loaded
        && let Some(ref pr) = patch_result
    {
                // Disable the PCI device so the kernel releases its
                // BAR resource claims. Without this, nvidia's direct
                // request_mem_region call on BAR0 fails because the
                // PCI subsystem still has the region reserved from the
                // previous driver's pci_enable_device.
                // Direct sysfs_write is safe here: the device is unbound
                // and the `enable` attribute is a non-blocking kernel op.
                let enable_path = crate::linux_paths::sysfs_pci_device_file(
                    &config.bdf, "enable",
                );
                match guarded_sysfs::sysfs_write(&enable_path, "0") {
                    Ok(()) => tracing::info!(bdf = config.bdf.as_str(),
                        "pci device disabled — BAR resources released for driver takeover"),
                    Err(e) => tracing::warn!(bdf = config.bdf.as_str(), error = %e,
                        "pci disable failed (continuing — request_mem_region may fail)"),
                }

                let override_path = crate::linux_paths::sysfs_pci_device_file(
                    &config.bdf, "driver_override",
                );
                if let Err(e) = guarded_sysfs::sysfs_write_guarded(
                    &override_path, &config.module_name,
                    guarded_sysfs::UNBIND_TIMEOUT,
                ) {
                    tracing::warn!(error = %e, "driver_override write failed (continuing)");
                }

                let patched_path = PathBuf::from(&pr.patched_path);
                let t = Instant::now();
                match guarded_sysfs::insmod_guarded(&patched_path, guarded_sysfs::INSMOD_TIMEOUT) {
                    Ok(()) => {
                        module_loaded = true;
                        // Trigger re-probe so the device binds to our module
                        let probe_path = crate::linux_paths::sysfs_pci_driver_bind(
                            &config.module_name,
                        );
                        let _ = guarded_sysfs::sysfs_write_guarded(
                            &probe_path, &config.bdf,
                            guarded_sysfs::PROBE_TIMEOUT,
                        );
                        steps.push(HandoffStep {
                            name: "deferred_insmod".into(), ok: true,
                            detail: Some("dual-load module loaded + bound via driver_override".to_string()),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                    }
                    Err(e) => {
                        let poisoned = matches!(
                            e, guarded_sysfs::GuardedSysfsError::KmodTimeout { .. }
                        );

                        if poisoned {
                            tracing::error!(bdf = config.bdf.as_str(),
                                "insmod TIMED OUT — device likely D-state poisoned. \
                                 Skipping all sysfs ops to protect ember.");
                        } else {
                            // Safe to touch sysfs — insmod failed fast (e.g. ENODEV, EBUSY)
                            let _ = guarded_sysfs::sysfs_write_guarded(
                                &override_path, "",
                                guarded_sysfs::UNBIND_TIMEOUT,
                            );
                        }

                        steps.push(HandoffStep {
                            name: "deferred_insmod".into(), ok: false,
                            detail: Some(format!("deferred insmod failed: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });

                        if poisoned {
                            return halt_result_poisoned(
                                &config.bdf, "deferred_insmod", steps, patch_result,
                                false, false, overall, &sibling_state,
                                &config.module_name, true);
                        }
                        return halt_result(&config.bdf, "deferred_insmod", steps, patch_result,
                                           false, false, overall, &sibling_state,
                                           &config.module_name, true);
                    }
                }
    }

    heartbeat();
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

    heartbeat();
    // ── Step 3b: Catalyst RM trigger — open chardev to start GPU init ──
    //
    // nvidia RM defers GPU initialization to userspace open. With the
    // catalyst PatchByteAt(0x7b) patch, __register_chrdev uses major 0
    // (dynamic allocation). We find the assigned major, create a device
    // node, and open it to trigger rm_init_adapter → full GPU init
    // (SEC2 → ACR → FECS → GPCCS → TPC PRI station creation).
    // The chardev name is "nvidia-frontend" (from .rodata), not the renamed
    // module name. trigger_rm_init searches for "nvidia-frontend" entries.
    let mut rm_channel_evidence = None;
    if is_catalyst && module_loaded {
        let t = Instant::now();
        // Exp 229: pass create_channel=true for catalyst strategies to
        // establish a full RM compute channel before warm swap.
        match trigger_rm_init(&config.module_name, /* create_channel */ true, &config.bdf, &hw.interrupt_profile) {
            Ok(result) => {
                tracing::info!(bdf = config.bdf.as_str(), summary = result.summary.as_str(),
                    channel_evidence = ?result.channel_evidence,
                    "catalyst RM init triggered");
                rm_channel_evidence = result.channel_evidence;
                steps.push(HandoffStep {
                    name: "rm_trigger".into(), ok: true,
                    detail: Some(result.summary),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                tracing::warn!(bdf = config.bdf.as_str(), error = %e,
                    "catalyst RM trigger failed — RM may not initialize GPU");
                steps.push(HandoffStep {
                    name: "rm_trigger".into(), ok: false,
                    detail: Some(format!("RM trigger failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    heartbeat();
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

    // ── Post-settle GPU health check (catalyst only) ─────────────
    //
    // After the settle period, verify the seeder driver (RM) actually
    // completed DEVINIT. If PMC_ENABLE is still cold (popcount < 10),
    // the driver failed to initialize — log a clear diagnostic but
    // continue to capture whatever state exists for forensics.
    if is_catalyst {
        let t = Instant::now();
        // Map full 16MB BAR0 — FECS is at 0x409xxx, TPC at 0x504xxx.
        match crate::vfio::device::MappedBar::from_sysfs_rw(&config.bdf, 16 * 1024 * 1024) {
            Ok(bar0) => {
                let pmc = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
                let popcount = pmc.count_ones();
                if popcount < hw.pmc_warm_threshold {
                    tracing::error!(
                        bdf = config.bdf.as_str(),
                        pmc = format_args!("0x{pmc:08x}"),
                        popcount,
                        "catalyst settle: RM did NOT complete DEVINIT — GPU still cold"
                    );
                    steps.push(HandoffStep {
                        name: "settle_health".into(), ok: false,
                        detail: Some(format!(
                            "RM failed DEVINIT: PMC_ENABLE=0x{pmc:08x} (popcount={popcount})"
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                } else {
                    tracing::info!(
                        bdf = config.bdf.as_str(),
                        pmc = format_args!("0x{pmc:08x}"),
                        popcount,
                        "catalyst settle: RM DEVINIT healthy"
                    );
                    steps.push(HandoffStep {
                        name: "settle_health".into(), ok: true,
                        detail: Some(format!(
                            "PMC_ENABLE=0x{pmc:08x} (popcount={popcount}) — RM initialized"
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                drop(bar0);
            }
            Err(e) => {
                tracing::warn!(
                    bdf = config.bdf.as_str(),
                    error = %e,
                    "settle health: cannot open BAR0 — RM may be holding resource0"
                );
            }
        }
    }

    // ── Deadline check ──────────────────────────────────────────────

    if overall.elapsed() >= deadline {

        return deadline_exceeded(&config.bdf, steps, patch_result, module_loaded,
                                 &config.module_name, &sibling_state, overall);
    }

    heartbeat();
    // ── Step 4b: Catalyst Capture (if catalyst strategy) ──────────
    //
    // While the catalyst driver owns the GPU and has fully initialized
    // the compute pipeline, capture BAR0 state for preservation.
    // This is the "golden snapshot" — the catalyst's product.
    // (is_catalyst already set at Step 0e)

    if is_catalyst {
        let t = Instant::now();
        let bar0_size = 16 * 1024 * 1024; // 16 MiB
        match crate::vfio::device::MappedBar::from_sysfs_rw(&config.bdf, bar0_size) {
            Ok(catalyst_bar0) => {
                // Quick targeted reads: tier classification + sovereign snapshot.
                // These read ~20 specific registers and complete in microseconds.
                // The full 16MB capture is deferred to after warm swap (back on
                // vfio-pci), because bulk MMIO reads while the nvidia RM is
                // active can hit PRI fault regions and hang the thread.
                let sovereign_snap = crate::vfio::sovereign_stages::SovereignSnapshot::capture(&catalyst_bar0);
                let tier_ev = classify_tier(&catalyst_bar0);

                tracing::info!(
                    bdf = config.bdf.as_str(),
                    tier = ?tier_ev.tier,
                    pmc_enable = format_args!("{:#010x}", tier_ev.pmc_enable),
                    tpc_alive = tier_ev.tpc_alive,
                    "catalyst capture: tier evidence while catalyst owns GPU"
                );

                tracing::info!(
                    pmc_enable = format_args!("{:#010x}", sovereign_snap.pmc_enable),
                    fecs_cpuctl = format_args!("{:#010x}", sovereign_snap.fecs_cpuctl),
                    fecs_pc = format_args!("{:#010x}", sovereign_snap.fecs_pc),
                    gpccs_cpuctl = format_args!("{:#010x}", sovereign_snap.gpccs_cpuctl),
                    pmu_cpuctl = format_args!("{:#010x}", sovereign_snap.pmu_cpuctl),
                    pgraph_status = format_args!("{:#010x}", sovereign_snap.pgraph_status),
                    "catalyst capture: sovereign snapshot registers (pre-swap)"
                );

                catalyst_tier = Some(tier_ev);

                // ── ExitBootServices: capture firmware evidence ──
                let mut evidence = BootServiceEvidence::new(
                    "gpu-falcon",
                    "FECS/GPCCS/PMU state captured pre-swap (ExitBootServices)",
                );
                evidence.record("bdf", &config.bdf);
                evidence.record("fecs_cpuctl", format!("{:#010x}", sovereign_snap.fecs_cpuctl));
                evidence.record("fecs_pc", format!("{:#010x}", sovereign_snap.fecs_pc));
                evidence.record("gpccs_cpuctl", format!("{:#010x}", sovereign_snap.gpccs_cpuctl));
                evidence.record("pmu_cpuctl", format!("{:#010x}", sovereign_snap.pmu_cpuctl));
                evidence.record("pmc_enable", format!("{:#010x}", sovereign_snap.pmc_enable));
                evidence.record("pgraph_status", format!("{:#010x}", sovereign_snap.pgraph_status));
                // Probe TPC status across GPCs (generation-aware topology)
                for gpc in 0..hw.gpc_count {
                    let addr = hw.tpc_base as usize + gpc as usize * hw.tpc_gpc_stride as usize;
                    if let Ok(tpc_val) = catalyst_bar0.read_u32(addr) {
                        let is_fault = tpc_val & 0xBADF_0000 == 0xBADF_0000;
                        evidence.record(
                            format!("gpc{gpc}_tpc0"),
                            format!("{:#010x}{}", tpc_val, if is_fault { " FAULT" } else { " ALIVE" }),
                        );
                    }
                }

                // ── FECS IMEM capture attempt (while nvidia still loaded) ──
                // Falcon PIO: set IMEMC once with auto-increment read
                // (bit 25), then read IMEMD sequentially.
                let fecs_base = hw.fecs_base as usize;
                let imemc = fecs_base + 0x180;
                let imemd = fecs_base + 0x184;
                // Set IMEMC: start at address 0, auto-increment read (bit 25)
                let _ = catalyst_bar0.write_u32(imemc, 0x0200_0000);
                std::thread::sleep(std::time::Duration::from_micros(100));
                let mut imem_probe = Vec::with_capacity(16);
                for _ in 0..16 {
                    let word = catalyst_bar0.read_u32(imemd).unwrap_or(0xDEAD_DEAD);
                    imem_probe.push(word);
                }
                let imem_nonzero = imem_probe.iter().filter(|&&w| w != 0 && w != 0xDEAD_DEAD).count();
                let imem_faulted = imem_probe.iter().filter(|&&w| w & 0xBADF_0000 == 0xBADF_0000).count();
                evidence.record("fecs_imem_probe_nonzero", imem_nonzero.to_string());
                evidence.record("fecs_imem_probe_faulted", imem_faulted.to_string());
                evidence.record("fecs_imem_word0", format!("{:#010x}", imem_probe[0]));
                evidence.record("fecs_imem_word1", format!("{:#010x}", imem_probe[1]));
                tracing::info!(
                    bdf = config.bdf.as_str(),
                    nonzero = imem_nonzero,
                    faulted = imem_faulted,
                    word0 = format_args!("{:#010x}", imem_probe[0]),
                    word1 = format_args!("{:#010x}", imem_probe[1]),
                    word2 = format_args!("{:#010x}", imem_probe[2]),
                    word3 = format_args!("{:#010x}", imem_probe[3]),
                    "FECS IMEM probe (PIO read) while nvidia loaded"
                );

                // If IMEM is readable, do full FECS + GPCCS capture
                if imem_nonzero > 0 && imem_faulted == 0 {
                    let fw_dir = "/var/lib/toadstool/catalysts/firmware";
                    let _ = std::fs::create_dir_all(fw_dir);
                    for (name, base) in [("fecs", 0x40_9000usize), ("gpccs", 0x41_a000usize)] {
                        let imemc_r = base + 0x180;
                        let imemd_r = base + 0x184;
                        let imem_size = 32 * 1024usize;
                        let word_count = imem_size / 4;
                        let _ = catalyst_bar0.write_u32(imemc_r, 0x0200_0000);
                        std::thread::sleep(std::time::Duration::from_micros(100));
                        let mut fw_data = Vec::with_capacity(word_count);
                        for _ in 0..word_count {
                            fw_data.push(catalyst_bar0.read_u32(imemd_r).unwrap_or(0));
                        }
                        let fw_bytes: Vec<u8> = fw_data.iter()
                            .flat_map(|w| w.to_le_bytes()).collect();
                        let nonzero_bytes = fw_bytes.iter().filter(|&&b| b != 0).count();
                        let fw_path = format!("{fw_dir}/{name}_imem_{}.bin", hw.chip_name);
                        if std::fs::write(&fw_path, &fw_bytes).is_ok() {
                            evidence.record(
                                format!("{name}_imem_captured"),
                                format!("{} bytes, {} nonzero", fw_bytes.len(), nonzero_bytes),
                            );
                            tracing::info!(
                                engine = name, path = fw_path.as_str(),
                                size = fw_bytes.len(), nonzero = nonzero_bytes,
                                "{name} IMEM firmware captured to disk"
                            );
                        }
                    }
                }

                // ── PCCSR scan: verify RM channel is ACTIVE (Exp 229) ──
                if let Some(ref ev) = rm_channel_evidence
                    && ev.all_ok
                {
                        let pccsr_base = hw.pccsr_base as usize;
                        let mut active_channels = Vec::new();
                        let mut pending_channels = Vec::new();
                        for ch in 0..hw.pccsr_channel_count {
                            let addr = pccsr_base + ch as usize * 8;
                            if let Ok(val) = catalyst_bar0.read_u32(addr) {
                                let enabled = val & 1;
                                let status = (val >> 24) & 0x1F;
                                if enabled != 0 {
                                    if status >= 5 {
                                        active_channels.push((ch, val));
                                    } else {
                                        pending_channels.push((ch, val));
                                    }
                                }
                            }
                        }
                        for &(ch, val) in &active_channels {
                            evidence.record(
                                format!("pccsr_ch{ch}_active"),
                                format!("{:#010x} (status={})", val, (val >> 24) & 0x1F),
                            );
                        }
                        for &(ch, val) in &pending_channels {
                            evidence.record(
                                format!("pccsr_ch{ch}_pending"),
                                format!("{:#010x} (status={})", val, (val >> 24) & 0x1F),
                            );
                        }
                        tracing::info!(
                            bdf = config.bdf.as_str(),
                            active = active_channels.len(),
                            pending = pending_channels.len(),
                            rm_channel_id = ?ev.channel_id,
                            "PCCSR channel scan while catalyst loaded (Exp 229)"
                        );
                }

                boot_evidence = Some(evidence);
                tracing::info!(
                    bdf = config.bdf.as_str(),
                    preserved_keys = boot_evidence.as_ref().map(|e| e.preserved_state.len()).unwrap_or(0),
                    "ExitBootServices: firmware evidence captured"
                );

                // Drop the BAR0 mapping before warm swap to release the fd
                drop(catalyst_bar0);

                steps.push(HandoffStep {
                    name: "catalyst_capture".into(), ok: true,
                    detail: Some(format!(
                        "pre-swap tier={:?} (full capture deferred to post-swap)",
                        catalyst_tier.as_ref().map(|t| &t.tier),
                    )),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                tracing::warn!(
                    bdf = config.bdf.as_str(),
                    err = %e,
                    "catalyst capture: failed to open BAR0 — skipping capture"
                );
                steps.push(HandoffStep {
                    name: "catalyst_capture".into(), ok: false,
                    detail: Some(format!("BAR0 open failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    heartbeat();
    // ── Step 5: Pin bridges + disable FLR + suppress SBR ───────────
    //
    // The RPC handler calls prepare_anchor_release() before dropping the
    // VfioAnchor. For cold GPUs (Exp 229), SBR was intentionally allowed
    // during anchor release so RM could cold-boot with a clean PCIe reset.
    // Now that RM init is complete (seeder_settle finished), we MUST
    // suppress SBR before the warm swap to preserve the RM-initialized
    // state. For warm GPUs, SBR was already suppressed — this is idempotent.

    let t = Instant::now();
    guarded_sysfs::pin_bridge_hierarchy(&config.bdf);
    guarded_sysfs::disable_flr(&config.bdf);
    if let Err(e) = guarded_sysfs::suppress_bus_reset(&config.bdf) {
        tracing::warn!(
            bdf = config.bdf.as_str(),
            error = %e,
            "failed to suppress SBR before warm swap — state may be lost"
        );
    }
    steps.push(HandoffStep {
        name: "prepare_warm_swap".into(), ok: true,
        detail: Some("bridge pinned, FLR disabled, SBR suppressed".into()),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    heartbeat();
    // ── Step 6: Warm swap — seeder → final driver (GUARDED) ─────────

    let t = Instant::now();
    if let Some(ref current) = guarded_sysfs::read_current_driver(&config.bdf) {
        let remaining = deadline.saturating_sub(overall.elapsed());
        let unbind_result = if is_catalyst {
            // nvidia RM teardown takes 160-400s on GV100. Fire-and-poll
            // avoids blocking ember's thread — we just poll the driver
            // symlink every 2s until it clears.
            guarded_sysfs::sysfs_unbind_fire_and_poll(
                &config.bdf, current, remaining,
            )
            .map(|elapsed| {
                tracing::info!(
                    bdf = config.bdf.as_str(),
                    elapsed_s = elapsed.as_secs(),
                    "catalyst teardown completed via fire-and-poll"
                );
            })
        } else {
            let unbind_path = crate::linux_paths::sysfs_pci_driver_unbind(current);
            guarded_sysfs::sysfs_write_guarded(
                &unbind_path, &config.bdf, guarded_sysfs::UNBIND_TIMEOUT,
            )
        };
        if let Err(e) = unbind_result {
            steps.push(HandoffStep {
                name: "warm_swap".into(), ok: false,
                detail: Some(format!("unbind {current} failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });

            return halt_result(&config.bdf, "warm_swap", steps, patch_result,
                               module_loaded, false, overall, &sibling_state,
                               &config.module_name, needs_device_rollback);
        }
    }

    if is_catalyst {
        // Diagnostic: probe PCI config space and BAR0 between unbind and rebind.
        // Read PCI command register to check if bus mastering was disabled.
        let pci_config_path = crate::linux_paths::sysfs_pci_device_file(
            &config.bdf, "config",
        );
        let pci_cmd = std::fs::read(&pci_config_path)
            .ok()
            .and_then(|data| {
                if data.len() >= 6 {
                    Some(u16::from_le_bytes([data[4], data[5]]))
                } else {
                    None
                }
            });
        let pci_pm_ctrl = std::fs::read(&pci_config_path)
            .ok()
            .and_then(|data| {
                // PM cap at offset 0x60, PMCSR at +0x04 = 0x64
                if data.len() >= 0x66 {
                    Some(u16::from_le_bytes([data[0x64], data[0x65]]))
                } else {
                    None
                }
            });
        tracing::info!(
            bdf = config.bdf.as_str(),
            pci_cmd = pci_cmd.map(|v| format!("{:#06x}", v)),
            bus_master = pci_cmd.map(|v| v & 0x4 != 0),
            mem_space = pci_cmd.map(|v| v & 0x2 != 0),
            pm_state = pci_pm_ctrl.map(|v| format!("D{}", v & 0x3)),
            "post-unbind PCI config: command register and power state"
        );

        if let Ok(diag_bar0) = crate::vfio::device::MappedBar::from_sysfs_rw(
            &config.bdf, 16 * 1024 * 1024,
        ) {
            let fecs_cpuctl = diag_bar0
                .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                .unwrap_or(0xDEAD);
            let fecs_hw_pc = diag_bar0
                .read_u32((falcon::FECS_BASE + 0x11C) as usize)
                .unwrap_or(0xDEAD);
            let fecs_ctxsw = diag_bar0
                .read_u32(falcon::FECS_CTXSW_PC as usize)
                .unwrap_or(0xDEAD);
            let gpccs_cpuctl = diag_bar0
                .read_u32((falcon::GPCCS_BASE + falcon::CPUCTL) as usize)
                .unwrap_or(0xDEAD);
            let pri_intr = diag_bar0.read_u32(pri::INTR_STATUS as usize).unwrap_or(0xDEAD);
            let pri_status = diag_bar0.read_u32(pri::STATUS_ENUM as usize).unwrap_or(0xDEAD);
            let pmc_enable = diag_bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0xDEAD);
            let tpc0 = diag_bar0
                .read_u32(gpc::gpc_tpc0(0) as usize)
                .unwrap_or(0xDEAD);
            tracing::info!(
                bdf = config.bdf.as_str(),
                fecs_cpuctl = format_args!("{:#010x}", fecs_cpuctl),
                fecs_hw_pc = format_args!("{:#010x}", fecs_hw_pc),
                fecs_ctxsw = format_args!("{:#010x}", fecs_ctxsw),
                gpccs_cpuctl = format_args!("{:#010x}", gpccs_cpuctl),
                pmc_enable = format_args!("{:#010x}", pmc_enable),
                pri_ring_intr = format_args!("{:#010x}", pri_intr),
                pri_ring_status = format_args!("{:#010x}", pri_status),
                tpc0_ctrl = format_args!("{:#010x}", tpc0),
                "catalyst diagnostic: BAR0 state AFTER unbind, BEFORE rebind"
            );
            drop(diag_bar0);
        }

        // After fire-and-poll, the driver symlink clears in ~2s but nvidia
        // RM teardown still holds the PCI lock for 160-400s. We need to:
        //   1. Wait for driver=None (done by fire-and-poll)
        //   2. Set driver_override to final_driver via guarded write (may
        //      block until PCI lock releases — use 5s timeout, retry)
        //   3. Write drivers_probe to trigger rebind
        //   4. Poll for final_driver to appear
        let poll_deadline = deadline.saturating_sub(overall.elapsed());
        let poll_start = Instant::now();
        let poll_interval = Duration::from_secs(5);
        let mut override_set = false;
        let mut probe_sent = false;
        let mut final_driver = guarded_sysfs::read_current_driver(&config.bdf);

        while final_driver.as_deref() != Some(config.final_driver.as_str()) {
            if poll_start.elapsed() >= poll_deadline {
                steps.push(HandoffStep {
                    name: "warm_swap".into(), ok: false,
                    detail: Some(format!(
                        "poll for {} timed out (driver={:?}, override_set={}, probe_sent={})",
                        config.final_driver, final_driver, override_set, probe_sent,
                    )),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                return halt_result(&config.bdf, "warm_swap", steps, patch_result,
                                   module_loaded, false, overall, &sibling_state,
                                   &config.module_name, needs_device_rollback);
            }

            if !override_set
                && matches!(
                    guarded_sysfs::sysfs_write_guarded(
                        &override_path, &config.final_driver,
                        Duration::from_secs(5),
                    ),
                    Ok(()),
                )
            {
                override_set = true;
                tracing::info!(bdf = config.bdf.as_str(),
                    "catalyst poll: driver_override set to {}", config.final_driver);
            }

            if override_set && !probe_sent
                && matches!(
                    guarded_sysfs::sysfs_write_guarded(
                        &probe_path, &config.bdf,
                        Duration::from_secs(5),
                    ),
                    Ok(()),
                )
            {
                probe_sent = true;
                tracing::info!(bdf = config.bdf.as_str(),
                    "catalyst poll: drivers_probe sent");
            }

            std::thread::sleep(poll_interval);
            final_driver = guarded_sysfs::read_current_driver(&config.bdf);
        }

        let swap_elapsed = t.elapsed();
        tracing::info!(
            bdf = config.bdf.as_str(),
            final_driver = config.final_driver.as_str(),
            elapsed_s = swap_elapsed.as_secs(),
            "catalyst warm_swap: final driver bound via poll"
        );
        steps.push(HandoffStep {
            name: "warm_swap".into(), ok: true,
            detail: Some(format!("{} → {} (poll-waited {}s)",
                config.seeder_driver, config.final_driver, swap_elapsed.as_secs())),
            duration_ms: swap_elapsed.as_millis() as u64,
        });
    } else {
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
    }

    heartbeat();
    // ── Step 7a: Deferred catalyst full capture (BEFORE sibling rebind) ──
    //
    // For catalyst: capture BAR0 immediately after warm_swap while the
    // register state is warm-preserved. This MUST happen before sibling
    // rebind (step 6b) because rebind_siblings_to_vfio does sysfs
    // writes that contend with the nvidia RM teardown's PCI device lock,
    // blocking for 7+ minutes. The BAR0 mmap via resource0 is safe once
    // vfio-pci owns the device — it bypasses the PCI config lock path.

    if is_catalyst {
        let t = Instant::now();
        let bar0_size = 16 * 1024 * 1024;
        tracing::info!(
            bdf = config.bdf.as_str(),
            pipeline_elapsed_s = overall.elapsed().as_secs(),
            "catalyst profile: starting BAR0 open (from_sysfs_rw)"
        );
        match crate::vfio::device::MappedBar::from_sysfs_rw(&config.bdf, bar0_size) {
            Ok(post_swap_bar0) => {
                tracing::info!(
                    bdf = config.bdf.as_str(),
                    open_ms = t.elapsed().as_millis() as u64,
                    "catalyst profile: BAR0 mmap open succeeded"
                );

                let cap_start = Instant::now();
                let domains = hw.bar0_domains;
                let full_snapshot = Bar0Snapshot::capture_domains(
                    &post_swap_bar0, &config.bdf, "catalyst-post-swap", domains,
                );
                let alive = full_snapshot.alive_count();

                tracing::info!(
                    bdf = config.bdf.as_str(),
                    total_regs = full_snapshot.len(),
                    alive_regs = alive,
                    capture_ms = cap_start.elapsed().as_millis() as u64,
                    open_ms = t.elapsed().as_millis() as u64,
                    num_domains = domains.len(),
                    "catalyst capture: domain-scoped BAR0 snapshot (post-swap, vfio-pci safe)"
                );

                let snapshot_path = format!(
                    "/tmp/toadstool-catalyst-{}.json",
                    config.bdf.replace([':', '.'], "-")
                );
                if let Ok(json) = full_snapshot.to_json() {
                    if let Err(e) = std::fs::write(&snapshot_path, &json) {
                        tracing::warn!(err = %e, path = snapshot_path.as_str(),
                                       "catalyst capture: failed to persist snapshot");
                    } else {
                        tracing::info!(path = snapshot_path.as_str(),
                                       bytes = json.len(),
                                       "catalyst capture: snapshot persisted");
                        catalyst_snapshot_path = Some(snapshot_path.clone());
                    }
                }

                let chip_family = crate::nv::gr_init::ChipFamily::from_sm(hw.sm);
                let replay = full_snapshot.to_catalyst_replay(
                    chip_family,
                    "470.256.02",
                    hw.bar0_domains,
                );
                let replay_path = format!(
                    "/tmp/toadstool-catalyst-replay-{}.json",
                    config.bdf.replace([':', '.'], "-")
                );
                if let Ok(json) = replay.to_json() {
                    if let Err(e) = std::fs::write(&replay_path, &json) {
                        tracing::warn!(err = %e, "catalyst capture: failed to persist replay");
                    } else {
                        tracing::info!(
                            path = replay_path.as_str(),
                            writes = replay.len(),
                            domains = replay.domains().len(),
                            "catalyst capture: replay sequence persisted"
                        );
                    }
                }

                catalyst_alive_count = Some(alive);

                steps.push(HandoffStep {
                    name: "catalyst_full_capture".into(), ok: true,
                    detail: Some(format!(
                        "BAR0 post-swap: {} alive regs, snapshot={}, open_ms={}, capture_ms={}",
                        alive,
                        catalyst_snapshot_path.as_deref().unwrap_or("none"),
                        t.elapsed().as_millis(),
                        cap_start.elapsed().as_millis(),
                    )),
                    duration_ms: t.elapsed().as_millis() as u64,
                });

                // ── FECS INIT_CTXSW: trigger GR init while BAR0 is warm ──
                //
                // CRITICAL: this must happen NOW, using the same BAR0 mapping
                // that captured alive registers. The PRI ring recovery step
                // (below) issues enumerate/start commands that destroy the
                // PRI routing RM set up, causing FECS/TPC reads to PRI-fault.
                // By sending INIT_CTXSW here, FECS is still accessible.
                let fecs_t = Instant::now();
                let fecs_pc = post_swap_bar0
                    .read_u32(falcon::FECS_CTXSW_PC as usize)
                    .unwrap_or(0);
                let fecs_cpuctl = post_swap_bar0
                    .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                    .unwrap_or(0);
                let fecs_os = post_swap_bar0
                    .read_u32((falcon::FECS_BASE + falcon::PC) as usize)
                    .unwrap_or(0);
                let gpccs_cpuctl = post_swap_bar0
                    .read_u32((falcon::GPCCS_BASE + falcon::CPUCTL) as usize)
                    .unwrap_or(0);
                let gpc_en = post_swap_bar0
                    .read_u32(gpc::BCAST_ENABLES as usize)
                    .unwrap_or(0);
                tracing::info!(
                    bdf = config.bdf.as_str(),
                    fecs_pc = format_args!("0x{fecs_pc:08x}"),
                    fecs_os = format_args!("0x{fecs_os:08x}"),
                    fecs_cpuctl = format_args!("0x{fecs_cpuctl:08x}"),
                    gpccs_cpuctl = format_args!("0x{gpccs_cpuctl:08x}"),
                    gpc_enables = format_args!("0x{gpc_en:08x}"),
                    "pre-INIT_CTXSW: FECS state (using warm post-swap BAR0)"
                );

                let fecs_halted = fecs_cpuctl & 0x10 != 0;
                let fecs_alive = fecs_cpuctl & 0xBADF_0000 != 0xBADF_0000;
                if fecs_alive {
                    if fecs_halted {
                        // FECS is halted but accessible — try to unhalt and
                        // restart from its current PC. CPUCTL bit 1 = START_CPU.
                        tracing::info!(bdf = config.bdf.as_str(),
                            "FECS halted — attempting unhalt (CPUCTL START_CPU)");
                        let _ = post_swap_bar0.write_u32(
                            (falcon::FECS_BASE + falcon::CPUCTL) as usize,
                            0x2,
                        ); // START_CPU
                        std::thread::sleep(Duration::from_millis(200));
                        let pc_after = post_swap_bar0
                            .read_u32(falcon::FECS_CTXSW_PC as usize)
                            .unwrap_or(0);
                        let cpuctl_after = post_swap_bar0
                            .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                            .unwrap_or(0);
                        tracing::info!(
                            bdf = config.bdf.as_str(),
                            fecs_pc_after = format_args!("0x{pc_after:08x}"),
                            fecs_cpuctl_after = format_args!("0x{cpuctl_after:08x}"),
                            "FECS unhalt result"
                        );
                    }

                    tracing::info!(bdf = config.bdf.as_str(),
                        "FECS accessible — sending INIT_CTXSW");
                    match crate::vfio::channel::fecs::fecs_init_ctxsw(&post_swap_bar0) {
                        Ok(r) => {
                            std::thread::sleep(Duration::from_millis(1000));
                            let tpc0 = post_swap_bar0
                                .read_u32((gpc::gpc_tpc0(0) + 0x100) as usize)
                                .unwrap_or(0xdead);
                            let gpc_en_post = post_swap_bar0
                                .read_u32(gpc::BCAST_ENABLES as usize)
                                .unwrap_or(0);
                            let gpccs_post = post_swap_bar0
                                .read_u32((falcon::GPCCS_BASE + falcon::CPUCTL) as usize)
                                .unwrap_or(0);
                            tracing::info!(
                                bdf = config.bdf.as_str(),
                                status = r.status,
                                mailbox0 = format_args!("0x{:08x}", r.mailbox0),
                                tpc0_ctrl = format_args!("0x{tpc0:08x}"),
                                gpc_enables = format_args!("0x{gpc_en_post:08x}"),
                                gpccs_cpuctl = format_args!("0x{gpccs_post:08x}"),
                                "FECS INIT_CTXSW result (pre-PRI-recovery)"
                            );
                            steps.push(HandoffStep {
                                name: "fecs_init_ctxsw".into(), ok: r.status == 0,
                                detail: Some(format!(
                                    "status={}, mb0=0x{:08x}, tpc0=0x{tpc0:08x}, gpc_en=0x{gpc_en_post:08x}",
                                    r.status, r.mailbox0
                                )),
                                duration_ms: fecs_t.elapsed().as_millis() as u64,
                            });
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "FECS INIT_CTXSW failed");
                            steps.push(HandoffStep {
                                name: "fecs_init_ctxsw".into(), ok: false,
                                detail: Some(format!("failed: {e}")),
                                duration_ms: fecs_t.elapsed().as_millis() as u64,
                            });
                        }
                    }
                } else {
                    tracing::warn!(
                        bdf = config.bdf.as_str(),
                        fecs_cpuctl = format_args!("0x{fecs_cpuctl:08x}"),
                        "FECS not accessible post-swap — skipping INIT_CTXSW"
                    );
                    steps.push(HandoffStep {
                        name: "fecs_init_ctxsw".into(), ok: false,
                        detail: Some(format!("FECS PRI fault: cpuctl=0x{fecs_cpuctl:08x}")),
                        duration_ms: fecs_t.elapsed().as_millis() as u64,
                    });
                }

                // ── Early tier classification using warm BAR0 ──
                // The PRI ring recovery step below issues enumerate/start
                // commands that destroy RM's PRI routing. Classify tier NOW
                // while the BAR0 still reflects RM's warm state.
                let _tier_t = Instant::now();
                let warm_tier = classify_tier(&post_swap_bar0);
                tracing::info!(
                    bdf = config.bdf.as_str(),
                    tier = ?warm_tier.tier,
                    tpc_alive = warm_tier.tpc_alive,
                    gpc_enables = warm_tier.gpc_enables,
                    tpc_status = warm_tier.tpc_status.map(|v| format!("0x{v:08x}")),
                    "early tier classification (warm BAR0, pre-PRI-recovery)"
                );
                catalyst_tier = Some(warm_tier);
            }
            Err(e) => {
                tracing::warn!(
                    bdf = config.bdf.as_str(),
                    err = %e,
                    open_ms = t.elapsed().as_millis() as u64,
                    "catalyst capture: post-swap BAR0 open failed"
                );
                steps.push(HandoffStep {
                    name: "catalyst_full_capture".into(), ok: false,
                    detail: Some(format!("post-swap BAR0 open failed ({}ms): {e}",
                        t.elapsed().as_millis())),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // ── Step 6b: Rebind IOMMU siblings to vfio-pci ─────────────────

    {
        let t = Instant::now();
        tracing::info!(
            bdf = config.bdf.as_str(),
            num_siblings = sibling_state.len(),
            pipeline_elapsed_s = overall.elapsed().as_secs(),
            "catalyst profile: starting sibling rebind"
        );
        if !sibling_state.is_empty() {
            guarded_sysfs::rebind_siblings_to_vfio(&sibling_state);
        }
        let sib_ms = t.elapsed().as_millis() as u64;
        tracing::info!(
            bdf = config.bdf.as_str(),
            elapsed_ms = sib_ms,
            "catalyst profile: sibling rebind complete"
        );
        if sib_ms > 1000 {
            steps.push(HandoffStep {
                name: "sibling_rebind".into(), ok: true,
                detail: Some(format!("{} siblings, {}ms", sibling_state.len(), sib_ms)),
                duration_ms: sib_ms,
            });
        }
    }

    // ── Step 6c: PRI Ring Recovery ────────────────────────────────────
    //
    // After PCI unbind, the kernel PCI framework disables PGRAPH, which
    // kills PRI ring routing to GPC/TPC/FECS/GPCCS. We re-enable PGRAPH
    // and re-enumerate PRI ring stations to restore hardware accessibility.

    if is_catalyst {
        let t = Instant::now();
        match recover_pri_ring(&config.bdf, hw.chip_name) {
            Ok(detail) => {
                steps.push(HandoffStep {
                    name: "pri_ring_recovery".into(), ok: true,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                tracing::warn!(bdf = config.bdf.as_str(), error = %e,
                    "PRI ring recovery failed (non-fatal)");
                steps.push(HandoffStep {
                    name: "pri_ring_recovery".into(), ok: false,
                    detail: Some(format!("recovery failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    heartbeat();
    // ── Step 7: Tier Classification ─────────────────────────────────

    let mut tier = if is_catalyst {
        if let Some(ref ct) = catalyst_tier {
            // Use the early tier classification captured with warm BAR0
            // (before PRI ring recovery destroyed PRI routing).
            let t = Instant::now();
            steps.push(HandoffStep {
                name: "tier_classify".into(), ok: true,
                detail: Some(format!("{} (warm BAR0, pre-PRI-recovery)", ct.tier)),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            catalyst_tier.take()
        } else {
            None
        }
    } else {
        None
    };

    if tier.is_none() {
        tier = if let Some(b) = bar0 {
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
    }

    // ── Step 7b: Catalyst Preservation ────────────────────────────
    //
    // For catalyst strategies: archive the patched .ko (frozen starter)
    // and recipe JSON before module cleanup deletes the tmpfile.

    if is_catalyst
        && let Some(ref pr) = patch_result
    {
            let t = Instant::now();
            let frozen_dir = "/var/lib/toadstool/catalysts/frozen";
            let _ = std::fs::create_dir_all(frozen_dir);
            let krel = crate::linux_paths::kernel_release().unwrap_or("unknown");
            let frozen_dest = format!(
                "{}/nvsov_gv100_470.256.02_k{}.ko",
                frozen_dir, krel,
            );
            match std::fs::copy(&pr.patched_path, &frozen_dest) {
                Ok(bytes) => {
                    tracing::info!(
                        src = pr.patched_path.as_str(),
                        dest = frozen_dest.as_str(),
                        bytes,
                        "catalyst preserve: frozen .ko archived"
                    );
                    steps.push(HandoffStep {
                        name: "catalyst_preserve".into(), ok: true,
                        detail: Some(format!("frozen .ko: {} ({bytes} bytes)", frozen_dest)),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    tracing::warn!(err = %e, "catalyst preserve: failed to archive frozen .ko");
                    steps.push(HandoffStep {
                        name: "catalyst_preserve".into(), ok: false,
                        detail: Some(format!("frozen .ko copy failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
            }

            // Persist recipe JSON (PatchSet serialization)
            let recipe_dir = "/var/lib/toadstool/catalysts/recipes";
            let _ = std::fs::create_dir_all(recipe_dir);
            let patch_set_name = match &config.module_source {
                ModuleSourceConfig::DkmsPatched { patch_set, .. } => patch_set.clone(),
                ModuleSourceConfig::Patched { patch_set, .. } => patch_set.clone(),
                ModuleSourceConfig::System => "system".into(),
            };
            if let Some(ps) = module_patch::PatchSet::by_name(&patch_set_name)
                && let Ok(json) = ps.to_json()
            {
                let recipe_path = format!("{recipe_dir}/gv100_nvidia470_patchset.json");
                let _ = std::fs::write(&recipe_path, &json);
                tracing::info!(path = recipe_path.as_str(), "catalyst preserve: recipe JSON persisted");
            }
    }

    heartbeat();
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

    // ── Step 9: Restore reset capabilities ─────────────────────────────
    //
    // Re-enable default PCI reset methods so that subsequent cold resets
    // (e.g. VFIO group teardown) can issue FLR normally, and unload the
    // no_bus_reset module to re-enable SBR.
    guarded_sysfs::restore_flr(&config.bdf);
    if let Err(e) = guarded_sysfs::restore_bus_reset() {
        tracing::warn!(error = %e, "failed to unload no_bus_reset module (non-fatal)");
    }

    // _handoff_guard drops here, releasing the per-BDF lock

    // ── Build PRI ring anchor from boot service evidence ─────────
    let pri_ring_anchor = boot_evidence.as_ref().map(|ev| {
        let mut anchor = PriRingAnchor::from_evidence(&config.bdf, ev.clone());
        // Post-recovery health: probe current BAR0 state to classify
        let health = if let Ok(bar0) = crate::vfio::device::MappedBar::from_sysfs_rw(
            &config.bdf, 16 * 1024 * 1024,
        ) {
            let pmc = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
            let fecs = bar0
                .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                .unwrap_or(0xDEAD);
            let pgraph_on = pmc & (1 << 12) != 0;
            let fecs_ok = fecs & 0xBADF_0000 != 0xBADF_0000;
            let tpc0 = bar0
                .read_u32(gpc::gpc_tpc0(0) as usize)
                .unwrap_or(0xBADF);
            let tpc_ok = tpc0 & 0xBADF_0000 != 0xBADF_0000;
            if pgraph_on && fecs_ok && tpc_ok {
                PriRingHealth::Healthy
            } else if pgraph_on && fecs_ok {
                // PGRAPH on, falcons accessible, but TPC/GPC sub-ring not working
                // This is the expected state after PRI ring recovery (falcon HS-locked)
                PriRingHealth::Degraded { faulted_domains: 1 }
            } else {
                PriRingHealth::Destroyed
            }
        } else {
            PriRingHealth::Destroyed
        };
        anchor.update_health(health);
        tracing::info!(
            bdf = config.bdf.as_str(),
            health = ?anchor.health,
            compute_ready = anchor.is_compute_ready(),
            needs_reboot = anchor.needs_reboot(),
            "PRI ring anchor created from post-recovery state"
        );
        anchor
    });

    HandoffResult {
        bdf: config.bdf.clone(),
        success: true,
        halted_at: None,
        steps,
        patch_result,
        tier,
        module_loaded,
        module_unloaded,
        catalyst_snapshot_path,
        catalyst_alive_count,
        catalyst_tier,
        rm_channel_evidence,
        boot_service_evidence: boot_evidence,
        pri_ring_anchor,
        total_ms: overall.elapsed().as_millis() as u64,
    }
}
