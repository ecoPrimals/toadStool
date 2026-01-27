//! Modern Ecosystem service discovery using infant discovery
//!
//! ZERO primal name hardcoding - uses capability-based discovery

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

// Use infant discovery instead of hardcoded primal names
use toadstool_common::infant_discovery::{
    capability_names as capabilities,
    DiscoveryEngineBuilder,
    DiscoveryEngine,
    production_sources,
    standard_detectors,
};

#[derive(Debug, Clone)]
pub struct EcosystemIntegrator {
    /// Infant discovery engine for dynamic service location
    discovery_engine: Arc<DiscoveryEngine>,
    endpoints: HashMap<String, String>,
    credentials: Option<String>,
    storage_connections: Arc<Mutex<HashMap<String, StorageConnectionInfo>>>,
}

#[derive(Debug, Clone)]
pub struct EcosystemService {
    pub name: String,
    pub endpoint: String,
    pub service_type: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct StorageConnectionInfo {
    pub endpoint: String,
    pub status: String,
    pub available_space_gb: u64,
    pub mount_point: PathBuf,
    pub access_mode: String,
}

impl EcosystemIntegrator {
    /// Create new integrator with infant discovery
    pub async fn new() -> Result<Self> {
        // Build infant discovery engine
        let discovery_engine = DiscoveryEngineBuilder::new()
            .add_sources(production_sources())
            .add_detectors(standard_detectors())
            .build()
            .await?;

        Ok(Self {
            discovery_engine: Arc::new(discovery_engine),
            endpoints: HashMap::new(),
            credentials: None,
            storage_connections: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Discover ecosystem services using capabilities
    pub async fn discover_services(
        &self,
        requested_capabilities: Vec<String>,
        _timeout: u64,
    ) -> Result<DiscoveryResult> {
        info!("Starting capability-based ecosystem discovery");

        let start = std::time::Instant::now();
        let mut discovered_services = Vec::new();

        // Discover services by capability, not by name
        let capability_list = if requested_capabilities.is_empty() {
            // Default capabilities to discover
            vec![
                capabilities::ORCHESTRATION.to_string(),
                capabilities::AUTHENTICATION.to_string(),
                capabilities::STORAGE.to_string(),
                capabilities::AI_PROCESSING.to_string(),
            ]
        } else {
            requested_capabilities
        };

        for capability in &capability_list {
            if let Ok(service) = self.discovery_engine.discover(capability, None).await {
                discovered_services.push(DiscoveredService {
                    name: capability.clone(),
                    service_type: service.capability,
                    address: service.endpoint,
                    trust_level: if service.health.is_healthy() { "high" } else { "medium" }.to_string(),
                });
            }
        }

        let total_discovered = discovered_services.len();
        let verified_count = discovered_services.iter().filter(|s| s.trust_level == "high").count();

        let result = DiscoveryResult {
            services: discovered_services,
            total_discovered: total_discovered as u32,
            verified_count: verified_count as u32,
            scan_duration: start.elapsed(),
        };

        Ok(result)
    }

    /// Register with orchestration service (capability-based)
    pub async fn register_with_orchestration(
        &self,
        _endpoint: String,
        _token: Option<String>,
    ) -> Result<()> {
        info!("Registering with orchestration service via capability discovery");
        
        // Discover orchestration capability
        match self.discovery_engine.discover(capabilities::ORCHESTRATION, None).await {
            Ok(service) => {
                info!("Found orchestration service at: {}", service.endpoint);
                // Registration logic would go here
                Ok(())
            }
            Err(e) => {
                warn!("Orchestration service not available: {}", e);
                // Graceful degradation - continue without orchestration
                Ok(())
            }
        }
    }

    /// Install security permissions using capability discovery
    /// 
    /// Zero-Copy Optimization: Takes `&str` instead of `String` to avoid allocation.
    pub async fn install_security_permissions(&self, permissions_file: &str) -> Result<()> {
        info!("Installing security permissions from: {}", permissions_file);

        // Read permissions file
        let permissions_content = std::fs::read_to_string(permissions_file)
            .with_context(|| format!("Failed to read permissions file: {}", permissions_file))?;

        // Parse permissions (assuming JSON format)
        let permissions: serde_json::Value = serde_json::from_str(&permissions_content)
            .with_context(|| "Failed to parse permissions file as JSON")?;

        // Discover security/authentication capability
        match self.discovery_engine.discover(capabilities::AUTHENTICATION, None).await {
            Ok(security_service) => {
                // Send permissions to security service
                let client = reqwest::Client::new();
                let response = client
                    .post(&format!("{}/security/permissions", security_service.endpoint))
                    .header("Content-Type", "application/json")
                    .json(&permissions)
                    .send()
                    .await
                    .with_context(|| "Failed to send permissions to security service")?;

                if response.status().is_success() {
                    info!("Successfully installed permissions via security service API");
                } else {
                    warn!("Security service API returned status: {}", response.status());
                }
            }
            Err(_) => {
                // Fallback: Install locally if security service is not available
                info!("Security service not available, installing permissions locally");

                // Create local permissions directory if it doesn't exist
                let local_perms_dir = std::path::Path::new("/etc/toadstool/security");
                if !local_perms_dir.exists() {
                    std::fs::create_dir_all(local_perms_dir)
                        .with_context(|| "Failed to create local security permissions directory")?;
                }

                // Copy permissions file to local directory
                let local_perms_file = local_perms_dir.join("permissions.json");
                std::fs::copy(&permissions_file, &local_perms_file)
                    .with_context(|| "Failed to copy permissions file to local directory")?;

                info!("Permissions installed locally at: {}", local_perms_file.display());
            }
        }

        Ok(())
    }

    /// Connect to storage using capability discovery
    pub async fn connect_storage(
        &self,
        endpoint_override: Option<String>,
        credentials: String,
    ) -> Result<StorageConnectionInfo> {
        info!("Connecting to storage via capability discovery");

        // Parse credentials
        let creds: serde_json::Value = serde_json::from_str(&credentials)
            .with_context(|| "Failed to parse credentials as JSON")?;

        let auth_token = creds
            .get("auth_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing auth_token in credentials"))?;

        // Use override or discover storage capability
        let endpoint = if let Some(ep) = endpoint_override {
            ep
        } else {
            let storage_service = self.discovery_engine
                .discover(capabilities::STORAGE, None)
                .await
                .with_context(|| "Failed to discover storage service")?;
            storage_service.endpoint
        };

        info!("Connecting to storage at: {}", endpoint);

        // Test connection
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/api/v1/status", endpoint))
            .header("Authorization", format!("Bearer {}", auth_token))
            .send()
            .await
            .with_context(|| "Failed to connect to storage")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Storage connection failed with status: {}",
                response.status()
            ));
        }

        // Get storage information
        let storage_info = response
            .json::<serde_json::Value>()
            .await
            .with_context(|| "Failed to parse storage status response")?;

        let connection_info = StorageConnectionInfo {
            endpoint: endpoint.clone(),
            status: "connected".to_string(),
            available_space_gb: storage_info
                .get("available_space_gb")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            mount_point: std::path::PathBuf::from("/mnt/storage"),
            access_mode: "read-write".to_string(),
        };

        // Store connection info for future use
        let mut connections = self.storage_connections.lock().await;
        connections.insert(endpoint, connection_info.clone());

        info!("Successfully connected to storage");
        Ok(connection_info)
    }

    /// Discover AI processing services
    pub async fn discover_ai_services(&self) -> Result<Vec<EcosystemService>> {
        let mut services = Vec::new();

        // Discover AI capability
        if let Ok(service) = self.discovery_engine.discover(capabilities::AI_PROCESSING, None).await {
            services.push(EcosystemService {
                name: "AI Processing".to_string(),
                endpoint: service.endpoint,
                service_type: capabilities::AI_PROCESSING.to_string(),
                status: "healthy".to_string(),
            });
        }

        // Discover NLP capability
        if let Ok(service) = self.discovery_engine.discover(capabilities::NLP, None).await {
            services.push(EcosystemService {
                name: "Natural Language Processing".to_string(),
                endpoint: service.endpoint,
                service_type: capabilities::NLP.to_string(),
                status: "healthy".to_string(),
            });
        }

        Ok(services)
    }

    /// Discover compute services
    pub async fn discover_compute_services(&self) -> Result<Vec<EcosystemService>> {
        let mut services = Vec::new();

        // Check for local compute capabilities
        if std::path::Path::new("/etc/biomeos").exists() {
            services.push(EcosystemService {
                name: "Local Compute".to_string(),
                endpoint: "local://compute".to_string(),
                service_type: "compute".to_string(),
                status: "healthy".to_string(),
            });
        }

        // Discover remote compute via orchestration
        if let Ok(service) = self.discovery_engine.discover(capabilities::ORCHESTRATION, None).await {
            services.push(EcosystemService {
                name: "Orchestrated Compute".to_string(),
                endpoint: service.endpoint,
                service_type: capabilities::ORCHESTRATION.to_string(),
                status: "healthy".to_string(),
            });
        }

        Ok(services)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub services: Vec<DiscoveredService>,
    pub total_discovered: u32,
    pub verified_count: u32,
    pub scan_duration: std::time::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub name: String,
    pub service_type: String,
    pub address: String,
    pub trust_level: String,
}

