// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use super::super::pipeline::PipelineContext;
use super::super::rm_trigger::trigger_rm_init;
use super::super::types::{HandoffResult, HandoffStep};

fn breadcrumb(msg: &str) {
    crate::vfio::sovereign_handoff::forensics::breadcrumb(msg);
}

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        if ctx.is_catalyst && ctx.module_loaded {
            breadcrumb("rm_trigger: spawning rm_trigger binary");
            let t = Instant::now();
            match trigger_rm_init(&ctx.config.module_name, /* create_channel */ true, &ctx.config.bdf, &ctx.hw.interrupt_profile) {
                Ok(result) => {
                    breadcrumb(&format!("rm_trigger: completed OK in {}ms", t.elapsed().as_millis()));
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
