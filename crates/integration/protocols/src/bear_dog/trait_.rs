// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use std::collections::HashMap;

use toadstool::{error::ToadStoolResult, security::SecurityContext};

use super::auth::{AuthResponse, AuthzResponse};
use super::client::BearDogIntegration;

#[async_trait]
pub trait BearDogIntegrationTrait: Send + Sync {
    async fn authenticate(
        &self,
        service_id: &str,
        service_type: &str,
        capabilities: Vec<String>,
        security_context: SecurityContext,
    ) -> ToadStoolResult<AuthResponse>;

    async fn authorize(
        &self,
        resource: &str,
        action: &str,
        context: HashMap<String, serde_json::Value>,
    ) -> ToadStoolResult<AuthzResponse>;

    async fn zero_trust_validation(
        &self,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<bool>;
}

#[async_trait]
impl BearDogIntegrationTrait for BearDogIntegration {
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
