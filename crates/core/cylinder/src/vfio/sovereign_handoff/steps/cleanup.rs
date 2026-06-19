// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use crate::vfio::guarded_sysfs;
use crate::vfio::module_patch;

use super::super::pipeline::{PipelineContext, PipelineSignal};
use super::super::types::{HandoffResult, HandoffStep};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
    // ── Step 8: Module Cleanup (GUARDED) ────────────────────────────
    //
    // SAFETY: When nv_close_device is NOP'd (catalyst), the NVIDIA
    // IRQ handler is still registered via request_irq(). If we rmmod
    // the module, its code pages get freed, but the kernel's IRQ
    // dispatch table still points to the freed handler function.
    // The next GPU interrupt → page fault → hard lockup.
    //
    // Skip rmmod for catalyst handoffs — leave the module loaded as
    // a zombie. Its code stays resident so the IRQ handler remains
    // valid, preventing use-after-free crashes.

    ctx.module_unloaded = false;
    ctx.signal(PipelineSignal::EnterModuleCleanup);
    if ctx.module_loaded {
        if ctx.is_catalyst {
            // NOP'd nv_close_device means NVIDIA RM's kernel timers,
            // delayed work, and workqueue items are still registered.
            // The IRQ clutch only frees the IRQ handler and MSI vectors
            // — it does NOT cancel RM-internal timers. If we rmmod the
            // module, the timer callbacks jump to freed code pages →
            // use-after-free hard lockup (typically within 100ms).
            //
            // Leave the module loaded as a zombie. Its code stays
            // resident so any surviving RM timers fire safely. The
            // module's refcount will be 0 (device unbound), but the
            // code pages remain valid until the next reboot.
            tracing::warn!(
                module = ctx.config.module_name.as_str(),
                irq_clutch_engaged = ctx.irq_clutch_engaged,
                "catalyst: SKIPPING rmmod — RM kernel timers may still \
                     be active. Module kept loaded to prevent use-after-free."
            );
            ctx.steps.push(HandoffStep {
                name: "module_cleanup".into(),
                ok: true,
                detail: Some(format!(
                    "SKIPPED rmmod {} — catalyst zombie (RM timers may be live)",
                    ctx.config.module_name,
                )),
                duration_ms: 0,
            });
        } else {
            let t = Instant::now();
            match guarded_sysfs::rmmod_guarded(
                &ctx.config.module_name,
                guarded_sysfs::RMMOD_TIMEOUT,
            ) {
                Ok(()) => {
                    ctx.module_unloaded = true;
                    let _ = module_patch::cleanup_patched_module(&ctx.config.module_name);
                    ctx.steps.push(HandoffStep {
                        name: "module_cleanup".into(),
                        ok: true,
                        detail: Some(format!(
                            "guarded rmmod {} + tmpfile removed",
                            ctx.config.module_name
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    tracing::warn!(module = ctx.config.module_name.as_str(), error = %e,
                                       "guarded module cleanup failed (non-fatal)");
                    ctx.steps.push(HandoffStep {
                        name: "module_cleanup".into(),
                        ok: false,
                        detail: Some(format!("guarded rmmod failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
            }
        }
    }

    ctx.signal(PipelineSignal::ExitModuleCleanup);

    // ── Step 9: Restore reset capabilities ─────────────────────────────
    //
    // Re-enable default PCI reset methods so that subsequent cold resets
    // (e.g. VFIO group teardown) can issue FLR normally, and unload the
    // no_bus_reset module to re-enable SBR.
    guarded_sysfs::restore_flr(&ctx.config.bdf);
    if let Err(e) = guarded_sysfs::restore_bus_reset() {
        tracing::warn!(error = %e, "failed to unload no_bus_reset module (non-fatal)");
    }

    None
}
