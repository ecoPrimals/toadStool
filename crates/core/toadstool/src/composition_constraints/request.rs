// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Composition request types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::constraint::Constraint;

/// Priority level for composition requests
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ConstraintPriority {
    /// Lowest priority; best-effort scheduling.
    Background = 0,
    /// Default priority.
    #[default]
    Normal = 1,
    /// Higher than normal; prefer earlier scheduling.
    High = 2,
    /// Highest priority; schedule as soon as possible.
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
    /// Request identifier.
    pub name: String,
    /// Hard and soft constraints.
    pub constraints: Vec<Constraint>,
    /// Scheduling priority.
    pub priority: ConstraintPriority,
    /// Optional key-value metadata.
    pub metadata: HashMap<String, String>,
}

impl CompositionRequest {
    /// Creates a new composition request with default priority.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            constraints: Vec::new(),
            priority: ConstraintPriority::default(),
            metadata: HashMap::new(),
        }
    }

    /// Adds a constraint to the request.
    #[must_use]
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Sets the scheduling priority.
    #[must_use]
    pub const fn with_priority(mut self, priority: ConstraintPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Adds a metadata key-value pair.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Returns only hard (must-satisfy) constraints.
    pub fn hard_constraints(&self) -> Vec<&Constraint> {
        self.constraints.iter().filter(|c| c.is_hard()).collect()
    }

    /// Returns only soft (preference) constraints.
    pub fn soft_constraints(&self) -> Vec<&Constraint> {
        self.constraints.iter().filter(|c| c.is_soft()).collect()
    }

    /// Returns (`hard_count`, `soft_count`).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_request() {
        let request = CompositionRequest::new("test_workload");
        assert_eq!(request.name, "test_workload");
        assert!(request.constraints.is_empty());
        assert_eq!(request.priority, ConstraintPriority::Normal);
        assert!(request.metadata.is_empty());
    }

    #[test]
    fn test_with_constraint() {
        let request = CompositionRequest::new("test")
            .with_constraint(Constraint::RequiresGPU)
            .with_constraint(Constraint::MinMemoryGB(8.0));

        assert_eq!(request.constraints.len(), 2);
        assert_eq!(request.constraints[0], Constraint::RequiresGPU);
        assert_eq!(request.constraints[1], Constraint::MinMemoryGB(8.0));
    }

    #[test]
    fn test_with_priority() {
        let request = CompositionRequest::new("test").with_priority(ConstraintPriority::Critical);

        assert_eq!(request.priority, ConstraintPriority::Critical);
    }

    #[test]
    fn test_with_metadata() {
        let request = CompositionRequest::new("test")
            .with_metadata("user", "alice")
            .with_metadata("session", "12345");

        assert_eq!(request.metadata.get("user"), Some(&"alice".to_string()));
        assert_eq!(request.metadata.get("session"), Some(&"12345".to_string()));
    }

    #[test]
    fn test_hard_constraints() {
        let request = CompositionRequest::new("test")
            .with_constraint(Constraint::RequiresGPU)
            .with_constraint(Constraint::PrefersGPU)
            .with_constraint(Constraint::MinMemoryGB(8.0))
            .with_constraint(Constraint::PreferLocal);

        let hard = request.hard_constraints();
        assert_eq!(hard.len(), 2);
        assert!(hard.iter().all(|c| c.is_hard()));
    }

    #[test]
    fn test_soft_constraints() {
        let request = CompositionRequest::new("test")
            .with_constraint(Constraint::RequiresGPU)
            .with_constraint(Constraint::PrefersGPU)
            .with_constraint(Constraint::MinMemoryGB(8.0))
            .with_constraint(Constraint::PreferLocal);

        let soft = request.soft_constraints();
        assert_eq!(soft.len(), 2);
        assert!(soft.iter().all(|c| c.is_soft()));
    }

    #[test]
    fn test_constraint_count() {
        let request = CompositionRequest::new("test")
            .with_constraint(Constraint::RequiresGPU)
            .with_constraint(Constraint::PrefersGPU)
            .with_constraint(Constraint::MinMemoryGB(8.0))
            .with_constraint(Constraint::PreferLocal)
            .with_constraint(Constraint::MustBeLocal);

        let (hard, soft) = request.constraint_count();
        assert_eq!(hard, 3);
        assert_eq!(soft, 2);
    }

    #[test]
    fn test_constraint_priority_ordering() {
        assert!(ConstraintPriority::Background < ConstraintPriority::Normal);
        assert!(ConstraintPriority::Normal < ConstraintPriority::High);
        assert!(ConstraintPriority::High < ConstraintPriority::Critical);
    }

    #[test]
    fn test_constraint_priority_display() {
        assert_eq!(format!("{}", ConstraintPriority::Background), "Background");
        assert_eq!(format!("{}", ConstraintPriority::Normal), "Normal");
        assert_eq!(format!("{}", ConstraintPriority::High), "High");
        assert_eq!(format!("{}", ConstraintPriority::Critical), "Critical");
    }

    #[test]
    fn test_constraint_priority_default() {
        let priority = ConstraintPriority::default();
        assert_eq!(priority, ConstraintPriority::Normal);
    }

    #[test]
    fn test_request_display() {
        let request = CompositionRequest::new("ml_inference")
            .with_constraint(Constraint::RequiresGPU)
            .with_constraint(Constraint::PrefersGPU)
            .with_priority(ConstraintPriority::High);

        let display = format!("{request}");
        assert!(display.contains("ml_inference"));
        assert!(display.contains("High"));
        assert!(display.contains("1 hard"));
        assert!(display.contains("1 soft"));
    }

    #[test]
    fn test_request_serde_roundtrip() {
        let request = CompositionRequest::new("test")
            .with_constraint(Constraint::RequiresGPU)
            .with_constraint(Constraint::MinMemoryGB(16.0))
            .with_priority(ConstraintPriority::Critical)
            .with_metadata("key", "value");

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CompositionRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.name, deserialized.name);
        assert_eq!(request.constraints.len(), deserialized.constraints.len());
        assert_eq!(request.priority, deserialized.priority);
        assert_eq!(request.metadata, deserialized.metadata);
    }

    #[test]
    fn test_empty_request_constraint_count() {
        let request = CompositionRequest::new("empty");
        let (hard, soft) = request.constraint_count();
        assert_eq!(hard, 0);
        assert_eq!(soft, 0);
    }

    #[test]
    fn test_composition_request_new_with_string() {
        let request = CompositionRequest::new(String::from("dynamic_name"));
        assert_eq!(request.name, "dynamic_name");
    }

    #[test]
    fn test_constraint_priority_equality() {
        assert_eq!(
            ConstraintPriority::Background,
            ConstraintPriority::Background
        );
        assert_ne!(ConstraintPriority::Background, ConstraintPriority::Critical);
    }

    #[test]
    fn test_composition_request_with_multiple_metadata() {
        let request = CompositionRequest::new("test")
            .with_metadata("k1", "v1")
            .with_metadata("k2", "v2")
            .with_metadata("k3", "v3");
        assert_eq!(request.metadata.len(), 3);
        assert_eq!(request.metadata.get("k2"), Some(&"v2".to_string()));
    }

    #[test]
    fn test_composition_request_serde_roundtrip_minimal() {
        let request = CompositionRequest::new("minimal");
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CompositionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.name, deserialized.name);
        assert!(deserialized.constraints.is_empty());
    }
}
