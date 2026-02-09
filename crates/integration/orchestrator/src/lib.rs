//! # Generic Orchestrator Integration
//!
//! **LEGACY CRATE - DEPRECATED**
//!
//! This crate contains legacy HTTP-based orchestrator patterns and is being
//! phased out in favor of direct Songbird integration via Unix socket RPC.
//!
//! ## Migration
//!
//! Use these replacements:
//! - `toadstool-distributed::songbird_integration::SongbirdClient` for orchestration
//! - `toadstool-common::primal_sockets::discover_coordination_socket()` for discovery
//! - Unix socket RPC for all primal-to-primal communication
//!
//! All methods in this crate return stub responses and log deprecation warnings.
//! Remove usage from new code.
#![deprecated(
    since = "0.2.0",
    note = "Use toadstool-distributed::songbird_integration instead"
)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

use toadstool_common::infant_discovery::{CapabilityDiscovery, DiscoveryError};

/// Generic orchestrator client - LEGACY
///
/// **DEPRECATED**: Use Songbird integration directly
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OrchestratorClient {
    /// Service name (discovered)
    service_name: String,
    /// Request timeout
    #[allow(dead_code)]
    timeout: Duration,
}

impl OrchestratorClient {
    /// Discover and connect to an orchestrator via capability - STUB
    ///
    /// **DEPRECATED**: Use `SongbirdClient::discover()` instead
    pub async fn discover(discovery: &dyn CapabilityDiscovery) -> Result<Self, OrchestratorError> {
        warn!("OrchestratorClient::discover() is deprecated - use SongbirdClient instead");

        // Stub implementation
        let _ = discovery;
        Ok(Self {
            service_name: "orchestrator-stub".to_string(),
            timeout: Duration::from_secs(30),
        })
    }

    /// Create client with explicit endpoint - STUB
    ///
    /// **DEPRECATED**: Use Songbird integration
    pub fn with_endpoint(endpoint: String) -> Self {
        warn!("OrchestratorClient::with_endpoint() is deprecated - use SongbirdClient");
        let _ = endpoint;
        Self {
            service_name: "orchestrator-stub".to_string(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Register service - STUB
    ///
    /// **DEPRECATED**: Use Songbird service registration
    pub async fn register(
        &self,
        registration: ServiceRegistration,
    ) -> Result<RegistrationResponse, OrchestratorError> {
        info!(
            "Stub: register service {} - use Songbird",
            registration.service_id
        );
        Ok(RegistrationResponse {
            service_id: registration.service_id,
            status: "stub".to_string(),
            message: Some("Use Songbird for service registration".to_string()),
        })
    }

    /// Report health - STUB
    pub async fn report_health(&self, health: HealthReport) -> Result<(), OrchestratorError> {
        info!(
            "Stub: report health for {} - use Songbird",
            health.service_id
        );
        Ok(())
    }

    /// Discover service - STUB
    pub async fn discover_service(
        &self,
        capability: &str,
    ) -> Result<Vec<DiscoveredServiceInfo>, OrchestratorError> {
        info!(
            "Stub: discover service with capability {} - use Songbird",
            capability
        );
        Ok(vec![])
    }

    /// Deregister service - STUB
    pub async fn deregister(&self, service_id: &str) -> Result<(), OrchestratorError> {
        info!("Stub: deregister service {} - use Songbird", service_id);
        Ok(())
    }
}

/// Service registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    pub service_id: String,
    pub service_type: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    pub service_id: String,
    pub status: String,
    pub message: Option<String>,
}

/// Health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub service_id: String,
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: Option<HashMap<String, f64>>,
}

/// Discovered service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredServiceInfo {
    pub service_id: String,
    pub service_type: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
}

/// Orchestrator error types
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl From<DiscoveryError> for OrchestratorError {
    fn from(e: DiscoveryError) -> Self {
        OrchestratorError::DiscoveryFailed(e.to_string())
    }
}
