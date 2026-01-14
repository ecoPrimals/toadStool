// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Composition Engine
//!
//! This module implements the constraint evaluation and workload composition engine.
//! It evaluates composition requests against available capabilities and determines
//! feasibility and optimal placement.
//!
//! # Philosophy
//!
//! **Discover, Don't Assume**: The engine discovers what's available at runtime
//! and evaluates constraints dynamically. No hardcoded assumptions.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::composition_engine::CompositionEngine;
//! use toadstool::composition_constraints::{Constraint, CompositionRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize engine with current runtime capabilities
//! let engine = CompositionEngine::from_runtime().await?;
//!
//! // Evaluate a gaming workload request
//! let gaming = CompositionRequest::new("gaming")
//!     .with_constraint(Constraint::requires_gpu())
//!     .with_constraint(Constraint::max_latency_ms(16));
//!
//! let evaluation = engine.evaluate(&gaming).await?;
//!
//! if evaluation.is_feasible {
//!     println!("Gaming workload is feasible! Score: {}", evaluation.overall_score);
//! } else {
//!     println!("Cannot run gaming workload");
//! }
//! # Ok(())
//! # }
//! ```

use crate::composition_constraints::*;
use crate::fractal_integration::FractalRuntime;
use crate::layer_adaptation::AdaptedCapabilities;
use crate::ToadStoolResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Composition engine
///
/// Evaluates composition requests against available capabilities.
pub struct CompositionEngine {
    /// Current runtime with layer-adapted capabilities
    runtime: Arc<FractalRuntime>,

    /// Cached capabilities (for performance)
    capabilities: AdaptedCapabilities,

    /// Engine statistics
    stats: Arc<RwLock<EngineStats>>,
}

/// Engine statistics
#[derive(Debug, Default, Clone)]
pub struct EngineStats {
    /// Total evaluations performed
    pub total_evaluations: u64,

    /// Feasible evaluations
    pub feasible_count: u64,

    /// Infeasible evaluations
    pub infeasible_count: u64,

    /// Average evaluation time (ms)
    pub avg_evaluation_ms: f64,
}

impl CompositionEngine {
    /// Create engine from current runtime
    pub async fn from_runtime() -> ToadStoolResult<Self> {
        let runtime = FractalRuntime::init().await?;
        Self::new(Arc::new(runtime))
    }

    /// Create engine with specific runtime
    pub fn new(runtime: Arc<FractalRuntime>) -> ToadStoolResult<Self> {
        let capabilities = runtime.capabilities().clone();

        Ok(Self {
            runtime,
            capabilities,
            stats: Arc::new(RwLock::new(EngineStats::default())),
        })
    }

    /// Evaluate a composition request
    ///
    /// Returns detailed evaluation showing which constraints are satisfied.
    pub async fn evaluate(
        &self,
        request: &CompositionRequest,
    ) -> ToadStoolResult<ConstraintEvaluation> {
        let start = std::time::Instant::now();

        info!("🔍 Evaluating composition request: {}", request);

        let mut results = HashMap::new();
        let mut all_hard_satisfied = true;

        // Evaluate each constraint
        for constraint in &request.constraints {
            let satisfaction = self.evaluate_constraint(constraint).await;

            debug!("  {} -> {:?}", constraint, satisfaction);

            // If it's a hard constraint and not satisfied, mark as infeasible
            if constraint.is_hard() && !satisfaction.is_satisfied() {
                all_hard_satisfied = false;
                warn!("  ❌ Hard constraint failed: {}", constraint);
            }

            results.insert(constraint.name().to_string(), satisfaction);
        }

        // Calculate overall score
        let overall_score = self.calculate_overall_score(&results, request);

        let evaluation = ConstraintEvaluation {
            request: request.clone(),
            results,
            overall_score,
            is_feasible: all_hard_satisfied,
        };

        // Update stats
        let duration_ms = start.elapsed().as_millis() as f64;
        self.update_stats(&evaluation, duration_ms).await;

        if evaluation.is_feasible {
            info!(
                "✅ Request '{}' is FEASIBLE (score: {:.2})",
                request.name, overall_score
            );
        } else {
            info!("❌ Request '{}' is INFEASIBLE", request.name);
        }

        Ok(evaluation)
    }

    /// Evaluate a single constraint against available capabilities
    async fn evaluate_constraint(&self, constraint: &Constraint) -> ConstraintSatisfaction {
        match constraint {
            Constraint::RequiresGPU => {
                if self.runtime.has_gpu_access() {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: "No GPU access available".to_string(),
                    }
                }
            }

            Constraint::PrefersGPU => {
                if self.runtime.has_direct_gpu_access() {
                    ConstraintSatisfaction::Satisfied
                } else if self.runtime.has_gpu_access() {
                    ConstraintSatisfaction::Partial(0.7) // GPU via host/cloud
                } else {
                    ConstraintSatisfaction::Partial(0.0) // No GPU
                }
            }

            Constraint::MinMemoryGB(required_gb) => {
                if let Some(available_bytes) = self.capabilities.compute.memory_bytes {
                    let available_gb = available_bytes as f64 / 1_073_741_824.0; // bytes to GB
                    if available_gb >= *required_gb {
                        ConstraintSatisfaction::Satisfied
                    } else {
                        ConstraintSatisfaction::Unsatisfied {
                            reason: format!(
                                "Insufficient memory: need {}GB, have {:.2}GB",
                                required_gb, available_gb
                            ),
                        }
                    }
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: "Memory information unavailable".to_string(),
                    }
                }
            }

            Constraint::MinCPUCores(required_cores) => {
                if let Some(available_cores) = self.capabilities.compute.cpu_cores {
                    if available_cores >= *required_cores {
                        ConstraintSatisfaction::Satisfied
                    } else {
                        ConstraintSatisfaction::Unsatisfied {
                            reason: format!(
                                "Insufficient CPU cores: need {}, have {}",
                                required_cores, available_cores
                            ),
                        }
                    }
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: "CPU information unavailable".to_string(),
                    }
                }
            }

            Constraint::MaxLatencyMs(max_ms) => {
                // Estimate latency based on deployment layer
                let estimated_latency = self.estimate_latency_ms();
                if estimated_latency <= *max_ms {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: format!(
                            "Latency too high: need <{}ms, estimated {}ms",
                            max_ms, estimated_latency
                        ),
                    }
                }
            }

            Constraint::PreferredLatencyMs(preferred_ms) => {
                let estimated_latency = self.estimate_latency_ms();
                if estimated_latency <= *preferred_ms {
                    ConstraintSatisfaction::Satisfied
                } else {
                    // Partial satisfaction based on how close we are
                    let ratio = *preferred_ms as f64 / estimated_latency as f64;
                    ConstraintSatisfaction::Partial(ratio.min(1.0))
                }
            }

            Constraint::MinBandwidthGbps(required_gbps) => {
                // Estimate bandwidth based on network capabilities
                let estimated_bandwidth = self.estimate_bandwidth_gbps();
                if estimated_bandwidth >= *required_gbps {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: format!(
                            "Insufficient bandwidth: need {}Gbps, estimated {}Gbps",
                            required_gbps, estimated_bandwidth
                        ),
                    }
                }
            }

            Constraint::PreferredBandwidthGbps(preferred_gbps) => {
                let estimated_bandwidth = self.estimate_bandwidth_gbps();
                if estimated_bandwidth >= *preferred_gbps {
                    ConstraintSatisfaction::Satisfied
                } else {
                    let ratio = estimated_bandwidth / preferred_gbps;
                    ConstraintSatisfaction::Partial(ratio.min(1.0))
                }
            }

            Constraint::RequiresCapability(cap) => {
                let has_cap = self
                    .capabilities
                    .to_capability_list()
                    .iter()
                    .any(|c| c == cap);

                if has_cap {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: format!("Required capability '{}' not available", cap),
                    }
                }
            }

            Constraint::PrefersCapability(cap) => {
                let has_cap = self
                    .capabilities
                    .to_capability_list()
                    .iter()
                    .any(|c| c == cap);

                if has_cap {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Partial(0.5)
                }
            }

            Constraint::MustBeLocal => {
                // Check if we're running in a local layer (not cloud)
                let layer_str = format!("{}", self.runtime.deployment_layer());
                if !layer_str.contains("Cloud") {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: "Running in cloud, but must be local".to_string(),
                    }
                }
            }

            Constraint::PreferLocal => {
                let layer_str = format!("{}", self.runtime.deployment_layer());
                if !layer_str.contains("Cloud") {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Partial(0.3) // Cloud is OK but not preferred
                }
            }

            Constraint::RequiresLayer(required_layer) => {
                let current_layer = format!("{}", self.runtime.deployment_layer());
                if current_layer.contains(required_layer) {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: format!(
                            "Wrong layer: need '{}', have '{}'",
                            required_layer, current_layer
                        ),
                    }
                }
            }

            Constraint::PrefersLayer(preferred_layer) => {
                let current_layer = format!("{}", self.runtime.deployment_layer());
                if current_layer.contains(preferred_layer) {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Partial(0.5)
                }
            }

            Constraint::RequiresPersistentStorage => {
                // Check storage capabilities
                use crate::layer_adaptation::StorageType;
                let has_persistent = !matches!(
                    self.capabilities.storage.storage_type,
                    StorageType::HostFilesystem
                );

                if has_persistent {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: "No persistent storage available".to_string(),
                    }
                }
            }

            Constraint::MaxCostPerHour(max_cost) => {
                // Estimate cost based on layer
                let estimated_cost = self.estimate_cost_per_hour();
                if estimated_cost <= *max_cost {
                    ConstraintSatisfaction::Satisfied
                } else {
                    ConstraintSatisfaction::Unsatisfied {
                        reason: format!(
                            "Too expensive: need <${}/hr, estimated ${}/hr",
                            max_cost, estimated_cost
                        ),
                    }
                }
            }

            Constraint::MinimizeCost => {
                // Always partial satisfaction based on cost
                let cost = self.estimate_cost_per_hour();
                let score = 1.0 / (1.0 + cost); // Lower cost = higher score
                ConstraintSatisfaction::Partial(score)
            }

            Constraint::Custom { name, hard, value } => {
                debug!(
                    "Custom constraint '{}' = '{}' (hard: {})",
                    name, value, hard
                );
                // For now, assume custom constraints are satisfied
                // In a real implementation, this would call a plugin system
                ConstraintSatisfaction::Satisfied
            }
        }
    }

    /// Estimate latency based on deployment layer
    fn estimate_latency_ms(&self) -> u64 {
        let layer_str = format!("{}", self.runtime.deployment_layer());

        if layer_str.contains("BareMetalOS") {
            1 // Bare metal: very low latency
        } else if layer_str.contains("Container") {
            5 // Container: slight overhead
        } else if layer_str.contains("VM") {
            10 // VM: more overhead
        } else if layer_str.contains("Cloud") {
            50 // Cloud: network latency
        } else {
            20 // Default
        }
    }

    /// Estimate bandwidth based on network capabilities
    fn estimate_bandwidth_gbps(&self) -> f64 {
        use crate::layer_adaptation::NetworkAccess;

        match self.capabilities.network.network_access {
            NetworkAccess::Direct => 100.0,       // High-speed local network
            NetworkAccess::HostNamespace => 40.0, // Container networking
            NetworkAccess::CloudVPC => 10.0,      // Cloud network
        }
    }

    /// Estimate cost per hour based on deployment layer
    fn estimate_cost_per_hour(&self) -> f64 {
        let layer_str = format!("{}", self.runtime.deployment_layer());

        if layer_str.contains("BareMetalOS") || layer_str.contains("Middleware") {
            0.0 // Local is free
        } else if layer_str.contains("Container") {
            0.01 // Container overhead minimal
        } else if layer_str.contains("VM") {
            0.10 // VM has some cost
        } else if layer_str.contains("Cloud") {
            // Cloud cost depends on GPU
            if self.runtime.has_gpu_access() {
                5.00 // GPU instance expensive
            } else {
                0.50 // CPU instance cheaper
            }
        } else {
            0.10 // Default
        }
    }

    /// Calculate overall satisfaction score
    fn calculate_overall_score(
        &self,
        results: &HashMap<String, ConstraintSatisfaction>,
        request: &CompositionRequest,
    ) -> f64 {
        if results.is_empty() {
            return 1.0;
        }

        // Hard constraints: must all be satisfied (0.0 if any fail)
        let hard_score = {
            let hard_results: Vec<_> = request
                .hard_constraints()
                .iter()
                .filter_map(|c| results.get(c.name()))
                .collect();

            if hard_results.is_empty() {
                1.0
            } else if hard_results.iter().all(|s| s.is_satisfied()) {
                1.0
            } else {
                0.0 // Any hard constraint failure = 0
            }
        };

        // EVOLUTION FIX: If any hard constraint fails, overall score is 0.0
        // regardless of soft constraints. The workload is infeasible.
        if hard_score == 0.0 {
            return 0.0;
        }

        // Soft constraints: average satisfaction
        let soft_score = {
            let soft_results: Vec<_> = request
                .soft_constraints()
                .iter()
                .filter_map(|c| results.get(c.name()))
                .collect();

            if soft_results.is_empty() {
                1.0
            } else {
                let total: f64 = soft_results.iter().map(|s| s.score()).sum();
                total / soft_results.len() as f64
            }
        };

        // Overall: 70% weight on hard constraints, 30% on soft
        // (only reached if all hard constraints pass)
        (hard_score * 0.7) + (soft_score * 0.3)
    }

    /// Update engine statistics
    async fn update_stats(&self, evaluation: &ConstraintEvaluation, duration_ms: f64) {
        let mut stats = self.stats.write().await;

        stats.total_evaluations += 1;

        if evaluation.is_feasible {
            stats.feasible_count += 1;
        } else {
            stats.infeasible_count += 1;
        }

        // Update running average
        let total = stats.total_evaluations as f64;
        stats.avg_evaluation_ms = ((stats.avg_evaluation_ms * (total - 1.0)) + duration_ms) / total;
    }

    /// Get engine statistics
    pub async fn stats(&self) -> EngineStats {
        self.stats.read().await.clone()
    }

    /// Get current capabilities
    pub fn capabilities(&self) -> &AdaptedCapabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_initialization() {
        let result = CompositionEngine::from_runtime().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_gpu_constraint_evaluation() {
        let engine = CompositionEngine::from_runtime().await.unwrap();

        let request =
            CompositionRequest::new("test_gpu").with_constraint(Constraint::requires_gpu());

        let eval = engine.evaluate(&request).await.unwrap();

        // Should have evaluation for requires_gpu constraint
        assert!(eval.results.contains_key("requires_gpu"));

        // Feasibility depends on whether GPU is available
        let has_gpu = engine.runtime.has_gpu_access();
        assert_eq!(eval.is_feasible, has_gpu);
    }

    #[tokio::test]
    async fn test_soft_constraint_evaluation() {
        let engine = CompositionEngine::from_runtime().await.unwrap();

        let request = CompositionRequest::new("test_soft")
            .with_constraint(Constraint::prefers_gpu())
            .with_constraint(Constraint::prefer_local());

        let eval = engine.evaluate(&request).await.unwrap();

        // Soft constraints should always result in feasible (no hard constraints)
        assert!(eval.is_feasible);

        // Should have partial scores for soft constraints
        assert!(eval.results.contains_key("prefers_gpu"));
        assert!(eval.results.contains_key("prefer_local"));
    }

    #[tokio::test]
    async fn test_memory_constraint() {
        let engine = CompositionEngine::from_runtime().await.unwrap();

        // Request very small memory (should always succeed)
        let request =
            CompositionRequest::new("test_memory").with_constraint(Constraint::min_memory_gb(0.1));

        let eval = engine.evaluate(&request).await.unwrap();
        assert!(eval.is_feasible);
    }

    #[tokio::test]
    async fn test_multiple_constraints() {
        let engine = CompositionEngine::from_runtime().await.unwrap();

        let request = CompositionRequest::new("test_multi")
            .with_constraint(Constraint::min_memory_gb(0.1))
            .with_constraint(Constraint::min_cpu_cores(1))
            .with_constraint(Constraint::prefer_local());

        let eval = engine.evaluate(&request).await.unwrap();

        // Should evaluate all constraints
        assert_eq!(eval.results.len(), 3);

        // Hard constraints (memory, CPU) should be satisfied
        assert!(eval.results.get("min_memory_gb").unwrap().is_satisfied());
        assert!(eval.results.get("min_cpu_cores").unwrap().is_satisfied());
    }

    #[tokio::test]
    async fn test_engine_stats() {
        let engine = CompositionEngine::from_runtime().await.unwrap();

        let initial_stats = engine.stats().await;
        assert_eq!(initial_stats.total_evaluations, 0);

        // Run some evaluations
        let request = CompositionRequest::new("test");
        engine.evaluate(&request).await.unwrap();

        let updated_stats = engine.stats().await;
        assert_eq!(updated_stats.total_evaluations, 1);
    }
}
