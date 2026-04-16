// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`CloudProvider`] trait — vendor-agnostic cloud compute.

use std::future::Future;
use std::pin::Pin;

use super::types::{
    CloudCapabilities, CloudError, CostEstimate, GpuType, WorkloadHealth, WorkloadLocation,
    WorkloadSpec,
};

/// Cloud provider trait
///
/// All cloud providers (AWS, GCP, Azure, etc.) implement this trait.
pub trait CloudProvider: Send + Sync {
    /// Get provider name (e.g., "AWS", "GCP", "Azure")
    fn name(&self) -> &str;

    /// Get provider capabilities
    fn capabilities(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<CloudCapabilities, CloudError>> + Send + '_>>;

    /// Deploy a workload to this provider
    ///
    /// Returns instance/deployment ID
    fn deploy_workload<'a>(
        &'a self,
        workload_id: &'a str,
        region: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, CloudError>> + Send + 'a>>;

    /// Migrate workload from another location
    fn migrate_workload<'a>(
        &'a self,
        workload_id: &'a str,
        source: WorkloadLocation,
        target_region: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, CloudError>> + Send + 'a>>;

    /// Check workload health
    fn check_health<'a>(
        &'a self,
        instance_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<WorkloadHealth, CloudError>> + Send + 'a>>;

    /// Terminate workload
    fn terminate_workload<'a>(
        &'a self,
        instance_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), CloudError>> + Send + 'a>>;

    /// Estimate cost for workload
    fn estimate_cost<'a>(
        &'a self,
        workload_spec: &'a WorkloadSpec,
        region: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CostEstimate, CloudError>> + Send + 'a>>;

    /// Get available GPU types
    fn available_gpu_types<'a>(
        &'a self,
        region: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<GpuType>, CloudError>> + Send + 'a>>;
}
