// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cost/benefit analysis for optimizations

use std::collections::HashMap;

use crate::resource_estimator::ResourceEstimate;

use super::types::{ImprovementEstimate, Opportunity};

/// Estimate improvement from applying optimizations
#[must_use]
pub fn estimate_improvement(
    estimate: &ResourceEstimate,
    opportunities: &[Opportunity],
) -> ImprovementEstimate {
    let current_duration_secs = estimate.estimated_duration.as_secs();
    let total_time_savings: u64 = opportunities.iter().map(|o| o.time_savings_secs).sum();
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let effective_savings = total_time_savings.min((current_duration_secs as f32 * 0.8) as u64);
    let optimized_duration_secs = current_duration_secs.saturating_sub(effective_savings);
    #[allow(clippy::cast_precision_loss)]
    let speedup_factor = if optimized_duration_secs > 0 {
        current_duration_secs as f32 / optimized_duration_secs as f32
    } else {
        1.0
    };

    let mut total_resource_savings: HashMap<String, u64> = HashMap::new();
    for opportunity in opportunities {
        for (resource, savings) in &opportunity.resource_savings {
            *total_resource_savings.entry(resource.clone()).or_insert(0) += savings;
        }
    }

    let mut current_resources = HashMap::new();
    current_resources.insert("cpu_cores".to_string(), u64::from(estimate.cpu_cores));
    current_resources.insert("memory_bytes".to_string(), estimate.memory_bytes);
    current_resources.insert("gpu_memory_bytes".to_string(), estimate.gpu_memory_bytes);

    let mut optimized_resources = current_resources.clone();
    for (resource, savings) in &total_resource_savings {
        if let Some(current) = optimized_resources.get_mut(resource) {
            *current = current.saturating_sub(*savings);
        }
    }

    ImprovementEstimate {
        current_duration_secs,
        optimized_duration_secs,
        time_savings_secs: effective_savings,
        speedup_factor,
        current_resources,
        optimized_resources,
    }
}

/// Rank opportunities by priority (benefit * time saved)
#[must_use]
pub fn rank_by_priority(opportunities: &[Opportunity]) -> Vec<String> {
    let mut ranked: Vec<(String, f32)> = opportunities
        .iter()
        .map(|o| {
            let id = format!("{:?}-{}", o.opportunity_type, o.affected_nodes.join(","));
            #[allow(clippy::cast_precision_loss)]
            let priority = o.benefit * (o.time_savings_secs as f32 / 60.0);
            (id, priority)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.into_iter().map(|(id, _)| id).collect()
}
