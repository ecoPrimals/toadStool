// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security integration trait for testability and abstraction

use std::collections::HashMap;

use toadstool::{error::ToadStoolResult, security::SecurityContext};

use super::auth::{AuthResponse, AuthzResponse};
use super::client::SecurityServiceIntegration;

/// Trait for security-service auth/authz operations (enables mocking in tests).
#[expect(
    async_fn_in_trait,
    reason = "all implementors are Send + Sync; trait is internal, no dyn dispatch"
)]
pub trait SecurityServiceIntegrationTrait: Send + Sync {
    /// Authenticate and obtain access token
    async fn authenticate(
        &self,
        service_id: &str,
        service_type: &str,
        capabilities: Vec<String>,
        security_context: SecurityContext,
    ) -> ToadStoolResult<AuthResponse>;

    /// Check authorization for resource/action
    async fn authorize(
        &self,
        resource: &str,
        action: &str,
        context: HashMap<String, serde_json::Value>,
    ) -> ToadStoolResult<AuthzResponse>;

    /// Perform zero-trust validation
    async fn zero_trust_validation(
        &self,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<bool>;
}

impl SecurityServiceIntegrationTrait for SecurityServiceIntegration {
    async fn authenticate(
        &self,
        service_id: &str,
        service_type: &str,
        capabilities: Vec<String>,
        security_context: SecurityContext,
    ) -> ToadStoolResult<AuthResponse> {
        self.authenticate(service_id, service_type, capabilities, security_context)
            .await
    }

    async fn authorize(
        &self,
        resource: &str,
        action: &str,
        context: HashMap<String, serde_json::Value>,
    ) -> ToadStoolResult<AuthzResponse> {
        self.authorize(resource, action, context).await
    }

    async fn zero_trust_validation(
        &self,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<bool> {
        self.zero_trust_validation(security_context).await
    }
}
