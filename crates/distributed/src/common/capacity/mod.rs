// SPDX-License-Identifier: AGPL-3.0-or-later
//! Common Capacity Management Module
//!
//! Generic capacity tracking and management abstractions.

pub mod types;

pub use types::{
    AvailableCapacity, CapacityAlert, CapacityConfig, CapacityInfo, CapacityRequirement,
    NetworkCapacity, ResourceUsageSnapshot,
};
