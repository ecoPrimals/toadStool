// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use super::super::pipeline::PipelineContext;
use super::super::rm_trigger::trigger_rm_init;
use super::super::types::{HandoffResult, HandoffStep};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        // ── Step 3b: Catalyst RM trigger — open chardev to start GPU init ──
        //
        // nvidia RM defers GPU initialization to userspace open. With the
        // catalyst PatchByteAt(0x7b) patch, __register_chrdev uses major 0
        // (dynamic allocation). We find the assigned major, create a device
        // node, and open it to trigger rm_init_adapter → full GPU init
        // (SEC2 → ACR → FECS → GPCCS → TPC PRI station creation).
        // The chardev name is "nvidia-frontend" (from .rodata), not the renamed
        // module name. trigger_rm_init searches for "nvidia-frontend" entries.
        if ctx.is_catalyst && ctx.module_loaded {
            let t = Instant::now();
            // Exp 229: pass create_channel=true for catalyst strategies to
            // establish a full RM compute channel before warm swap.
            match trigger_rm_init(&ctx.config.module_name, /* create_channel */ true, &ctx.config.bdf, &ctx.hw.interrupt_profile) {
                Ok(result) => {
                    tracing::info!(bdf = ctx.config.bdf.as_str(), summary = result.summary.as_str(),
                        channel_evidence = ?result.channel_evidence,
                        "catalyst RM init triggered");
                    ctx.rm_channel_evidence = result.channel_evidence;
                    ctx.steps.push(HandoffStep {
                        name: "rm_trigger".into(), ok: true,
                        detail: Some(result.summary),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    tracing::warn!(bdf = ctx.config.bdf.as_str(), error = %e,
                        "catalyst RM trigger failed — RM may not initialize GPU");
                    ctx.steps.push(HandoffStep {
                        name: "rm_trigger".into(), ok: false,
                        detail: Some(format!("RM trigger failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
            }
        }


    None
}
