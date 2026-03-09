// SPDX-License-Identifier: AGPL-3.0-only

use crate::ecosystem::types::ServiceStatus;

use super::ServiceManager;

impl ServiceManager {
    /// Get healthy services count
    pub async fn healthy_count(&self) -> usize {
        self.count_by_status(ServiceStatus::Connected).await
    }

    /// Get unhealthy services
    pub async fn get_unhealthy_services(&self) -> Vec<String> {
        let statuses = self.statuses.read().await;
        statuses
            .iter()
            .filter(|(_, status)| status.is_error())
            .map(|(id, _)| id.clone())
            .collect()
    }
}
