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
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "precision loss and truncation acceptable for this conversion"
    )]
    let effective_savings = total_time_savings.min((current_duration_secs as f32 * 0.8) as u64);
    let optimized_duration_secs = current_duration_secs.saturating_sub(effective_savings);
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )]
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
            #[expect(
                clippy::cast_precision_loss,
                reason = "precision loss acceptable for this conversion"
            )]
            let priority = o.benefit * (o.time_savings_secs as f32 / 60.0);
            (id, priority)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.into_iter().map(|(id, _)| id).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use crate::resource_estimator::ResourceEstimate;

    use super::{estimate_improvement, rank_by_priority};
    use crate::resource_optimizer::types::{Opportunity, OpportunityType};

    fn sample_estimate(duration_secs: u64) -> ResourceEstimate {
        ResourceEstimate {
            graph_id: "g".into(),
            cpu_cores: 8,
            memory_bytes: 16 * 1024 * 1024 * 1024,
            gpu_memory_bytes: 4 * 1024 * 1024 * 1024,
            storage_bytes: 1024,
            network_bandwidth_mbps: 100,
            estimated_duration: Duration::from_secs(duration_secs),
            max_parallelism: 4,
            critical_path_length: 3,
            node_estimates: HashMap::new(),
            warnings: vec![],
        }
    }

    #[test]
    fn estimate_improvement_empty_opportunities_leaves_duration_and_speedup_neutral() {
        let est = sample_estimate(100);
        let improved = estimate_improvement(&est, &[]);
        assert_eq!(improved.current_duration_secs, 100);
        assert_eq!(improved.optimized_duration_secs, 100);
        assert_eq!(improved.time_savings_secs, 0);
        assert_eq!(improved.speedup_factor, 1.0);
    }

    #[test]
    fn estimate_improvement_single_opportunity_reduces_duration() {
        let est = sample_estimate(100);
        let opps = vec![Opportunity {
            opportunity_type: OpportunityType::Parallelization,
            affected_nodes: vec!["n1".into()],
            benefit: 0.5,
            description: "p".into(),
            recommendation: "r".into(),
            time_savings_secs: 25,
            resource_savings: HashMap::new(),
        }];
        let improved = estimate_improvement(&est, &opps);
        assert_eq!(improved.time_savings_secs, 25);
        assert_eq!(improved.optimized_duration_secs, 75);
        assert!(improved.speedup_factor > 1.0);
    }

    #[test]
    fn estimate_improvement_aggregates_weighted_resource_savings_across_opportunities() {
        let est = sample_estimate(60);
        let mut savings_a = HashMap::new();
        savings_a.insert("memory_bytes".to_string(), 1024);
        savings_a.insert("gpu_memory_bytes".to_string(), 512);
        let mut savings_b = HashMap::new();
        savings_b.insert("memory_bytes".to_string(), 2048);
        let opps = vec![
            Opportunity {
                opportunity_type: OpportunityType::MemoryStreaming,
                affected_nodes: vec!["a".into()],
                benefit: 0.6,
                description: "m".into(),
                recommendation: "r".into(),
                time_savings_secs: 10,
                resource_savings: savings_a,
            },
            Opportunity {
                opportunity_type: OpportunityType::Caching,
                affected_nodes: vec!["b".into()],
                benefit: 0.4,
                description: "c".into(),
                recommendation: "r".into(),
                time_savings_secs: 5,
                resource_savings: savings_b,
            },
        ];
        let improved = estimate_improvement(&est, &opps);
        assert_eq!(
            improved.optimized_resources.get("memory_bytes").copied(),
            Some(est.memory_bytes.saturating_sub(3072))
        );
        assert_eq!(
            improved
                .optimized_resources
                .get("gpu_memory_bytes")
                .copied(),
            Some(est.gpu_memory_bytes.saturating_sub(512))
        );
    }

    #[test]
    fn estimate_improvement_caps_time_savings_at_eighty_percent_of_current_duration() {
        let est = sample_estimate(100);
        let opps = vec![Opportunity {
            opportunity_type: OpportunityType::GpuAcceleration,
            affected_nodes: vec!["x".into()],
            benefit: 1.0,
            description: "g".into(),
            recommendation: "r".into(),
            time_savings_secs: 1_000,
            resource_savings: HashMap::new(),
        }];
        let improved = estimate_improvement(&est, &opps);
        assert_eq!(improved.time_savings_secs, 80);
        assert_eq!(improved.optimized_duration_secs, 20);
    }

    #[test]
    fn rank_by_priority_orders_by_benefit_times_normalized_time_savings() {
        let opps = vec![
            Opportunity {
                opportunity_type: OpportunityType::Caching,
                affected_nodes: vec!["low".into()],
                benefit: 1.0,
                description: String::new(),
                recommendation: String::new(),
                time_savings_secs: 60,
                resource_savings: HashMap::new(),
            },
            Opportunity {
                opportunity_type: OpportunityType::GpuAcceleration,
                affected_nodes: vec!["high".into()],
                benefit: 2.0,
                description: String::new(),
                recommendation: String::new(),
                time_savings_secs: 60,
                resource_savings: HashMap::new(),
            },
        ];
        let ranked = rank_by_priority(&opps);
        assert_eq!(ranked.len(), 2);
        assert!(
            ranked[0].contains("GpuAcceleration"),
            "expected higher priority first, got {ranked:?}"
        );
    }
}
