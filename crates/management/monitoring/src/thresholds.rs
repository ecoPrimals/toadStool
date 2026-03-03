// SPDX-License-Identifier: AGPL-3.0-or-later
//! Threshold monitoring and actions
//!
//! Resource threshold configuration and violation handling.

use tracing::{error, warn};

use toadstool::error::ToadStoolResult;
use toadstool::resources::{ResourceRequirements, RuntimeMetrics};

use crate::types::{ResourceMonitorError, ThresholdAction};

use crate::SystemResourceMonitor;

impl SystemResourceMonitor {
    /// Sets resource thresholds for a workload
    pub async fn set_thresholds(
        &self,
        workload_id: &str,
        requirements: ResourceRequirements,
    ) -> ToadStoolResult<()> {
        let mut threshold_data = self.threshold_data.write().await;
        threshold_data.insert(workload_id.to_string(), requirements);
        tracing::debug!("Set thresholds for workload: {}", workload_id);
        Ok(())
    }

    /// Check thresholds and take action if exceeded
    pub(super) fn check_thresholds(
        workload_id: &str,
        metrics: &RuntimeMetrics,
        requirements: &ResourceRequirements,
        action: &ThresholdAction,
    ) -> Result<(), ResourceMonitorError> {
        let mut violations = Vec::new();

        // Check CPU threshold
        if let Some(max_cores) = requirements.cpu.max_cores {
            let cpu_cores_used = metrics.cpu.usage_percent / 100.0;
            if cpu_cores_used > max_cores {
                violations.push(ResourceMonitorError::ThresholdViolation {
                    workload_id: workload_id.to_string(),
                    resource_type: "CPU".to_string(),
                    current_value: cpu_cores_used,
                    threshold: max_cores,
                });
            }
        }

        // Check memory threshold
        if let Some(max_memory) = requirements.memory.max_bytes {
            if metrics.memory.used_bytes > max_memory {
                violations.push(ResourceMonitorError::ThresholdViolation {
                    workload_id: workload_id.to_string(),
                    resource_type: "Memory".to_string(),
                    current_value: metrics.memory.used_bytes as f64,
                    threshold: max_memory as f64,
                });
            }
        }

        // Check storage threshold
        if let Some(max_storage) = requirements.storage.max_bytes {
            let storage_used = metrics.storage.bytes_read + metrics.storage.bytes_written;
            if storage_used > max_storage {
                violations.push(ResourceMonitorError::ThresholdViolation {
                    workload_id: workload_id.to_string(),
                    resource_type: "Storage".to_string(),
                    current_value: storage_used as f64,
                    threshold: max_storage as f64,
                });
            }
        }

        // Handle violations based on action
        if !violations.is_empty() {
            for violation in &violations {
                match action {
                    ThresholdAction::Log => {
                        warn!("Threshold violation: {}", violation);
                    }
                    ThresholdAction::Alert => {
                        error!("ALERT: Threshold violation: {}", violation);
                        // In a real implementation, this would send alerts to monitoring systems
                    }
                    ThresholdAction::Terminate => {
                        error!("TERMINATING: Threshold violation: {}", violation);
                        // In a real implementation, this would terminate the process
                        return Err(violation.clone());
                    }
                }
            }
        }

        Ok(())
    }
}
