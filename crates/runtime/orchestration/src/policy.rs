// SPDX-License-Identifier: AGPL-3.0-only
//! Selection Policy - Substrate selection strategies
//!
//! **Deep Debt**: Intelligent, configurable selection policies

use std::sync::Arc;

use crate::error::OrchestrationError;
use crate::orchestrator::*;

/// Selection policy for choosing substrates
///
/// **Deep Debt**: Configurable strategy, not hardcoded
#[derive(Debug, Clone, Default)]
pub enum SelectionPolicy {
    /// Always select fastest substrate
    Fastest,

    /// Select most energy-efficient substrate
    MostEfficient,

    /// Select based on workload target
    #[default]
    Adaptive,

    /// Round-robin across substrates
    RoundRobin { next: usize },

    /// Custom scoring function
    Custom,
}

impl SelectionPolicy {
    /// Select optimal substrate for workload
    pub fn select(
        &self,
        substrates: &[SubstrateHandle],
        request: &WorkloadRequest,
        history: &PerformanceHistory,
    ) -> Result<SubstrateHandle, OrchestrationError> {
        if substrates.is_empty() {
            return Err(OrchestrationError::NoSubstrates);
        }

        match self {
            Self::Fastest => self.select_fastest(substrates, history),
            Self::MostEfficient => self.select_most_efficient(substrates),
            Self::Adaptive => self.select_adaptive(substrates, request, history),
            Self::RoundRobin { next } => {
                let idx = *next % substrates.len();
                Ok(substrates[idx].clone())
            }
            Self::Custom => self.select_adaptive(substrates, request, history),
        }
    }

    /// Rank all substrates by score (descending).
    ///
    /// Uses `Arc::clone` rather than deep clone — cheap refcount bump.
    pub fn rank_all(
        &self,
        substrates: &[SubstrateHandle],
        request: &WorkloadRequest,
        history: &PerformanceHistory,
    ) -> Result<Vec<(SubstrateHandle, f64)>, OrchestrationError> {
        let mut ranked: Vec<_> = substrates
            .iter()
            .map(|s| {
                let score = self.score_substrate(s, request, history);
                (Arc::clone(s), score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(ranked)
    }

    /// Select fastest substrate based on history.
    ///
    /// Tracks index during comparison to avoid cloning every candidate;
    /// only the winning `Arc` is cloned once at the end.
    fn select_fastest(
        &self,
        substrates: &[SubstrateHandle],
        history: &PerformanceHistory,
    ) -> Result<SubstrateHandle, OrchestrationError> {
        let no_history = std::time::Duration::from_secs(999);
        let mut best_idx = 0;
        let mut best_duration = history
            .average_duration_for(substrates[0].substrate_type())
            .unwrap_or(no_history);

        for (i, substrate) in substrates.iter().enumerate().skip(1) {
            let duration = history
                .average_duration_for(substrate.substrate_type())
                .unwrap_or(no_history);

            if duration < best_duration {
                best_idx = i;
                best_duration = duration;
            }
        }

        Ok(Arc::clone(&substrates[best_idx]))
    }

    /// Select most energy-efficient substrate.
    ///
    /// Single `Arc::clone` at the end instead of per-candidate.
    fn select_most_efficient(
        &self,
        substrates: &[SubstrateHandle],
    ) -> Result<SubstrateHandle, OrchestrationError> {
        let mut best_idx = 0;
        let mut best_power = substrates[0].capabilities().power_watts;

        for (i, substrate) in substrates.iter().enumerate().skip(1) {
            let power = substrate.capabilities().power_watts;
            if power < best_power {
                best_idx = i;
                best_power = power;
            }
        }

        Ok(Arc::clone(&substrates[best_idx]))
    }

    /// Adaptive selection based on workload target.
    ///
    /// Single `Arc::clone` at the end instead of per-candidate.
    fn select_adaptive(
        &self,
        substrates: &[SubstrateHandle],
        request: &WorkloadRequest,
        history: &PerformanceHistory,
    ) -> Result<SubstrateHandle, OrchestrationError> {
        let mut best_idx = 0;
        let mut best_score = self.score_substrate(&substrates[0], request, history);

        for (i, substrate) in substrates.iter().enumerate().skip(1) {
            let score = self.score_substrate(substrate, request, history);
            if score > best_score {
                best_idx = i;
                best_score = score;
            }
        }

        Ok(Arc::clone(&substrates[best_idx]))
    }

    /// Score a substrate for a workload (higher is better)
    fn score_substrate(
        &self,
        substrate: &SubstrateHandle,
        request: &WorkloadRequest,
        history: &PerformanceHistory,
    ) -> f64 {
        let caps = substrate.capabilities();

        // Base scores
        let throughput_score = caps.throughput_ops_per_sec / 1e12; // Normalize
        let latency_score = 1000.0 / (caps.latency_ms + 1.0);
        let energy_score = 1000.0 / (caps.power_watts + 1.0);

        // Historical performance bonus
        let history_bonus = history
            .average_duration_for(substrate.substrate_type())
            .map_or(1.0, |avg_duration| {
                1.0 / (avg_duration.as_secs_f64() + 0.001)
            });

        // Power budget constraint
        let power_penalty = request.power_budget_watts.map_or(1.0, |budget| {
            if caps.power_watts > budget {
                0.1 // Heavy penalty for exceeding budget
            } else {
                1.0
            }
        });

        // Weight based on target
        let score = match request.target {
            PerformanceTarget::Latency => {
                latency_score * 0.7 + throughput_score * 0.2 + energy_score * 0.1
            }
            PerformanceTarget::Throughput => {
                throughput_score * 0.7 + latency_score * 0.2 + energy_score * 0.1
            }
            PerformanceTarget::Energy => {
                energy_score * 0.7 + throughput_score * 0.2 + latency_score * 0.1
            }
            PerformanceTarget::Balanced => {
                throughput_score * 0.4 + latency_score * 0.3 + energy_score * 0.3
            }
        };

        score * history_bonus * power_penalty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use toadstool_runtime_universal::SubstrateError;
    use toadstool_runtime_universal::substrate::*;

    struct MockSubstrate {
        name: String,
        substrate_type: SubstrateType,
        power: f64,
    }

    // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
    #[async_trait]
    impl ComputeSubstrate for MockSubstrate {
        fn name(&self) -> &str {
            &self.name
        }

        fn substrate_type(&self) -> SubstrateType {
            self.substrate_type
        }

        fn capabilities(&self) -> SubstrateCapabilities {
            let mut caps = SubstrateCapabilities::default_for_type(self.substrate_type);
            caps.power_watts = self.power;
            caps
        }

        async fn execute_buffer_op(
            &self,
            _op: BufferOperation,
        ) -> Result<BufferOutput, SubstrateError> {
            Ok(BufferOutput::default())
        }
    }

    #[test]
    fn test_adaptive_policy() {
        let policy = SelectionPolicy::Adaptive;

        let substrates: Vec<SubstrateHandle> = vec![
            Arc::new(MockSubstrate {
                name: "CPU".to_string(),
                substrate_type: SubstrateType::Cpu,
                power: 65.0,
            }),
            Arc::new(MockSubstrate {
                name: "GPU".to_string(),
                substrate_type: SubstrateType::Gpu,
                power: 250.0,
            }),
            Arc::new(MockSubstrate {
                name: "NPU".to_string(),
                substrate_type: SubstrateType::Npu,
                power: 2.0,
            }),
        ];

        let history = PerformanceHistory::new();

        // Energy target should prefer NPU (lowest power)
        let request = WorkloadRequest {
            operation_count: 1000,
            power_budget_watts: None,
            target: PerformanceTarget::Energy,
            batch_size: None,
        };

        let selected = policy.select(&substrates, &request, &history).unwrap();
        assert_eq!(selected.substrate_type(), SubstrateType::Npu);
    }

    #[test]
    fn test_power_budget_constraint() {
        let policy = SelectionPolicy::Adaptive;

        let substrates: Vec<SubstrateHandle> = vec![
            Arc::new(MockSubstrate {
                name: "GPU".to_string(),
                substrate_type: SubstrateType::Gpu,
                power: 250.0,
            }),
            Arc::new(MockSubstrate {
                name: "NPU".to_string(),
                substrate_type: SubstrateType::Npu,
                power: 2.0,
            }),
        ];

        let history = PerformanceHistory::new();

        // Power budget of 50W should exclude GPU
        let request = WorkloadRequest {
            operation_count: 1000,
            power_budget_watts: Some(50.0),
            target: PerformanceTarget::Throughput,
            batch_size: None,
        };

        let selected = policy.select(&substrates, &request, &history).unwrap();
        // NPU should be selected despite throughput target, due to power constraint
        assert_eq!(selected.substrate_type(), SubstrateType::Npu);
    }
}
