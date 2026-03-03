// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selection Policy - Substrate selection strategies
//!
//! **Deep Debt**: Intelligent, configurable selection policies

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

    /// Rank all substrates by score (descending)
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
                (s.clone(), score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(ranked)
    }

    /// Select fastest substrate based on history
    fn select_fastest(
        &self,
        substrates: &[SubstrateHandle],
        history: &PerformanceHistory,
    ) -> Result<SubstrateHandle, OrchestrationError> {
        let mut best = substrates[0].clone();
        let mut best_duration = history
            .average_duration_for(best.substrate_type())
            .unwrap_or(std::time::Duration::from_secs(999));

        for substrate in substrates.iter().skip(1) {
            let duration = history
                .average_duration_for(substrate.substrate_type())
                .unwrap_or(std::time::Duration::from_secs(999));

            if duration < best_duration {
                best = substrate.clone();
                best_duration = duration;
            }
        }

        Ok(best)
    }

    /// Select most energy-efficient substrate
    fn select_most_efficient(
        &self,
        substrates: &[SubstrateHandle],
    ) -> Result<SubstrateHandle, OrchestrationError> {
        let mut best = substrates[0].clone();
        let mut best_power = best.capabilities().power_watts;

        for substrate in substrates.iter().skip(1) {
            let power = substrate.capabilities().power_watts;
            if power < best_power {
                best = substrate.clone();
                best_power = power;
            }
        }

        Ok(best)
    }

    /// Adaptive selection based on workload target
    fn select_adaptive(
        &self,
        substrates: &[SubstrateHandle],
        request: &WorkloadRequest,
        history: &PerformanceHistory,
    ) -> Result<SubstrateHandle, OrchestrationError> {
        let mut best = substrates[0].clone();
        let mut best_score = self.score_substrate(&best, request, history);

        for substrate in substrates.iter().skip(1) {
            let score = self.score_substrate(substrate, request, history);
            if score > best_score {
                best = substrate.clone();
                best_score = score;
            }
        }

        Ok(best)
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
        let history_bonus =
            if let Some(avg_duration) = history.average_duration_for(substrate.substrate_type()) {
                1.0 / (avg_duration.as_secs_f64() + 0.001)
            } else {
                1.0 // No history, neutral
            };

        // Power budget constraint
        let power_penalty = if let Some(budget) = request.power_budget_watts {
            if caps.power_watts > budget {
                0.1 // Heavy penalty for exceeding budget
            } else {
                1.0
            }
        } else {
            1.0 // No budget constraint
        };

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
    use toadstool_runtime_universal::substrate::*;
    use toadstool_runtime_universal::SubstrateError;

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
