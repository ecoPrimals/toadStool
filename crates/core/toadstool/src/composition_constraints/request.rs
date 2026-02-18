// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Composition request types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::constraint::Constraint;

/// Priority level for composition requests
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ConstraintPriority {
    Background = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl fmt::Display for ConstraintPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "Background"),
            Self::Normal => write!(f, "Normal"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// A composition request with constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionRequest {
    pub name: String,
    pub constraints: Vec<Constraint>,
    pub priority: ConstraintPriority,
    pub metadata: HashMap<String, String>,
}

impl CompositionRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            constraints: Vec::new(),
            priority: ConstraintPriority::default(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn with_priority(mut self, priority: ConstraintPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn hard_constraints(&self) -> Vec<&Constraint> {
        self.constraints.iter().filter(|c| c.is_hard()).collect()
    }

    pub fn soft_constraints(&self) -> Vec<&Constraint> {
        self.constraints.iter().filter(|c| c.is_soft()).collect()
    }

    pub fn constraint_count(&self) -> (usize, usize) {
        let hard = self.hard_constraints().len();
        let soft = self.soft_constraints().len();
        (hard, soft)
    }
}

impl fmt::Display for CompositionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (hard, soft) = self.constraint_count();
        write!(
            f,
            "Request('{}', priority={}, constraints={} hard + {} soft)",
            self.name, self.priority, hard, soft
        )
    }
}
