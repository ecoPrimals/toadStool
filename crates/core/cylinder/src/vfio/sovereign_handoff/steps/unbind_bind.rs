// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::time::Instant;

use std::fmt::Write;

use crate::vfio::guarded_sysfs;

use super::super::pipeline::PipelineContext;
use super::super::rollback::{halt_result, halt_result_poisoned};
use super::super::types::{HandoffResult, HandoffStep, ModuleSourceConfig};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        // ── Step 2: Unbind current driver + IOMMU group siblings ────────

        let t = Instant::now();
        ctx.sibling_state = guarded_sysfs::unbind_iommu_siblings(&ctx.config.bdf);
        let prev_driver = guarded_sysfs::read_current_driver(&ctx.config.bdf);

        if let Some(ref current) = prev_driver {
            let unbind_path = crate::linux_paths::sysfs_pci_driver_unbind(current);
            if let Err(e) = guarded_sysfs::sysfs_write_guarded(
                &unbind_path, &ctx.config.bdf, guarded_sysfs::UNBIND_TIMEOUT,
            ) {
                tracing::warn!(bdf = ctx.config.bdf.as_str(), driver = current.as_str(),
                               error = %e, "guarded unbind failed (continuing)");
            }
        }

        let sibling_summary: Vec<String> = ctx.sibling_state.iter()
            .map(|(s, d)| format!("{s}: {} → unbound", d.as_deref().unwrap_or("none")))
            .collect();
        let mut detail_msg = prev_driver.map(|d| format!("was: {d}"))
            .unwrap_or_else(|| "unbound".into());
        if !sibling_summary.is_empty() {
            let _ = write!(detail_msg, "; siblings: [{}]", sibling_summary.join(", "));
        }

        // Verify all siblings actually unbound
        let siblings_clean = ctx.sibling_state.iter().all(|(s, _)| guarded_sysfs::read_current_driver(s).is_none());
        let target_clean = guarded_sysfs::read_current_driver(&ctx.config.bdf).is_none();
        let unbind_ok = siblings_clean && target_clean;

        if !unbind_ok {
            detail_msg.push_str(" [WARN: not all devices fully unbound]");
        }

        ctx.steps.push(HandoffStep {
            name: "unbind_current".into(),
            ok: unbind_ok,
            detail: Some(detail_msg),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        if !unbind_ok {

            return Some(halt_result(&ctx.config.bdf, "unbind_current", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                               ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                               &ctx.config.module_name, true));
        }

        // Device has been unbound — rollback must restore to vfio-pci on any failure
        ctx.needs_device_rollback = true;

        // ── Deferred insmod for dual-load ───────────────────────────────
        // The GPU is now unbound from vfio-pci. Set driver_override to our
        // renamed module name so the kernel binds this device to our module
        // (not the host nvidia) when we insmod.
        if let ModuleSourceConfig::DkmsPatched { .. } = &ctx.config.module_source
            && !ctx.module_loaded
            && let Some(ref pr) = ctx.patch_result
        {
                    // Disable the PCI device so the kernel releases its
                    // BAR resource claims. Without this, nvidia's direct
                    // request_mem_region call on BAR0 fails because the
                    // PCI subsystem still has the region reserved from the
                    // previous driver's pci_enable_device.
                    // Direct sysfs_write is safe here: the device is unbound
                    // and the `enable` attribute is a non-blocking kernel op.
                    let enable_path = crate::linux_paths::sysfs_pci_device_file(
                        &ctx.config.bdf, "enable",
                    );
                    match guarded_sysfs::sysfs_write(&enable_path, "0") {
                        Ok(()) => tracing::info!(bdf = ctx.config.bdf.as_str(),
                            "pci device disabled — BAR resources released for driver takeover"),
                        Err(e) => tracing::warn!(bdf = ctx.config.bdf.as_str(), error = %e,
                            "pci disable failed (continuing — request_mem_region may fail)"),
                    }

                    let override_path = crate::linux_paths::sysfs_pci_device_file(
                        &ctx.config.bdf, "driver_override",
                    );
                    if let Err(e) = guarded_sysfs::sysfs_write_guarded(
                        &override_path, &ctx.config.module_name,
                        guarded_sysfs::UNBIND_TIMEOUT,
                    ) {
                        tracing::warn!(error = %e, "driver_override write failed (continuing)");
                    }

                    let patched_path = PathBuf::from(&pr.patched_path);
                    let t = Instant::now();
                    match guarded_sysfs::insmod_guarded(&patched_path, guarded_sysfs::INSMOD_TIMEOUT) {
                        Ok(()) => {
                            ctx.module_loaded = true;
                            // Trigger re-probe so the device binds to our module
                            let probe_path = crate::linux_paths::sysfs_pci_driver_bind(
                                &ctx.config.module_name,
                            );
                            let _ = guarded_sysfs::sysfs_write_guarded(
                                &probe_path, &ctx.config.bdf,
                                guarded_sysfs::PROBE_TIMEOUT,
                            );
                            ctx.steps.push(HandoffStep {
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
                                tracing::error!(bdf = ctx.config.bdf.as_str(),
                                    "insmod TIMED OUT — device likely D-state poisoned. \
                                     Skipping all sysfs ops to protect ember.");
                            } else {
                                // Safe to touch sysfs — insmod failed fast (e.g. ENODEV, EBUSY)
                                let _ = guarded_sysfs::sysfs_write_guarded(
                                    &override_path, "",
                                    guarded_sysfs::UNBIND_TIMEOUT,
                                );
                            }

                            ctx.steps.push(HandoffStep {
                                name: "deferred_insmod".into(), ok: false,
                                detail: Some(format!("deferred insmod failed: {e}")),
                                duration_ms: t.elapsed().as_millis() as u64,
                            });

                            if poisoned {
                                return Some(halt_result_poisoned(
                                    &ctx.config.bdf, "deferred_insmod", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                                    false, false, ctx.overall, &ctx.sibling_state,
                                    &ctx.config.module_name, true));
                            }
                            return Some(halt_result(&ctx.config.bdf, "deferred_insmod", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                                               false, false, ctx.overall, &ctx.sibling_state,
                                               &ctx.config.module_name, true));
                        }
                    }
        }

        ctx.heartbeat();
        // ── Step 3: Bind seeder driver (GUARDED) ────────────────────────

        let t = Instant::now();
        let override_path = crate::linux_paths::sysfs_pci_device_file(&ctx.config.bdf, "driver_override");
        if let Err(e) = guarded_sysfs::sysfs_write(&override_path, &ctx.config.seeder_driver) {
            ctx.steps.push(HandoffStep {
                name: "seeder_bind".into(), ok: false,
                detail: Some(format!("driver_override failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });

            return Some(halt_result(&ctx.config.bdf, "seeder_bind", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                               ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                               &ctx.config.module_name, ctx.needs_device_rollback));
        }

        let probe_path = crate::linux_paths::sysfs_pci_drivers_probe();
        if let Err(e) = guarded_sysfs::sysfs_write_guarded(
            &probe_path, &ctx.config.bdf, guarded_sysfs::PROBE_TIMEOUT,
        ) {
            ctx.steps.push(HandoffStep {
                name: "seeder_bind".into(), ok: false,
                detail: Some(format!("guarded drivers_probe failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });

            return Some(halt_result(&ctx.config.bdf, "seeder_bind", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                               ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                               &ctx.config.module_name, ctx.needs_device_rollback));
        }

        let bound = guarded_sysfs::read_current_driver(&ctx.config.bdf);
        let bind_ok = bound.as_deref() == Some(ctx.config.seeder_driver.as_str());
        ctx.steps.push(HandoffStep {
            name: "seeder_bind".into(), ok: bind_ok,
            detail: Some(format!("driver={} expected={}",
                bound.as_deref().unwrap_or("none"), ctx.config.seeder_driver)),
            duration_ms: t.elapsed().as_millis() as u64,
        });
        if !bind_ok {

            return Some(halt_result(&ctx.config.bdf, "seeder_bind", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                               ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                               &ctx.config.module_name, ctx.needs_device_rollback));
        }

    ctx.override_path = override_path;
    ctx.probe_path = probe_path;

    None
}
