// SPDX-License-Identifier: AGPL-3.0-or-later
//! Optimization recommendation generation.
//!
//! Analyzes runtime statistics and produces actionable recommendations.

use std::collections::HashMap;
use std::time::SystemTime;

use toadstool::execution::RuntimeType;

use crate::types::{
    OptimizationRecommendation, PerformanceConfig, RecommendationType, RuntimeStats,
};

/// Generate optimization recommendations from runtime statistics.
pub(super) fn generate_recommendations(
    config: &PerformanceConfig,
    stats: &HashMap<RuntimeType, RuntimeStats>,
) -> Vec<OptimizationRecommendation> {
    if !config.enable_recommendations {
        return vec![];
    }

    let mut recs = Vec::new();
    let now = SystemTime::now();

    for (rt, rs) in stats.iter() {
        if rs.total_executions < config.min_prediction_samples as u64 {
            continue;
        }

        if rs.success_rate < 90.0 {
            recs.push(OptimizationRecommendation {
                id: format!("low-success-{rt:?}"),
                recommendation_type: RecommendationType::RuntimeSwitch,
                priority: if rs.success_rate < 70.0 { 9 } else { 6 },
                expected_improvement: 100.0 - rs.success_rate,
                description: format!(
                    "{rt:?} success rate is {:.1}% — consider alternate runtime",
                    rs.success_rate
                ),
                actions: vec![
                    format!("Investigate failures for {rt:?}"),
                    "Route workloads to higher-reliability runtime".into(),
                ],
                timestamp: now,
            });
        }

        if rs.avg_memory_usage > config.target_utilization_percent * 10.0 {
            recs.push(OptimizationRecommendation {
                id: format!("high-mem-{rt:?}"),
                recommendation_type: RecommendationType::ResourceIncrease,
                priority: 5,
                expected_improvement: 10.0,
                description: format!(
                    "{rt:?} avg memory {:.0} MB exceeds target utilization",
                    rs.avg_memory_usage
                ),
                actions: vec![
                    "Increase memory allocation or enable swap".into(),
                    "Profile workload for memory leaks".into(),
                ],
                timestamp: now,
            });
        }

        if rs.avg_cpu_usage < 20.0 && rs.avg_memory_usage < 100.0 {
            recs.push(OptimizationRecommendation {
                id: format!("low-util-{rt:?}"),
                recommendation_type: RecommendationType::ResourceDecrease,
                priority: 3,
                expected_improvement: 5.0,
                description: format!(
                    "{rt:?} underutilized (CPU {:.1}%, mem {:.0} MB)",
                    rs.avg_cpu_usage, rs.avg_memory_usage
                ),
                actions: vec!["Reduce reserved resources for this runtime".into()],
                timestamp: now,
            });
        }

        if rs.efficiency_score < 30.0 && rs.total_executions >= 20 {
            recs.push(OptimizationRecommendation {
                id: format!("low-eff-{rt:?}"),
                recommendation_type: RecommendationType::ConfigurationAdjustment,
                priority: 7,
                expected_improvement: 20.0,
                description: format!(
                    "{rt:?} efficiency score {:.1} — tune workgroup sizes or batching",
                    rs.efficiency_score
                ),
                actions: vec![
                    "Review dispatch configuration".into(),
                    "Enable batching for small workloads".into(),
                ],
                timestamp: now,
            });
        }
    }

    recs.sort_by(|a, b| b.priority.cmp(&a.priority));
    recs
}
