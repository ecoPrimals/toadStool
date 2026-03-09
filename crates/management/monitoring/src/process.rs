// SPDX-License-Identifier: AGPL-3.0-only
//! Process management for resource monitoring
//!
//! Registration, unregistration, and metrics retrieval for monitored processes.

use std::path::Path;
use std::time::Instant;
use tracing::info;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::resources::RuntimeMetrics;

use crate::types::ResourceMonitorError;
use crate::SystemResourceMonitor;

/// Internal process information for monitoring
#[derive(Clone, Debug)]
pub(crate) struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub last_cpu_time: u64,
    pub memory_usage: u64,
    pub start_time: u64,
}

impl SystemResourceMonitor {
    /// Registers a process for resource monitoring
    pub async fn register_process(
        &self,
        workload_id: &str,
        process_handle: u32,
        executable_path: &Path,
    ) -> Result<(), ToadStoolError> {
        let mut process_map = self.process_map.write().await;
        let process_info = ProcessInfo {
            pid: process_handle,
            name: executable_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            last_cpu_time: 0,
            memory_usage: 0,
            start_time: Instant::now().elapsed().as_secs(),
        };

        process_map.insert(workload_id.to_string(), process_info);
        info!(
            "Registered process {} with PID {} for monitoring",
            workload_id, process_handle
        );
        Ok(())
    }

    /// Unregisters a process from monitoring
    pub async fn unregister_process(&self, workload_id: &str) -> Result<(), ToadStoolError> {
        let mut process_map = self.process_map.write().await;
        let mut usage_data = self.usage_data.write().await;
        let mut threshold_data = self.threshold_data.write().await;

        if process_map.remove(workload_id).is_some() {
            usage_data.remove(workload_id);
            threshold_data.remove(workload_id);
            info!("Unregistered process {} from monitoring", workload_id);
            Ok(())
        } else {
            Err(ResourceMonitorError::ProcessNotRegistered(workload_id.to_string()).into())
        }
    }

    /// Gets current metrics for a workload (async version)
    pub async fn get_metrics_async(&self, workload_id: &str) -> ToadStoolResult<RuntimeMetrics> {
        let usage_data = self.usage_data.read().await;
        usage_data.get(workload_id).cloned().ok_or_else(|| {
            ResourceMonitorError::ProcessNotRegistered(workload_id.to_string()).into()
        })
    }
}
