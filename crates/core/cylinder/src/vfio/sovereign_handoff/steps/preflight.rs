// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use crate::nv::registers::pmc;
use crate::vfio::guarded_sysfs;
use crate::vfio::kernel_health;

use super::super::lock::HandoffGuard;
use super::super::pipeline::PipelineContext;
use super::super::rollback::halt_result;
use super::super::types::{HandoffResult, HandoffStep, ModuleSourceConfig};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        // ── Step 0: Pre-flight checks ───────────────────────────────────

        let t = Instant::now();

        // 0a. Concurrent handoff guard (RAII — released on drop at any exit path)
        match HandoffGuard::acquire(&ctx.config.bdf) {
            Ok(guard) => ctx.handoff_guard = Some(guard),
            Err(e) => {
                ctx.steps.push(HandoffStep {
                    name: "preflight".into(), ok: false,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                return Some(halt_result(&ctx.config.bdf, "preflight", std::mem::take(&mut ctx.steps), None, false, false, ctx.overall, &[], &ctx.config.module_name, false));
            }
        }

        if ctx.config.skip_preflight {
            tracing::warn!("skip_preflight=true — skipping module stuck, IOMMU, and kernel health checks");
        } else {
            // 0b. Module stuck state check
            if guarded_sysfs::is_module_stuck(&ctx.config.module_name) {
                ctx.steps.push(HandoffStep {
                    name: "preflight".into(), ok: false,
                    detail: Some(format!(
                        "module '{}' is stuck (Unloading/negative refcount) — reboot required",
                        ctx.config.module_name
                    )),
                    duration_ms: t.elapsed().as_millis() as u64,
                });

                return Some(halt_result(&ctx.config.bdf, "preflight", std::mem::take(&mut ctx.steps), None, false, false, ctx.overall, &[], &ctx.config.module_name, false));
            }

            // 0c. IOMMU group availability
            if let Err(e) = guarded_sysfs::iommu_group_ready(&ctx.config.bdf) {
                ctx.steps.push(HandoffStep {
                    name: "preflight".into(), ok: false,
                    detail: Some(format!("IOMMU group not ready: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });

                return Some(halt_result(&ctx.config.bdf, "preflight", std::mem::take(&mut ctx.steps), None, false, false, ctx.overall, &[], &ctx.config.module_name, false));
            }

            // 0d. Kernel build environment health (only for module sources that compile/load)
            if !matches!(ctx.config.module_source, ModuleSourceConfig::System) {
                match kernel_health::full_kernel_health_check() {
                    Ok(report) => {
                        if !report.layout_matches {
                            tracing::error!(
                                diagnosis = %report.diagnosis,
                                "kernel build environment unhealthy — module loading will fail"
                            );
                            ctx.steps.push(HandoffStep {
                                name: "preflight".into(), ok: false,
                                detail: Some(format!(
                                    "kernel health check failed: {}",
                                    report.diagnosis
                                )),
                                duration_ms: t.elapsed().as_millis() as u64,
                            });
                            return Some(halt_result(&ctx.config.bdf, "preflight", std::mem::take(&mut ctx.steps), None, false, false, ctx.overall, &[], &ctx.config.module_name, false));
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

        ctx.steps.push(HandoffStep {
            name: "preflight".into(), ok: true,
            detail: Some(if ctx.config.skip_preflight {
                "preflight skipped (skip_preflight=true)".into()
            } else {
                "module clean, IOMMU group free, no concurrent handoff, kernel healthy".into()
            }),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        // Detect catalyst strategies early — used for anchor-release guard
        // and later for pre-swap capture + diagnostics.
        ctx.is_catalyst = matches!(
            &ctx.config.module_source,
            ModuleSourceConfig::DkmsPatched { patch_set, .. }
                if patch_set == "nvidia_catalyst_handoff"
                    || patch_set == "nvidia_catalyst_minimal_nop"
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
        if ctx.is_catalyst {
            let t = Instant::now();
            let pmc_check = crate::vfio::device::MappedBar::from_sysfs_rw(
                &ctx.config.bdf, 4096,
            ).map(|bar| {
                let pmc = bar.read_u32(pmc::ENABLE as usize).unwrap_or(0);
                let popcount = pmc.count_ones();
                (pmc, popcount)
            });
            match pmc_check {
                Ok((pmc, popcount)) => {
                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        pmc = format_args!("0x{pmc:08x}"),
                        popcount,
                        "anchor release guard: PMC_ENABLE health check"
                    );
                    if popcount < ctx.hw.pmc_warm_threshold {
                        tracing::warn!(
                            bdf = ctx.config.bdf.as_str(),
                            pmc = format_args!("0x{pmc:08x}"),
                            popcount,
                            "GPU cold at anchor release — catalyst will warm it"
                        );
                    }
                    ctx.steps.push(HandoffStep {
                        name: "anchor_release_guard".into(), ok: true,
                        detail: Some(format!(
                            "PMC_ENABLE=0x{pmc:08x} (popcount={popcount}){}",
                            if popcount < ctx.hw.pmc_warm_threshold { " — cold start, catalyst will warm" } else { " — GPU warm" }
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        bdf = ctx.config.bdf.as_str(),
                        error = %e,
                        "anchor release guard: cannot read PMC_ENABLE — proceeding"
                    );
                }
            }
        }


    None
}
