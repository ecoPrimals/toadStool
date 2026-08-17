// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;

use std::sync::RwLock;
use toadstool_core::resources::{ResourceRequirements, RuntimeMetrics};

use crate::types::MonitoringConfig;

/// Internal process information for monitoring
#[derive(Clone, Debug)]
pub(crate) struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub last_cpu_time: u64,
    pub memory_usage: u64,
    pub start_time: u64,
}

/// Concrete implementation of `ResourceMonitor` trait that provides
/// configurable, high-granularity resource monitoring
#[derive(Debug)]
pub struct SystemResourceMonitor {
    pub(crate) process_map: Arc<RwLock<HashMap<String, ProcessInfo>>>,
    pub(crate) usage_data: Arc<RwLock<HashMap<String, RuntimeMetrics>>>,
    pub(crate) threshold_data: Arc<RwLock<HashMap<String, ResourceRequirements>>>,
    pub(crate) config: MonitoringConfig,
    pub(crate) is_monitoring: Arc<RwLock<bool>>,
    /// Workload IDs registered via `ResourceMonitor::start_monitoring`.
    pub(crate) monitored_workloads: Arc<Mutex<HashSet<String>>>,
}
