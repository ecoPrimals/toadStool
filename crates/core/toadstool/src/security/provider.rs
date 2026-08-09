// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security provider trait

use crate::ToadStoolResult;

use super::{AuditEvent, Capability, SecurityContext, SecurityPolicy};

/// Security provider trait
#[expect(
    async_fn_in_trait,
    reason = "all implementors are Send + Sync; trait is internal, no dyn dispatch"
)]
pub trait SecurityProvider: Send + Sync {
    /// Create a security context
    async fn create_security_context(
        &self,
        policy: &SecurityPolicy,
    ) -> ToadStoolResult<SecurityContext>;

    /// Validate a security context
    async fn validate_security_context(&self, context: &SecurityContext) -> ToadStoolResult<()>;

    /// Apply security context to a workload
    async fn apply_security_context(
        &self,
        context: &SecurityContext,
        workload_id: &str,
    ) -> ToadStoolResult<()>;

    /// Remove security context from a workload
    async fn remove_security_context(&self, workload_id: &str) -> ToadStoolResult<()>;

    /// Check if a capability is allowed
    async fn check_capability(
        &self,
        context: &SecurityContext,
        capability: &Capability,
    ) -> ToadStoolResult<bool>;

    /// Audit security event
    async fn audit_event(
        &self,
        event: AuditEvent,
        context: &SecurityContext,
    ) -> ToadStoolResult<()>;
}
