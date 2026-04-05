// SPDX-License-Identifier: AGPL-3.0-or-later
//! Aggregated resource totals used when validating BYOB deployments against team quotas.
//!
//! Totals are derived by summing per-service [`crate::byob::byob_types::ServiceResourceRequirements`]
//! with missing fields treated as zero.
//!
//! This aggregation is intentionally conservative: unspecified limits contribute nothing to the
//! sum, so teams only pay for what they declare. Quota enforcement compares these totals against
//! [`crate::byob::byob_types::TeamResourceQuotas`] on the enclosing deployment request.
//!
//! GPU counts are summed as integers; fractional CPU cores from services are accumulated in
//! floating point and compared against the team's `max_cpu_cores` limit.
//!
//! Memory and storage are both byte counts; callers must use consistent units (bytes) across
//! services so the sum is comparable to quota fields on the request.
//!
//! The struct is internal to the validation module; external callers use `DeploymentValidator` only.

use crate::byob::byob_types::ByobDeploymentRequest;

/// Total resources calculated for a deployment (sum across all services).
#[derive(Debug, Default)]
pub(super) struct TotalResources {
    /// Sum of requested CPU cores across services.
    pub(super) cpu: f64,
    /// Sum of requested memory in bytes across services.
    pub(super) memory: u64,
    /// Sum of requested storage in bytes across services.
    pub(super) storage: u64,
    /// Sum of requested GPU devices across services.
    pub(super) gpu: u32,
}

impl TotalResources {
    /// Aggregate resource requirements from every service in the deployment request.
    ///
    /// Iteration order follows the map's arbitrary order; addition is commutative so the sum is
    /// stable regardless of service ordering.
    pub(super) fn from_request(request: &ByobDeploymentRequest) -> Self {
        let mut total = TotalResources::default();

        for spec in request.services.values() {
            total.cpu += spec.resources.cpu_cores.unwrap_or(0.0);
            total.memory += spec.resources.memory_bytes.unwrap_or(0);
            total.storage += spec.resources.storage_bytes.unwrap_or(0);
            total.gpu += spec.resources.gpu_count.unwrap_or(0);
        }

        total
    }
}
