// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use crate::vfio::guarded_sysfs;

use super::super::pipeline::PipelineContext;
use super::super::pri_recovery::recover_pri_ring;
use super::super::types::{HandoffResult, HandoffStep};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        // ── Step 6b: Rebind IOMMU siblings to vfio-pci ─────────────────

        {
            let t = Instant::now();
            tracing::info!(
                bdf = ctx.config.bdf.as_str(),
                num_siblings = ctx.sibling_state.len(),
                pipeline_elapsed_s = ctx.overall.elapsed().as_secs(),
                "catalyst profile: starting sibling rebind"
            );
            if !ctx.sibling_state.is_empty() {
                guarded_sysfs::rebind_siblings_to_vfio(&ctx.sibling_state);
            }
            let sib_ms = t.elapsed().as_millis() as u64;
            tracing::info!(
                bdf = ctx.config.bdf.as_str(),
                elapsed_ms = sib_ms,
                "catalyst profile: sibling rebind complete"
            );
            if sib_ms > 1000 {
                ctx.steps.push(HandoffStep {
                    name: "sibling_rebind".into(), ok: true,
                    detail: Some(format!("{} siblings, {}ms", ctx.sibling_state.len(), sib_ms)),
                    duration_ms: sib_ms,
                });
            }
        }

        // ── Step 6c: PRI Ring Recovery ────────────────────────────────────
        //
        // After PCI unbind, the kernel PCI framework disables PGRAPH, which
        // kills PRI ring routing to GPC/TPC/FECS/GPCCS. We re-enable PGRAPH
        // and re-enumerate PRI ring stations to restore hardware accessibility.

        if ctx.is_catalyst {
            let t = Instant::now();
            match recover_pri_ring(&ctx.config.bdf, ctx.hw.chip_name) {
                Ok(detail) => {
                    ctx.steps.push(HandoffStep {
                        name: "pri_ring_recovery".into(), ok: true,
                        detail: Some(detail),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    tracing::warn!(bdf = ctx.config.bdf.as_str(), error = %e,
                        "PRI ring recovery failed (non-fatal)");
                    ctx.steps.push(HandoffStep {
                        name: "pri_ring_recovery".into(), ok: false,
                        detail: Some(format!("recovery failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
            }
        }


    None
}
