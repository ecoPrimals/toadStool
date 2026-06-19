// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::time::Instant;

use crate::vfio::guarded_sysfs;
use crate::vfio::kmod;
use crate::vfio::module_patch::{self, PatchSet};

use super::super::module_deps::load_module_dependencies;
use super::super::pipeline::PipelineContext;
use super::super::rollback::{deadline_exceeded, halt_result};
use super::super::types::{HandoffResult, HandoffStep, ModuleSourceConfig};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
    // ── Step 1: Module Preparation ──────────────────────────────────

    let t = Instant::now();
    match &ctx.config.module_source {
        ModuleSourceConfig::Patched {
            stock_module,
            patch_set,
        } => {
            if kmod::is_module_loaded(&ctx.config.module_name) {
                tracing::info!(
                    module = ctx.config.module_name.as_str(),
                    "module already loaded — guarded unload before patched load"
                );
                if let Err(e) = guarded_sysfs::rmmod_guarded(
                    &ctx.config.module_name,
                    guarded_sysfs::RMMOD_TIMEOUT,
                ) {
                    ctx.steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!(
                            "cannot unload existing {}: {e}",
                            ctx.config.module_name
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return Some(halt_result(
                        &ctx.config.bdf,
                        "module_prep",
                        std::mem::take(&mut ctx.steps),
                        None,
                        false,
                        false,
                        ctx.overall,
                        &[],
                        &ctx.config.module_name,
                        false,
                    ));
                }
            }

            let ps = if let Some(ref json) = ctx.config.patch_set_override {
                match PatchSet::from_json(json) {
                    Ok(ps) => ps,
                    Err(e) => {
                        ctx.steps.push(HandoffStep {
                            name: "module_prep".into(),
                            ok: false,
                            detail: Some(format!("invalid patch_set_override JSON: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return Some(halt_result(
                            &ctx.config.bdf,
                            "module_prep",
                            std::mem::take(&mut ctx.steps),
                            None,
                            false,
                            false,
                            ctx.overall,
                            &[],
                            &ctx.config.module_name,
                            false,
                        ));
                    }
                }
            } else {
                match PatchSet::by_name(patch_set) {
                    Some(ps) => ps,
                    None => {
                        ctx.steps.push(HandoffStep {
                            name: "module_prep".into(),
                            ok: false,
                            detail: Some(format!("unknown patch set: {patch_set}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return Some(halt_result(
                            &ctx.config.bdf,
                            "module_prep",
                            std::mem::take(&mut ctx.steps),
                            None,
                            false,
                            false,
                            ctx.overall,
                            &[],
                            &ctx.config.module_name,
                            false,
                        ));
                    }
                }
            };

            let stock_path = match kmod::find_stock_module(stock_module) {
                Ok(p) => p,
                Err(e) => {
                    ctx.steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("stock module lookup failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return Some(halt_result(
                        &ctx.config.bdf,
                        "module_prep",
                        std::mem::take(&mut ctx.steps),
                        None,
                        false,
                        false,
                        ctx.overall,
                        &[],
                        &ctx.config.module_name,
                        false,
                    ));
                }
            };

            let rename_pair = if stock_module != &ctx.config.module_name {
                Some((stock_module.as_str(), ctx.config.module_name.as_str()))
            } else {
                None
            };

            match module_patch::patch_module_with_rename(&stock_path, &ps, rename_pair) {
                Ok(pr) => {
                    let patched_path = PathBuf::from(&pr.patched_path);
                    ctx.patch_result = Some(pr);

                    // Load module dependencies before insmod. insmod doesn't
                    // resolve deps like modprobe — we use modprobe --dry-run
                    // to discover the chain and load each dep.
                    if let Err(e) = load_module_dependencies(stock_module) {
                        tracing::warn!(module = stock_module.as_str(), error = %e,
                                           "failed to load module dependencies (continuing)");
                    }

                    if let Err(e) =
                        guarded_sysfs::insmod_guarded(&patched_path, guarded_sysfs::INSMOD_TIMEOUT)
                    {
                        ctx.steps.push(HandoffStep {
                            name: "module_prep".into(),
                            ok: false,
                            detail: Some(format!("guarded insmod failed: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });

                        return Some(halt_result(
                            &ctx.config.bdf,
                            "module_prep",
                            std::mem::take(&mut ctx.steps),
                            ctx.patch_result.take(),
                            false,
                            false,
                            ctx.overall,
                            &[],
                            &ctx.config.module_name,
                            false,
                        ));
                    }
                    ctx.module_loaded = true;
                }
                Err(e) => {
                    ctx.steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("module patching failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return Some(halt_result(
                        &ctx.config.bdf,
                        "module_prep",
                        std::mem::take(&mut ctx.steps),
                        None,
                        false,
                        false,
                        ctx.overall,
                        &[],
                        &ctx.config.module_name,
                        false,
                    ));
                }
            }

            let patch_detail = ctx
                .patch_result
                .as_ref()
                .map(|pr| {
                    format!(
                        "patched module loaded (guarded, {}/{} patches applied)",
                        pr.applied_count, pr.total_count
                    )
                })
                .unwrap_or_else(|| "patched module loaded (guarded)".into());
            ctx.steps.push(HandoffStep {
                name: "module_prep".into(),
                ok: true,
                detail: Some(patch_detail),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        ModuleSourceConfig::DkmsPatched {
            dkms_module,
            dkms_version,
            patch_set,
        } => {
            if kmod::is_module_loaded(&ctx.config.module_name) {
                tracing::info!(
                    module = ctx.config.module_name.as_str(),
                    "module already loaded — guarded unload before DKMS patched load"
                );
                if let Err(e) = guarded_sysfs::rmmod_guarded(
                    &ctx.config.module_name,
                    guarded_sysfs::RMMOD_TIMEOUT,
                ) {
                    ctx.steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!(
                            "cannot unload existing {}: {e}",
                            ctx.config.module_name
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return Some(halt_result(
                        &ctx.config.bdf,
                        "module_prep",
                        std::mem::take(&mut ctx.steps),
                        None,
                        false,
                        false,
                        ctx.overall,
                        &[],
                        &ctx.config.module_name,
                        false,
                    ));
                }
            }

            let ps = if let Some(ref json) = ctx.config.patch_set_override {
                match PatchSet::from_json(json) {
                    Ok(ps) => ps,
                    Err(e) => {
                        ctx.steps.push(HandoffStep {
                            name: "module_prep".into(),
                            ok: false,
                            detail: Some(format!("invalid patch_set_override JSON: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return Some(halt_result(
                            &ctx.config.bdf,
                            "module_prep",
                            std::mem::take(&mut ctx.steps),
                            None,
                            false,
                            false,
                            ctx.overall,
                            &[],
                            &ctx.config.module_name,
                            false,
                        ));
                    }
                }
            } else {
                match PatchSet::by_name(patch_set) {
                    Some(ps) => ps,
                    None => {
                        ctx.steps.push(HandoffStep {
                            name: "module_prep".into(),
                            ok: false,
                            detail: Some(format!("unknown patch set: {patch_set}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return Some(halt_result(
                            &ctx.config.bdf,
                            "module_prep",
                            std::mem::take(&mut ctx.steps),
                            None,
                            false,
                            false,
                            ctx.overall,
                            &[],
                            &ctx.config.module_name,
                            false,
                        ));
                    }
                }
            };

            let stock_path = match kmod::find_dkms_module(dkms_module, dkms_version) {
                Ok(p) => p,
                Err(e) => {
                    ctx.steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!(
                            "DKMS module lookup failed for {dkms_module}/{dkms_version}: {e}"
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return Some(halt_result(
                        &ctx.config.bdf,
                        "module_prep",
                        std::mem::take(&mut ctx.steps),
                        None,
                        false,
                        false,
                        ctx.overall,
                        &[],
                        &ctx.config.module_name,
                        false,
                    ));
                }
            };

            let rename_pair = if dkms_module != &ctx.config.module_name {
                Some((dkms_module.as_str(), ctx.config.module_name.as_str()))
            } else {
                None
            };

            // For dual-load (renamed) modules, run objcopy BEFORE patching.
            // This strips __ksymtab export sections that cause "duplicate
            // symbol" errors, and ensures that all subsequent ELF
            // manipulation (normalization, NOPs, relocation nullification)
            // operates on the final ELF layout.
            let patch_source = if rename_pair.is_some() {
                let staging = std::env::temp_dir()
                    .join(format!("toadstool-staging-{}.ko", ctx.config.module_name));
                if let Err(e) = std::fs::copy(&stock_path, &staging) {
                    ctx.steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("failed to copy DKMS module to staging: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return Some(halt_result(
                        &ctx.config.bdf,
                        "module_prep",
                        std::mem::take(&mut ctx.steps),
                        None,
                        false,
                        false,
                        ctx.overall,
                        &[],
                        &ctx.config.module_name,
                        false,
                    ));
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
                        let staging = std::env::temp_dir()
                            .join(format!("toadstool-staging-{}.ko", ctx.config.module_name));
                        let _ = std::fs::remove_file(&staging);
                    }

                    if !deferred_insmod {
                        let patched_path = PathBuf::from(&pr.patched_path);
                        if let Err(e) = guarded_sysfs::insmod_guarded(
                            &patched_path,
                            guarded_sysfs::INSMOD_TIMEOUT,
                        ) {
                            ctx.steps.push(HandoffStep {
                                name: "module_prep".into(),
                                ok: false,
                                detail: Some(format!("guarded insmod DKMS module failed: {e}")),
                                duration_ms: t.elapsed().as_millis() as u64,
                            });
                            ctx.patch_result = Some(pr);
                            return Some(halt_result(
                                &ctx.config.bdf,
                                "module_prep",
                                std::mem::take(&mut ctx.steps),
                                ctx.patch_result.take(),
                                false,
                                false,
                                ctx.overall,
                                &[],
                                &ctx.config.module_name,
                                false,
                            ));
                        }
                        ctx.module_loaded = true;
                    }
                    ctx.patch_result = Some(pr);
                }
                Err(e) => {
                    ctx.steps.push(HandoffStep {
                        name: "module_prep".into(),
                        ok: false,
                        detail: Some(format!("DKMS module patching failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    return Some(halt_result(
                        &ctx.config.bdf,
                        "module_prep",
                        std::mem::take(&mut ctx.steps),
                        None,
                        false,
                        false,
                        ctx.overall,
                        &[],
                        &ctx.config.module_name,
                        false,
                    ));
                }
            }

            let patch_detail = ctx
                .patch_result
                .as_ref()
                .map(|pr| {
                    format!(
                        "DKMS patched module {} ({}/{} patches, {})",
                        if deferred_insmod {
                            "prepared"
                        } else {
                            "loaded"
                        },
                        pr.applied_count,
                        pr.total_count,
                        rename_pair
                            .map(|(o, n)| format!("renamed {o}→{n}"))
                            .unwrap_or_else(|| "no rename".into())
                    )
                })
                .unwrap_or_else(|| "DKMS patched module prepared".into());
            ctx.steps.push(HandoffStep {
                name: "module_prep".into(),
                ok: true,
                detail: Some(patch_detail),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        ModuleSourceConfig::System => match kmod::ensure_module_loaded(&ctx.config.module_name) {
            Ok(freshly_loaded) => {
                ctx.module_loaded = freshly_loaded;
                ctx.steps.push(HandoffStep {
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
                ctx.steps.push(HandoffStep {
                    name: "module_prep".into(),
                    ok: false,
                    detail: Some(format!("system module load failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });

                return Some(halt_result(
                    &ctx.config.bdf,
                    "module_prep",
                    std::mem::take(&mut ctx.steps),
                    None,
                    false,
                    false,
                    ctx.overall,
                    &[],
                    &ctx.config.module_name,
                    false,
                ));
            }
        },
    }

    // ── Deadline check ──────────────────────────────────────────────

    if ctx.overall.elapsed() >= ctx.deadline {
        return Some(deadline_exceeded(
            &ctx.config.bdf,
            std::mem::take(&mut ctx.steps),
            ctx.patch_result.take(),
            ctx.module_loaded,
            &ctx.config.module_name,
            &ctx.sibling_state,
            ctx.overall,
        ));
    }

    None
}
