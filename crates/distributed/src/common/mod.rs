// SPDX-License-Identifier: AGPL-3.0-or-later
//! Common Distributed Computing Abstractions
//!
//! Generic, reusable abstractions for distributed systems including:
//! - Distribution strategies and planning
//! - Load balancing algorithms
//! - Capacity management
//! - Scheduling coordination
//! - Authentication patterns
//!
//! These abstractions are used across Songbird integration, Cloud orchestration,
//! and other distributed computing components.

pub mod capacity;
pub mod distribution;
pub mod load_balancing;

pub mod auth;
pub mod scheduling;

pub use auth::{CapabilityToken, TrustLevel};
pub use capacity::{AvailableCapacity, CapacityConfig, CapacityInfo, CapacityRequirement};
pub use distribution::{DistributionConfig, DistributionPlan, DistributionStrategy};
pub use load_balancing::{LoadBalancerConfig, LoadBalancingStrategy};
pub use scheduling::{PlacementConstraint, SchedulingDecision, SchedulingPriority};
