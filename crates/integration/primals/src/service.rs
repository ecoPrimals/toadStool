//! Service registration and startup types.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Service registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    /// Service ID
    pub service_id: Uuid,
    /// Service name
    pub service_name: String,
    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,
    /// Service metadata
    pub metadata: HashMap<String, String>,
    /// Health check endpoint
    pub health_endpoint: Option<String>,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Endpoint name
    pub name: String,
    /// Protocol (http, https, grpc, etc.)
    pub protocol: String,
    /// Host address
    pub host: String,
    /// Port number
    pub port: u16,
    /// Base path
    pub path: Option<String>,
}

/// Startup result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupResult {
    /// Startup duration
    pub duration: Duration,
    /// Services started
    pub services_started: Vec<String>,
    /// Startup logs
    pub logs: Vec<String>,
    /// Startup status
    pub status: StartupStatus,
}

/// Startup status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartupStatus {
    /// Startup successful
    Success,
    /// Startup failed
    Failed(String),
    /// Startup partially successful
    Partial(Vec<String>),
}
