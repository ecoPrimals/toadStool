// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use tracing::{debug, error, info, warn};

use crate::ecosystem::types::ServiceStatus;

use super::ServiceManager;

impl ServiceManager {
    /// Get service status
    pub async fn get_service_status(&self, service_id: &str) -> Option<ServiceStatus> {
        let statuses = self.statuses.read().unwrap_or_else(|e| e.into_inner());
        statuses.get(service_id).cloned()
    }

    /// Update service status
    pub async fn update_service_status(&self, service_id: &str, status: ServiceStatus) {
        debug!("📊 Updating service status: {} -> {:?}", service_id, status);

        let mut statuses = self.statuses.write().unwrap_or_else(|e| e.into_inner());
        statuses.insert(service_id.to_string(), status);
    }

    /// Get all service statuses
    pub async fn get_all_statuses(&self) -> HashMap<String, ServiceStatus> {
        let statuses = self.statuses.read().unwrap_or_else(|e| e.into_inner());
        statuses.clone()
    }

    /// Mark service as connected
    pub async fn mark_connected(&self, service_id: &str) {
        info!("🔗 Service connected: {}", service_id);
        self.update_service_status(service_id, ServiceStatus::Connected)
            .await;
    }

    /// Mark service as disconnected
    pub async fn mark_disconnected(&self, service_id: &str) {
        warn!("⚠️  Service disconnected: {}", service_id);
        self.update_service_status(service_id, ServiceStatus::Disconnected)
            .await;
    }

    /// Mark service as failed
    pub async fn mark_failed(&self, service_id: &str, reason: impl Into<String>) {
        let reason = reason.into();
        error!("❌ Service failed: {} - {}", service_id, reason);
        self.update_service_status(service_id, ServiceStatus::Failed(reason))
            .await;
    }

    /// Get count of services by status
    pub async fn count_by_status(&self, status: ServiceStatus) -> usize {
        let statuses = self.statuses.read().unwrap_or_else(|e| e.into_inner());
        statuses.values().filter(|s| **s == status).count()
    }
}
