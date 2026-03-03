// SPDX-License-Identifier: AGPL-3.0-or-later
//! HTTP API handlers for server endpoints
//!
//! Organized by resource/domain: execution, capabilities, cluster, health.

mod capabilities;
mod cluster;
mod execution;
mod health;

pub use capabilities::list_runtime_engines_handler;
pub use cluster::get_cluster_status_handler;
pub use execution::{
    cancel_execution_handler, get_execution_status_handler, submit_execution_handler,
};
pub use health::{
    dashboard_handler, health_check_handler, metrics_handler, readiness_check_handler,
    DASHBOARD_HTML,
};

#[cfg(test)]
mod tests;
