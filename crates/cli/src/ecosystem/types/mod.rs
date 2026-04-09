// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ecosystem Integration - Type Definitions
//!
//! # Zero-Copy Optimization (Phase 2.3)
//! Service discovery types use `Arc<str>` for frequently-cloned string fields.

mod crypto;
mod discovery;
mod endpoint;
mod storage;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(test)]
use std::sync::Arc;

pub use crypto::{
    BearDogPermission, CryptoVerificationContext, SecurityPermission, ServiceSignature,
    SignedServiceResponse,
};
pub use discovery::{DiscoveredService, DiscoveryResult, ServiceType};
pub use endpoint::{EcosystemService, ServiceEndpoint, TrustLevel};
pub use storage::{NestGateMount, StorageMount};

/// Ecosystem service discovery and integration (main struct)
pub struct EcosystemIntegrator {
    /// Known ecosystem endpoints
    pub(super) endpoints: HashMap<String, ServiceEndpoint>,
    /// Active connections
    pub(super) connections: HashMap<String, ServiceConnection>,
    /// Authentication credentials
    pub(super) credentials: Option<EcosystemCredentials>,
}

#[derive(Debug, Clone)]
pub(super) struct ServiceConnection {
    pub(super) endpoint: ServiceEndpoint,
    pub(super) status: ConnectionStatus,
    pub(super) last_heartbeat: std::time::SystemTime,
    pub(super) _auth_token: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) enum ConnectionStatus {
    _Connecting,
    Connected,
    _Disconnected,
    _Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct EcosystemCredentials {
    pub(super) identity: String,
    pub(super) private_key: String,
    pub(super) public_key: String,
    pub(super) certificates: Vec<String>,
}

// Private helper types (used internally by EcosystemIntegrator)
// CoordinationRegistration, coordination heartbeat payloads, CoordinationResponse: REMOVED
// These types were part of the deprecated hardcoded coordination-service integration.
// New code uses capability-based adapters in ecosystem::adapters instead.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct EcosystemStatus {
    pub(super) endpoints: HashMap<String, ServiceEndpoint>,
    pub(super) active_connections: usize,
    pub(super) total_discovered: usize,
    pub(super) credential_status: bool,
}

#[cfg(test)]
mod integrator_tests {
    use super::*;

    #[test]
    #[expect(deprecated)]
    fn test_service_endpoint_deserialize_roundtrip() {
        let endpoint = ServiceEndpoint {
            service_type: EcosystemService::Discovery,
            address: "127.0.0.1:8080".parse().unwrap(),
            version: Arc::from("1.0"),
            capabilities: vec!["discovery".to_string()],
            trust_level: TrustLevel::Verified,
        };
        let json = serde_json::to_string(&endpoint).unwrap();
        let restored: ServiceEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.version.as_ref(), "1.0");
        assert!(matches!(restored.service_type, EcosystemService::Discovery));
    }
}
