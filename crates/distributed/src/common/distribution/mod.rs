// SPDX-License-Identifier: AGPL-3.0-or-later
//! Common Distribution Module
//!
//! Generic distribution abstractions for distributing work across nodes, clouds, and devices.

pub mod types;

pub use types::{
    DistributionAlgorithm, DistributionConfig, DistributionPlan, DistributionResult,
    DistributionStrategy, DistributionTarget, ResourceCapacity, TargetType,
};
