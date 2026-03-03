// SPDX-License-Identifier: AGPL-3.0-or-later
//! Common Load Balancing Module
//!
//! Generic load balancing abstractions for balancing work across nodes, clouds, and devices.

pub mod types;

pub use types::{
    FailoverConfig, HealthCheckConfig, HealthStatus, LoadBalancerConfig, LoadBalancingAdvice,
    LoadBalancingAlgorithm, LoadBalancingStrategy, LoadMetrics,
};
