// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`CloudProvider`] trait — vendor-agnostic cloud compute.

use async_trait::async_trait;

use super::types::{
    CloudCapabilities, CloudError, CostEstimate, GpuType, WorkloadHealth, WorkloadLocation,
    WorkloadSpec,
};

/// Cloud provider trait
///
/// All cloud providers (AWS, GCP, Azure, etc.) implement this trait.
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait CloudProvider: Send + Sync {
    /// Get provider name (e.g., "AWS", "GCP", "Azure")
    fn name(&self) -> &str;

    /// Get provider capabilities
    async fn capabilities(&self) -> Result<CloudCapabilities, CloudError>;

    /// Deploy a workload to this provider
    ///
    /// Returns instance/deployment ID
    async fn deploy_workload(&self, workload_id: &str, region: &str) -> Result<String, CloudError>;

    /// Migrate workload from another location
    async fn migrate_workload(
        &self,
        workload_id: &str,
        source: WorkloadLocation,
        target_region: &str,
    ) -> Result<String, CloudError>;

    /// Check workload health
    async fn check_health(&self, instance_id: &str) -> Result<WorkloadHealth, CloudError>;

    /// Terminate workload
    async fn terminate_workload(&self, instance_id: &str) -> Result<(), CloudError>;

    /// Estimate cost for workload
    async fn estimate_cost(
        &self,
        workload_spec: &WorkloadSpec,
        region: &str,
    ) -> Result<CostEstimate, CloudError>;

    /// Get available GPU types
    async fn available_gpu_types(&self, region: &str) -> Result<Vec<GpuType>, CloudError>;
}
