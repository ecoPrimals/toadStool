// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use toadstool::resources::{ResourceRequirements, RuntimeMetrics};
use tokio::sync::RwLock;

use crate::process::ProcessInfo;
use crate::types::MonitoringConfig;

/// Concrete implementation of `ResourceMonitor` trait that provides
/// configurable, high-granularity resource monitoring
#[derive(Debug)]
pub struct SystemResourceMonitor {
    pub(crate) process_map: Arc<RwLock<HashMap<String, ProcessInfo>>>,
    pub(crate) usage_data: Arc<RwLock<HashMap<String, RuntimeMetrics>>>,
    pub(crate) threshold_data: Arc<RwLock<HashMap<String, ResourceRequirements>>>,
    pub(crate) config: MonitoringConfig,
    pub(crate) is_monitoring: Arc<RwLock<bool>>,
}
