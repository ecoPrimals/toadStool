// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Constraint evaluation and satisfaction types

use std::collections::HashMap;

use super::request::CompositionRequest;

/// Constraint satisfaction result
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintSatisfaction {
    Satisfied,
    Partial(f64),
    Unsatisfied { reason: String },
}

impl ConstraintSatisfaction {
    pub fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied | Self::Partial(_))
    }

    pub fn is_fully_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }

    pub fn score(&self) -> f64 {
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
    pub request: CompositionRequest,
    pub results: HashMap<String, ConstraintSatisfaction>,
    pub overall_score: f64,
    pub is_feasible: bool,
}

impl ConstraintEvaluation {
    pub fn get_satisfaction(&self, constraint_name: &str) -> Option<&ConstraintSatisfaction> {
        self.results.get(constraint_name)
    }

    pub fn unsatisfied_hard_constraints(&self) -> Vec<(&String, &ConstraintSatisfaction)> {
        self.results
            .iter()
            .filter(|(_, sat)| !sat.is_satisfied())
            .collect()
    }

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
        total_score / soft_results.len() as f64
    }
}
