//! Common Distribution Module
//!
//! Generic distribution abstractions for distributing work across nodes, clouds, and devices.

pub mod types;

pub use types::{
    DistributionAlgorithm, DistributionConfig, DistributionPlan, DistributionResult,
    DistributionStrategy, DistributionTarget, ResourceCapacity, TargetType,
};
