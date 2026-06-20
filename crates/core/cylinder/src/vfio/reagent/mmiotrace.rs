// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mmiotrace → reagent recipe distillation.
//!
//! Parses an mmiotrace log through [`BootTrace`], extracts the write sequence,
//! filters to falcon/ACR/GR domains, and saves as a JSON recipe file.
//! This captures the exact register programming that nvidia uses to
//! boot FECS/GPCCS — the chemical agents for Tier 2.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ReagentError;

/// Distill an mmiotrace log into an ACR-focused reagent recipe.
pub fn distill_mmiotrace_to_reagent(
    trace_path: &Path,
    output_path: &Path,
) -> Result<MmiotraceReagentSummary, ReagentError> {
    use crate::vfio::channel::diagnostic::boot_follower::BootTrace;

    let trace = BootTrace::from_mmiotrace(trace_path)?;

    let total_writes = trace.writes.len();
    let total_reads = trace.reads.len();
    let domain_summary = trace.domain_summary();

    let recipe = trace.to_recipe();
    let recipe_steps = recipe.len();

    let acr_domains = ["PMC", "PRI_MASTER", "PMU", "PFIFO", "PBDMA", "PRAMIN"];
    let acr_recipe: Vec<_> = recipe
        .iter()
        .filter(|s| acr_domains.contains(&s.domain.as_str()) || s.domain == "UNKNOWN")
        .cloned()
        .collect();
    let acr_steps = acr_recipe.len();

    let json = serde_json::to_string_pretty(&recipe).map_err(ReagentError::Serialize)?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ReagentError::MirrorMkdirFailed {
            path: parent.display().to_string(),
            source,
        })?;
    }
    std::fs::write(output_path, &json).map_err(ReagentError::WriteRecipe)?;

    let acr_output = output_path.with_file_name(
        output_path
            .file_stem()
            .map(|s| format!("{}_acr_subset", s.to_string_lossy()))
            .unwrap_or_else(|| "acr_subset".to_owned())
            + ".json",
    );
    let acr_json = serde_json::to_string_pretty(&acr_recipe).map_err(ReagentError::Serialize)?;
    std::fs::write(&acr_output, &acr_json).map_err(ReagentError::WriteRecipe)?;

    let summary = MmiotraceReagentSummary {
        trace_path: trace_path.to_path_buf(),
        total_writes,
        total_reads,
        duration_us: trace.duration_us,
        domain_summary,
        recipe_steps,
        acr_steps,
        output_path: output_path.to_path_buf(),
        acr_output_path: acr_output,
    };

    tracing::info!(
        trace = %trace_path.display(),
        writes = total_writes,
        reads = total_reads,
        recipe_steps = recipe_steps,
        acr_steps = acr_steps,
        "mmiotrace distilled to reagent recipe"
    );

    Ok(summary)
}

/// Summary of an mmiotrace distillation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmiotraceReagentSummary {
    pub trace_path: PathBuf,
    pub total_writes: usize,
    pub total_reads: usize,
    pub duration_us: u64,
    pub domain_summary: std::collections::BTreeMap<String, usize>,
    pub recipe_steps: usize,
    pub acr_steps: usize,
    pub output_path: PathBuf,
    pub acr_output_path: PathBuf,
}
