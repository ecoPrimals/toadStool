// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use crate::vfio::module_patch;
use crate::vfio::sovereign_tiers::classify_tier;

use super::super::pipeline::PipelineContext;
use super::super::types::{HandoffResult, HandoffStep, ModuleSourceConfig};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        // ── Step 7: Tier Classification ─────────────────────────────────

        ctx.tier = if ctx.is_catalyst {
            if let Some(ref ct) = ctx.catalyst_tier {
                // Use the early tier classification captured with warm BAR0
                // (before PRI ring recovery destroyed PRI routing).
                let t = Instant::now();
                ctx.steps.push(HandoffStep {
                    name: "tier_classify".into(), ok: true,
                    detail: Some(format!("{} (warm BAR0, pre-PRI-recovery)", ct.tier)),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                ctx.catalyst_tier.take()
            } else {
                None
            }
        } else {
            None
        };

        if ctx.tier.is_none() {
            ctx.tier = if let Some(b) = ctx.bar0 {
            let t = Instant::now();
            let evidence = classify_tier(b);
            ctx.steps.push(HandoffStep {
                name: "tier_classify".into(), ok: true,
                detail: Some(format!("{}", evidence.tier)),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            Some(evidence)
        } else {
            let t = Instant::now();
            match crate::vfio::device::MappedBar::from_sysfs_rw(&ctx.config.bdf, 16 * 1024 * 1024) {
                Ok(sysfs_bar) => {
                    let evidence = classify_tier(&sysfs_bar);
                    ctx.steps.push(HandoffStep {
                        name: "tier_classify".into(), ok: true,
                        detail: Some(format!("{} (via sysfs)", evidence.tier)),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    Some(evidence)
                }
                Err(e) => {
                    ctx.steps.push(HandoffStep {
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

        if ctx.is_catalyst
            && let Some(ref pr) = ctx.patch_result
        {
                let t = Instant::now();
                let frozen_dir = crate::linux_paths::data_subdir("catalysts/frozen");
                let _ = std::fs::create_dir_all(&frozen_dir);
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
                        ctx.steps.push(HandoffStep {
                            name: "catalyst_preserve".into(), ok: true,
                            detail: Some(format!("frozen .ko: {} ({bytes} bytes)", frozen_dest)),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(err = %e, "catalyst preserve: failed to archive frozen .ko");
                        ctx.steps.push(HandoffStep {
                            name: "catalyst_preserve".into(), ok: false,
                            detail: Some(format!("frozen .ko copy failed: {e}")),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                    }
                }

                // Persist recipe JSON (PatchSet serialization)
                let recipe_dir = crate::linux_paths::data_subdir("catalysts/recipes");
                let _ = std::fs::create_dir_all(&recipe_dir);
                let patch_set_name = match &ctx.config.module_source {
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


    None
}
