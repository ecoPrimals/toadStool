// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use crate::vfio::guarded_sysfs;
use crate::vfio::module_patch;

use super::super::pipeline::{PipelineContext, PipelineSignal};
use super::super::types::{HandoffResult, HandoffStep};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        // ── Step 8: Module Cleanup (GUARDED) ────────────────────────────

        ctx.module_unloaded = false;
        if ctx.module_loaded {
            let t = Instant::now();
            match guarded_sysfs::rmmod_guarded(&ctx.config.module_name, guarded_sysfs::RMMOD_TIMEOUT) {
                Ok(()) => {
                    ctx.module_unloaded = true;
                    let _ = module_patch::cleanup_patched_module(&ctx.config.module_name);
                    ctx.steps.push(HandoffStep {
                        name: "module_cleanup".into(), ok: true,
                        detail: Some(format!("guarded rmmod {} + tmpfile removed", ctx.config.module_name)),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    tracing::warn!(module = ctx.config.module_name.as_str(), error = %e,
                                   "guarded module cleanup failed (non-fatal)");
                    ctx.steps.push(HandoffStep {
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
        guarded_sysfs::restore_flr(&ctx.config.bdf);
        if let Err(e) = guarded_sysfs::restore_bus_reset() {
            tracing::warn!(error = %e, "failed to unload no_bus_reset module (non-fatal)");
        }


    None
}
