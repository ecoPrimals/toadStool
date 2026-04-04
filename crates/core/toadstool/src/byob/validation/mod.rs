// SPDX-License-Identifier: AGPL-3.0-only
//! BYOB deployment validation logic
//!
//! **Design**: Extracted validation concerns for clarity and testability

mod quota;
mod services;
mod types;

#[cfg(test)]
mod tests;

use crate::ToadStoolResult;
use crate::byob::byob_types::ByobDeploymentRequest;

/// Validates BYOB deployment requests against resource quotas
pub(super) struct DeploymentValidator;

impl DeploymentValidator {
    /// Validate deployment request against resource quotas
    ///
    /// **Design**: Checks CPU, memory, storage, GPU, and service count limits
    pub fn validate_deployment(request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
        quota::validate_resource_quotas(request)?;
        services::validate_services(request)?;
        Ok(())
    }
}
