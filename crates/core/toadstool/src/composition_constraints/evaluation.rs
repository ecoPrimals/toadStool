// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! Constraint evaluation and satisfaction types

use std::collections::HashMap;

use super::request::CompositionRequest;

/// Constraint satisfaction result
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintSatisfaction {
    /// Constraint fully satisfied.
    Satisfied,
    /// Partially satisfied; payload is score in [0, 1).
    Partial(f64),
    /// Not satisfied; reason explains why.
    Unsatisfied {
        /// Human-readable reason for failure.
        reason: String,
    },
}

impl ConstraintSatisfaction {
    /// Returns true if satisfied or partially satisfied.
    pub const fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied | Self::Partial(_))
    }

    /// Returns true only when fully satisfied.
    pub const fn is_fully_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }

    /// Returns satisfaction score: 1.0 satisfied, partial value, or 0.0 unsatisfied.
    pub const fn score(&self) -> f64 {
        match self {
            Self::Satisfied => 1.0,
            Self::Partial(s) => *s,
            Self::Unsatisfied { .. } => 0.0,
        }
    }
}

/// Constraint evaluation result for a composition request
#[derive(Debug, Clone)]
pub struct ConstraintEvaluation {
    /// Original composition request.
    pub request: CompositionRequest,
    /// Per-constraint satisfaction results.
    pub results: HashMap<String, ConstraintSatisfaction>,
    /// Aggregate score in [0, 1].
    pub overall_score: f64,
    /// True if all hard constraints satisfied.
    pub is_feasible: bool,
}

impl ConstraintEvaluation {
    /// Returns satisfaction for a constraint by name.
    pub fn get_satisfaction(&self, constraint_name: &str) -> Option<&ConstraintSatisfaction> {
        self.results.get(constraint_name)
    }

    /// Returns hard constraints that are not satisfied.
    pub fn unsatisfied_hard_constraints(&self) -> Vec<(&str, &ConstraintSatisfaction)> {
        self.results
            .iter()
            .filter(|(_, sat)| !sat.is_satisfied())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Returns average score across soft constraints only.
    pub fn soft_constraint_score(&self) -> f64 {
        let soft_results: Vec<_> = self
            .request
            .soft_constraints()
            .iter()
            .filter_map(|c| self.results.get(c.name()))
            .collect();

        if soft_results.is_empty() {
            return 1.0;
        }

        let total_score: f64 = soft_results.iter().map(|s| s.score()).sum();
        let len = soft_results.len();
        #[expect(
            clippy::cast_precision_loss,
            reason = "precision loss acceptable for this conversion"
        )]
        let result = total_score / len as f64;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition_constraints::constraint::Constraint;

    #[test]
    fn test_constraint_satisfaction_satisfied() {
        let sat = ConstraintSatisfaction::Satisfied;
        assert!(sat.is_satisfied());
        assert!(sat.is_fully_satisfied());
        assert!((sat.score() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_constraint_satisfaction_partial() {
        let sat = ConstraintSatisfaction::Partial(0.7);
        assert!(sat.is_satisfied());
        assert!(!sat.is_fully_satisfied());
        assert!((sat.score() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_constraint_satisfaction_unsatisfied() {
        let sat = ConstraintSatisfaction::Unsatisfied {
            reason: "No GPU available".to_string(),
        };
        assert!(!sat.is_satisfied());
        assert!(!sat.is_fully_satisfied());
        assert!((sat.score() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_constraint_evaluation_get_satisfaction() {
        let request = CompositionRequest::new("test");
        let mut results = HashMap::new();
        results.insert("gpu".to_string(), ConstraintSatisfaction::Satisfied);
        results.insert("memory".to_string(), ConstraintSatisfaction::Partial(0.8));

        let eval = ConstraintEvaluation {
            request,
            results,
            overall_score: 0.9,
            is_feasible: true,
        };

        assert!(eval.get_satisfaction("gpu").is_some());
        assert!(eval.get_satisfaction("memory").is_some());
        assert!(eval.get_satisfaction("nonexistent").is_none());
    }

    #[test]
    fn test_unsatisfied_hard_constraints() {
        let request = CompositionRequest::new("test");
        let mut results = HashMap::new();
        results.insert("gpu".to_string(), ConstraintSatisfaction::Satisfied);
        results.insert(
            "memory".to_string(),
            ConstraintSatisfaction::Unsatisfied {
                reason: "Insufficient memory".to_string(),
            },
        );
        results.insert("latency".to_string(), ConstraintSatisfaction::Partial(0.5));

        let eval = ConstraintEvaluation {
            request,
            results,
            overall_score: 0.5,
            is_feasible: false,
        };

        let unsatisfied = eval.unsatisfied_hard_constraints();
        assert_eq!(unsatisfied.len(), 1);
        assert_eq!(unsatisfied[0].0, "memory");
    }

    #[test]
    fn test_soft_constraint_score_empty() {
        let request = CompositionRequest::new("test");
        let eval = ConstraintEvaluation {
            request,
            results: HashMap::new(),
            overall_score: 1.0,
            is_feasible: true,
        };

        assert!((eval.soft_constraint_score() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_soft_constraint_score_with_soft_constraints() {
        let request = CompositionRequest::new("test")
            .with_constraint(Constraint::PrefersGPU)
            .with_constraint(Constraint::PreferLocal);

        let mut results = HashMap::new();
        results.insert("prefers_gpu".to_string(), ConstraintSatisfaction::Satisfied);
        results.insert(
            "prefer_local".to_string(),
            ConstraintSatisfaction::Partial(0.5),
        );

        let eval = ConstraintEvaluation {
            request,
            results,
            overall_score: 0.75,
            is_feasible: true,
        };

        assert!((eval.soft_constraint_score() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_soft_constraint_score_mixed_constraints() {
        let request = CompositionRequest::new("test")
            .with_constraint(Constraint::RequiresGPU)
            .with_constraint(Constraint::PrefersGPU);

        let mut results = HashMap::new();
        results.insert(
            "requires_gpu".to_string(),
            ConstraintSatisfaction::Satisfied,
        );
        results.insert("prefers_gpu".to_string(), ConstraintSatisfaction::Satisfied);

        let eval = ConstraintEvaluation {
            request,
            results,
            overall_score: 1.0,
            is_feasible: true,
        };

        assert!((eval.soft_constraint_score() - 1.0).abs() < f64::EPSILON);
    }
}
