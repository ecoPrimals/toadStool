// SPDX-License-Identifier: AGPL-3.0-only
//! Team resource quota checks for BYOB deployments.
//!
//! Validates aggregated CPU, memory, storage, and GPU demand against
//! [`crate::byob::byob_types::TeamResourceQuotas`] attached to the request.
//!
//! Each comparison uses strict greater-than against the quota ceiling: a deployment that exactly
//! matches limits is accepted; any over-allocation fails fast with a resource error that names
//! the dimension that was exceeded.
//!
//! Validation order is CPU, memory, storage, then GPU so the first violated limit is reported.

use crate::byob::byob_types::ByobDeploymentRequest;
use crate::{ToadStoolError, ToadStoolResult};

use super::types::TotalResources;

/// Ensure summed service resources do not exceed the team's quota limits.
pub(super) fn validate_resource_quotas(request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
    let total_resources = TotalResources::from_request(request);

    if total_resources.cpu > request.resource_quotas.max_cpu_cores {
        return Err(ToadStoolError::resource(format!(
            "CPU requirement {:.2} exceeds team quota {:.2}",
            total_resources.cpu, request.resource_quotas.max_cpu_cores
        )));
    }

    if total_resources.memory > request.resource_quotas.max_memory_bytes {
        return Err(ToadStoolError::resource(format!(
            "Memory requirement {} exceeds team quota {}",
            total_resources.memory, request.resource_quotas.max_memory_bytes
        )));
    }

    if total_resources.storage > request.resource_quotas.max_storage_bytes {
        return Err(ToadStoolError::resource(format!(
            "Storage requirement {} exceeds team quota {}",
            total_resources.storage, request.resource_quotas.max_storage_bytes
        )));
    }

    if total_resources.gpu > request.resource_quotas.max_gpu_count {
        return Err(ToadStoolError::resource(format!(
            "GPU requirement {} exceeds team quota {}",
            total_resources.gpu, request.resource_quotas.max_gpu_count
        )));
    }

    Ok(())
}
