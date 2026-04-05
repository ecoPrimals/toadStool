// SPDX-License-Identifier: AGPL-3.0-or-later
//! Component instance management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::core::ComponentInterface;

/// Component instance
#[derive(Debug)]
pub struct ComponentInstance {
    /// Instance ID
    pub id: String,
    /// Component interfaces
    pub interfaces: HashMap<String, ComponentInterface>,
    /// Instance state
    pub state: ComponentState,
    /// Creation timestamp
    pub created_at: std::time::SystemTime,
    /// Resource usage
    pub resource_usage: ComponentResourceUsage,
}

/// Component state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentState {
    /// Component is being initialized
    Initializing,
    /// Component is ready for use
    Ready,
    /// Component is executing
    Running,
    /// Component has failed
    Failed {
        /// Error message describing the failure
        error: String,
    },
    /// Component is shutting down
    Terminating,
}

/// Component resource usage tracking
#[derive(Debug, Default, Clone)]
pub struct ComponentResourceUsage {
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// CPU time in milliseconds
    pub cpu_time_ms: u64,
    /// Number of function calls
    pub function_calls: u64,
    /// Number of interface imports
    pub imports_count: u32,
    /// Number of interface exports
    pub exports_count: u32,
}
