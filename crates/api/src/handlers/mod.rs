//! API handlers module
//!
//! This module contains all HTTP handlers for the ToadStool API, organized by concern:
//! - `execution`: Execution lifecycle management (submit, status, list, cancel)
//! - `logs`: Log retrieval and parsing
//! - `metrics`: Execution and API metrics
//! - `cluster`: Cluster status and management
//! - `health`: Health checks and system status
//! - `workload`: Workload execution via primal capability system
//! - `helpers`: Shared utility functions

// Re-export all handlers for easy access
pub use cluster::get_cluster_status;
pub use execution::{cancel_execution, get_execution_status, list_executions, submit_execution};
pub use health::health_check;
pub use logs::get_execution_logs;
pub use metrics::{get_api_metrics, get_execution_metrics};
pub use workload::execute_workload;

// Public modules
pub mod cluster;
pub mod execution;
pub mod health;
pub mod helpers;
pub mod logs;
pub mod metrics;
pub mod workload;
