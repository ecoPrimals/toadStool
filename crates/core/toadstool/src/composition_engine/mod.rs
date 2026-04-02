// SPDX-License-Identifier: AGPL-3.0-only

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

mod estimators;
mod evaluators;

use crate::ToadStoolResult;
use crate::composition_constraints::{
    CompositionRequest, ConstraintEvaluation, ConstraintSatisfaction,
};
use crate::fractal_integration::FractalRuntime;
use crate::layer_adaptation::AdaptedCapabilities;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Composition engine
///
/// Evaluates composition requests against available capabilities.
pub struct CompositionEngine {
    /// Current runtime with layer-adapted capabilities
    pub(crate) runtime: Arc<FractalRuntime>,

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
    ///
    /// # Errors
    ///
    /// Returns error if fractal runtime initialization fails.
    pub async fn from_runtime() -> ToadStoolResult<Self> {
        let runtime = FractalRuntime::init().await?;
        Self::new(Arc::new(runtime))
    }

    /// Create engine with specific runtime
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`; the `Result` type is reserved for future validation.
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
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`; the `Result` type is reserved for future failures.
    pub async fn evaluate(
        &self,
        request: &CompositionRequest,
    ) -> ToadStoolResult<ConstraintEvaluation> {
        let start = std::time::Instant::now();

        info!("🔍 Evaluating composition request: {}", request);

        let mut results = HashMap::new();
        let mut all_hard_satisfied = true;

        for constraint in &request.constraints {
            let satisfaction = evaluators::evaluate_constraint(
                self.runtime.as_ref(),
                &self.capabilities,
                constraint,
            );

            debug!("  {} -> {:?}", constraint, satisfaction);

            if constraint.is_hard() && !satisfaction.is_satisfied() {
                all_hard_satisfied = false;
                warn!("  ❌ Hard constraint failed: {}", constraint);
            }

            results.insert(constraint.name().to_string(), satisfaction);
        }

        let overall_score = self.calculate_overall_score(&results, request);

        let evaluation = ConstraintEvaluation {
            request: request.clone(),
            results,
            overall_score,
            is_feasible: all_hard_satisfied,
        };

        #[expect(
            clippy::cast_precision_loss,
            reason = "integer count to f64 acceptable"
        )]
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

    fn calculate_overall_score(
        &self,
        results: &HashMap<String, ConstraintSatisfaction>,
        request: &CompositionRequest,
    ) -> f64 {
        if results.is_empty() {
            return 1.0;
        }

        let hard_score: f64 = {
            let hard_results: Vec<_> = request
                .hard_constraints()
                .iter()
                .filter_map(|c| results.get(c.name()))
                .collect();

            if hard_results.is_empty() || hard_results.iter().all(|s| s.is_satisfied()) {
                1.0
            } else {
                0.0
            }
        };

        if hard_score.abs() < f64::EPSILON {
            return 0.0;
        }

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
                let len = soft_results.len();
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "integer count to f64 acceptable"
                )]
                let result = total / len as f64;
                result
            }
        };

        hard_score.mul_add(0.7, soft_score * 0.3)
    }

    async fn update_stats(&self, evaluation: &ConstraintEvaluation, duration_ms: f64) {
        let mut stats = self.stats.write().await;

        stats.total_evaluations += 1;

        if evaluation.is_feasible {
            stats.feasible_count += 1;
        } else {
            stats.infeasible_count += 1;
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "integer count to f64 acceptable"
        )]
        let total = stats.total_evaluations as f64;
        stats.avg_evaluation_ms = stats.avg_evaluation_ms.mul_add(total - 1.0, duration_ms) / total;
    }

    /// Get engine statistics
    pub async fn stats(&self) -> EngineStats {
        self.stats.read().await.clone()
    }

    /// Get current capabilities
    pub const fn capabilities(&self) -> &AdaptedCapabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests;
