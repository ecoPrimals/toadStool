// SPDX-License-Identifier: AGPL-3.0-or-later
//! Statistics, evolution metrics, backend selection, and resource monitor wiring.

use std::sync::Arc;

use toadstool::WorkloadType;
use toadstool::resources::ResourceMonitor;

use crate::strategy::{BackendSelectionStrategy, EvolutionMetrics};
use crate::types::{ComputeEngineStatistics, GpuFramework};

use super::UniversalGpuEngine;

impl UniversalGpuEngine {
    /// Set resource monitor
    #[must_use]
    pub fn with_resource_monitor(mut self, monitor: Arc<dyn ResourceMonitor>) -> Self {
        self.resource_monitor = Some(monitor);
        self
    }

    /// Get engine statistics
    pub async fn get_statistics(&self) -> ComputeEngineStatistics {
        let devices = self.devices.read().await;
        let sessions = self.active_sessions.read().await;
        let frameworks = self.frameworks.read().await;

        let recursive_sessions = sessions.values().filter(|s| s.recursion_depth > 0).count();
        let max_recursion_depth = sessions
            .values()
            .map(|s| s.recursion_depth)
            .max()
            .unwrap_or(0);

        ComputeEngineStatistics {
            total_devices: devices.len(),
            active_sessions: sessions.len(),
            frameworks_available: frameworks.len(),
            recursive_sessions,
            max_recursion_depth,
        }
    }

    /// Log current evolution status
    pub(super) async fn log_evolution_status(&self) {
        let metrics = self.evolution_metrics.read().await;
        metrics.log_status();
    }

    /// Get evolution metrics
    pub async fn get_evolution_metrics(&self) -> EvolutionMetrics {
        self.evolution_metrics.read().await.clone()
    }

    /// Update evolution metrics (for future dynamic tracking)
    pub async fn update_evolution_metrics(&self, metrics: EvolutionMetrics) {
        *self.evolution_metrics.write().await = metrics;
        self.log_evolution_status().await;
    }

    /// Get backend selection strategy
    pub fn get_selection_strategy(&self) -> BackendSelectionStrategy {
        self.selection_strategy.clone()
    }

    /// Select best framework for a workload
    pub async fn select_framework_for_workload(
        &self,
        workload: Option<&WorkloadType>,
    ) -> Option<GpuFramework> {
        let frameworks = self.frameworks.read().await;
        let available: Vec<GpuFramework> = frameworks.keys().cloned().collect();
        drop(frameworks);

        self.selection_strategy
            .select_framework(workload, &available)
    }
}
