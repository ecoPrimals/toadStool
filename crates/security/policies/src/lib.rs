//! Advanced Security Policy Management for ToadStool
//!
//! This crate provides comprehensive security policy management, including:
//! - Policy composition and validation
//! - Dynamic policy resolution
//! - Cross-platform security enforcement
//! - Security event monitoring and alerting
//!
//! # Module Organization
//!
//! - [`types`] - Core type definitions (policies, rules, conditions, actions)
//! - [`manager`] - Policy management trait and implementations
//! - [`evaluator`] - Policy condition evaluation logic
//! - [`executor`] - Policy action execution logic

pub mod evaluator;
pub mod executor;
pub mod manager;
pub mod types;

// Re-export public types for convenience
pub use types::{
    AppliedRule, FilePolicyConfig, LogicalOperator, PolicyAction, PolicyCondition,
    PolicyEvaluationContext, PolicyEvaluationResult, PolicyManagerConfig, PolicyResult, PolicyRule,
    PolicyWarning, ResourceModification, SecurityModification, SecurityPolicy, SystemInfo,
    UserInfo, ViolationAction,
};

// Re-export manager types and traits
pub use manager::{FilePolicyManager, PolicyManager};

// Re-export evaluator and executor
pub use evaluator::ConditionEvaluator;
pub use executor::ActionExecutor;

// Unit tests for library code coverage
#[cfg(test)]
mod lib_tests;
