//! # Biome Manifest Handling
//!
//! Parsing, validation, and processing of biome.yaml manifests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Errors related to manifest processing
#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("Failed to read manifest file: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Failed to parse manifest: {0}")]
    ParseError(#[from] serde_yaml::Error),
    
    #[error("Validation error: {message}")]
    ValidationError { message: String },
    
    #[error("Unsupported API version: {version}")]
    UnsupportedVersion { version: String },
    
    #[error("Invalid service configuration: {service} - {error}")]
    ServiceError { service: String, error: String },
    
    #[error("Resource constraint violation: {constraint}")]
    ResourceError { constraint: String },
    
    #[error("Security policy violation: {policy}")]
    SecurityError { policy: String },
}

/// Main biome manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    
    pub kind: String,
    
    pub metadata: BiomeMetadata,
    
    #[serde(default)]
    pub primals: HashMap<String, PrimalConfig>,
    
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
    
    #[serde(default)]
    pub federation: Option<FederationConfig>,
    
    #[serde(default)]
    pub resources: Option<ResourceConfig>,
    
    #[serde(default)]
    pub health_checks: Vec<HealthCheckConfig>,
    
    #[serde(default)]
    pub security: Option<SecurityConfig>,
    
    #[serde(default)]
    pub networking: Option<NetworkingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    pub name: String,
    
    #[serde(default)]
    pub namespace: Option<String>,
    
    #[serde(default)]
    pub labels: HashMap<String, String>,
    
    #[serde(default)]
    pub annotations: HashMap<String, String>,
    
    #[serde(default)]
    pub version: Option<String>,
    
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    pub enabled: bool,
    
    #[serde(default)]
    pub source: Option<String>,
    
    #[serde(default)]
    pub version: Option<String>,
    
    #[serde(default)]
    pub config: HashMap<String, serde_yaml::Value>,
    
    #[serde(default)]
    pub dependencies: Vec<String>,
    
    #[serde(default)]
    pub resources: Option<PrimalResourceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResourceConfig {
    #[serde(default)]
    pub cpu: Option<String>,
    
    #[serde(default)]
    pub memory: Option<String>,
    
    #[serde(default)]
    pub disk: Option<String>,
    
    #[serde(default)]
    pub network: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    
    #[serde(default = "default_runtime")]
    pub runtime: String,
    
    #[serde(default)]
    pub source: Option<String>,
    
    #[serde(default)]
    pub command: Option<Vec<String>>,
    
    #[serde(default)]
    pub args: Vec<String>,
    
    #[serde(default)]
    pub environment: HashMap<String, String>,
    
    #[serde(default)]
    pub ports: Vec<PortConfig>,
    
    #[serde(default)]
    pub volumes: Vec<VolumeConfig>,
    
    #[serde(default)]
    pub capabilities: Vec<String>,
    
    #[serde(default)]
    pub resources: Option<ServiceResourceConfig>,
    
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
    
    #[serde(default)]
    pub restart_policy: Option<RestartPolicy>,
    
    #[serde(default)]
    pub dependencies: Vec<String>,
}

fn default_runtime() -> String {
    "wasm".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    pub container_port: u16,
    
    #[serde(default)]
    pub host_port: Option<u16>,
    
    #[serde(default = "default_protocol")]
    pub protocol: String,
    
    #[serde(default)]
    pub name: Option<String>,
}

fn default_protocol() -> String {
    "tcp".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    pub name: String,
    pub mount_path: String,
    
    #[serde(default)]
    pub host_path: Option<String>,
    
    #[serde(default)]
    pub read_only: bool,
    
    #[serde(default)]
    pub volume_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResourceConfig {
    #[serde(default)]
    pub cpu_limit: Option<String>,
    
    #[serde(default)]
    pub memory_limit: Option<String>,
    
    #[serde(default)]
    pub cpu_request: Option<String>,
    
    #[serde(default)]
    pub memory_request: Option<String>,
    
    #[serde(default)]
    pub disk_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub enabled: bool,
    
    #[serde(default)]
    pub trust_policy: Option<String>,
    
    #[serde(default)]
    pub peers: Vec<String>,
    
    #[serde(default)]
    pub discovery: Option<DiscoveryConfig>,
    
    #[serde(default)]
    pub security: Option<FederationSecurityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub enabled: bool,
    
    #[serde(default)]
    pub method: Option<String>,
    
    #[serde(default)]
    pub interval: Option<u64>,
    
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSecurityConfig {
    #[serde(default)]
    pub tls_enabled: bool,
    
    #[serde(default)]
    pub cert_path: Option<String>,
    
    #[serde(default)]
    pub key_path: Option<String>,
    
    #[serde(default)]
    pub ca_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    #[serde(default)]
    pub cpu_limit: Option<String>,
    
    #[serde(default)]
    pub memory_limit: Option<String>,
    
    #[serde(default)]
    pub disk_limit: Option<String>,
    
    #[serde(default)]
    pub network_limit: Option<String>,
    
    #[serde(default)]
    pub gpu_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub name: String,
    
    #[serde(rename = "type")]
    pub check_type: String,
    
    #[serde(default)]
    pub command: Option<Vec<String>>,
    
    #[serde(default)]
    pub http: Option<HttpHealthCheck>,
    
    #[serde(default)]
    pub tcp: Option<TcpHealthCheck>,
    
    #[serde(default = "default_interval")]
    pub interval: u64,
    
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    #[serde(default = "default_retries")]
    pub retries: u32,
    
    #[serde(default)]
    pub initial_delay: Option<u64>,
}

fn default_interval() -> u64 { 30 }
fn default_timeout() -> u64 { 5 }
fn default_retries() -> u32 { 3 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHealthCheck {
    pub path: String,
    
    #[serde(default = "default_http_port")]
    pub port: u16,
    
    #[serde(default)]
    pub headers: HashMap<String, String>,
    
    #[serde(default)]
    pub expected_status: Option<u16>,
}

fn default_http_port() -> u16 { 80 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpHealthCheck {
    pub port: u16,
    
    #[serde(default)]
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub sandbox_enabled: bool,
    
    #[serde(default)]
    pub capabilities: Vec<String>,
    
    #[serde(default)]
    pub seccomp_profile: Option<String>,
    
    #[serde(default)]
    pub apparmor_profile: Option<String>,
    
    #[serde(default)]
    pub selinux_context: Option<String>,
    
    #[serde(default)]
    pub user_id: Option<u32>,
    
    #[serde(default)]
    pub group_id: Option<u32>,
    
    #[serde(default)]
    pub read_only_root: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingConfig {
    #[serde(default)]
    pub mode: Option<String>,
    
    #[serde(default)]
    pub bridge_name: Option<String>,
    
    #[serde(default)]
    pub dns: Vec<String>,
    
    #[serde(default)]
    pub dns_search: Vec<String>,
    
    #[serde(default)]
    pub hostname: Option<String>,
    
    #[serde(default)]
    pub domain_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    #[serde(default = "default_restart_policy")]
    pub policy: String,
    
    #[serde(default)]
    pub max_retries: Option<u32>,
    
    #[serde(default)]
    pub delay: Option<u64>,
}

fn default_restart_policy() -> String {
    "on-failure".to_string()
}

/// Manifest loader and validator
pub struct ManifestLoader {
    strict_validation: bool,
}

impl ManifestLoader {
    pub fn new(strict_validation: bool) -> Self {
        Self { strict_validation }
    }

    /// Load and parse a biome manifest from file
    pub async fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<BiomeManifest, ManifestError> {
        let content = tokio::fs::read_to_string(path).await?;
        self.load_from_string(&content).await
    }

    /// Load and parse a biome manifest from string
    pub async fn load_from_string(&self, content: &str) -> Result<BiomeManifest, ManifestError> {
        let manifest: BiomeManifest = serde_yaml::from_str(content)?;
        
        // Validate the manifest
        self.validate(&manifest).await?;
        
        Ok(manifest)
    }

    /// Validate a biome manifest
    pub async fn validate(&self, manifest: &BiomeManifest) -> Result<(), ManifestError> {
        // Check API version
        if !self.is_supported_version(&manifest.api_version) {
            return Err(ManifestError::UnsupportedVersion {
                version: manifest.api_version.clone(),
            });
        }

        // Check kind
        if manifest.kind != "Biome" {
            return Err(ManifestError::ValidationError {
                message: format!("Unsupported kind: {}", manifest.kind),
            });
        }

        // Validate metadata
        self.validate_metadata(&manifest.metadata)?;

        // Validate services
        for service in &manifest.services {
            self.validate_service(service)?;
        }

        // Validate primals
        for (name, primal) in &manifest.primals {
            self.validate_primal(name, primal)?;
        }

        // Validate federation config
        if let Some(federation) = &manifest.federation {
            self.validate_federation(federation)?;
        }

        // Validate resource constraints
        if let Some(resources) = &manifest.resources {
            self.validate_resources(resources)?;
        }

        // Validate health checks
        for health_check in &manifest.health_checks {
            self.validate_health_check(health_check)?;
        }

        Ok(())
    }

    fn is_supported_version(&self, version: &str) -> bool {
        matches!(version, "biomeOS/v1" | "toadstool/v1" | "v1")
    }

    fn validate_metadata(&self, metadata: &BiomeMetadata) -> Result<(), ManifestError> {
        if metadata.name.is_empty() {
            return Err(ManifestError::ValidationError {
                message: "Biome name cannot be empty".to_string(),
            });
        }

        // Validate name format (DNS-1123 compliant)
        if !metadata.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.') {
            return Err(ManifestError::ValidationError {
                message: "Biome name must be DNS-1123 compliant".to_string(),
            });
        }

        Ok(())
    }

    fn validate_service(&self, service: &ServiceConfig) -> Result<(), ManifestError> {
        if service.name.is_empty() {
            return Err(ManifestError::ServiceError {
                service: "unknown".to_string(),
                error: "Service name cannot be empty".to_string(),
            });
        }

        // Validate runtime
        if !self.is_supported_runtime(&service.runtime) {
            return Err(ManifestError::ServiceError {
                service: service.name.clone(),
                error: format!("Unsupported runtime: {}", service.runtime),
            });
        }

        // Validate ports
        for port in &service.ports {
            if port.container_port == 0 {
                return Err(ManifestError::ServiceError {
                    service: service.name.clone(),
                    error: "Container port cannot be 0".to_string(),
                });
            }
        }

        // Validate capabilities
        for capability in &service.capabilities {
            if !self.is_valid_capability(capability) {
                return Err(ManifestError::ServiceError {
                    service: service.name.clone(),
                    error: format!("Invalid capability: {}", capability),
                });
            }
        }

        Ok(())
    }

    fn validate_primal(&self, name: &str, primal: &PrimalConfig) -> Result<(), ManifestError> {
        if name.is_empty() {
            return Err(ManifestError::ValidationError {
                message: "Primal name cannot be empty".to_string(),
            });
        }

        // Validate known primals
        if self.strict_validation && !self.is_known_primal(name) {
            return Err(ManifestError::ValidationError {
                message: format!("Unknown primal: {}", name),
            });
        }

        Ok(())
    }

    fn validate_federation(&self, federation: &FederationConfig) -> Result<(), ManifestError> {
        if federation.enabled {
            if let Some(trust_policy) = &federation.trust_policy {
                if !self.is_valid_trust_policy(trust_policy) {
                    return Err(ManifestError::ValidationError {
                        message: format!("Invalid trust policy: {}", trust_policy),
                    });
                }
            }
        }

        Ok(())
    }

    fn validate_resources(&self, resources: &ResourceConfig) -> Result<(), ManifestError> {
        // Validate resource format (e.g., "100m", "1Gi")
        if let Some(cpu) = &resources.cpu_limit {
            if !self.is_valid_resource_spec(cpu) {
                return Err(ManifestError::ResourceError {
                    constraint: format!("Invalid CPU limit: {}", cpu),
                });
            }
        }

        if let Some(memory) = &resources.memory_limit {
            if !self.is_valid_resource_spec(memory) {
                return Err(ManifestError::ResourceError {
                    constraint: format!("Invalid memory limit: {}", memory),
                });
            }
        }

        Ok(())
    }

    fn validate_health_check(&self, health_check: &HealthCheckConfig) -> Result<(), ManifestError> {
        match health_check.check_type.as_str() {
            "http" => {
                if health_check.http.is_none() {
                    return Err(ManifestError::ValidationError {
                        message: format!("HTTP health check '{}' missing http configuration", health_check.name),
                    });
                }
            }
            "tcp" => {
                if health_check.tcp.is_none() {
                    return Err(ManifestError::ValidationError {
                        message: format!("TCP health check '{}' missing tcp configuration", health_check.name),
                    });
                }
            }
            "command" => {
                if health_check.command.is_none() {
                    return Err(ManifestError::ValidationError {
                        message: format!("Command health check '{}' missing command", health_check.name),
                    });
                }
            }
            _ => {
                return Err(ManifestError::ValidationError {
                    message: format!("Invalid health check type: {}", health_check.check_type),
                });
            }
        }

        Ok(())
    }

    fn is_supported_runtime(&self, runtime: &str) -> bool {
        matches!(runtime, "wasm" | "container" | "native" | "python")
    }

    fn is_valid_capability(&self, capability: &str) -> bool {
        // Basic capability validation
        capability.starts_with("network.") || 
        capability.starts_with("fs.") || 
        capability.starts_with("sys.") ||
        capability == "all"
    }

    fn is_known_primal(&self, name: &str) -> bool {
        matches!(name, "beardog" | "nestgate" | "songbird" | "squirrel" | "toadstool")
    }

    fn is_valid_trust_policy(&self, policy: &str) -> bool {
        matches!(policy, "beardog_verified" | "manual" | "auto" | "strict")
    }

    fn is_valid_resource_spec(&self, spec: &str) -> bool {
        // Basic resource specification validation
        let re = regex::Regex::new(r"^\d+(\.\d+)?[mMgGtTkK]?i?$").unwrap();
        re.is_match(spec)
    }
}

/// Runtime information for a biome
#[derive(Debug, Clone)]
pub struct BiomeRuntime {
    pub id: Uuid,
    pub name: String,
    pub manifest: BiomeManifest,
    pub status: BiomeStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub stopped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiomeStatus {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Unknown,
}

impl BiomeRuntime {
    pub fn new(manifest: BiomeManifest) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: manifest.metadata.name.clone(),
            manifest,
            status: BiomeStatus::Created,
            created_at: chrono::Utc::now(),
            started_at: None,
            stopped_at: None,
            pid: None,
            exit_code: None,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, BiomeStatus::Running)
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self.status, BiomeStatus::Stopped | BiomeStatus::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manifest_loading() {
        let yaml = r#"
apiVersion: biomeOS/v1
kind: Biome
metadata:
  name: test-biome
  labels:
    environment: test
services:
  - name: web-service
    runtime: wasm
    ports:
      - container_port: 8080
        host_port: 8080
        protocol: tcp
"#;

        let loader = ManifestLoader::new(false);
        let manifest = loader.load_from_string(yaml).await.unwrap();
        
        assert_eq!(manifest.metadata.name, "test-biome");
        assert_eq!(manifest.services.len(), 1);
        assert_eq!(manifest.services[0].name, "web-service");
        assert_eq!(manifest.services[0].runtime, "wasm");
    }

    #[tokio::test]
    async fn test_manifest_validation() {
        let yaml = r#"
apiVersion: biomeOS/v1
kind: Biome
metadata:
  name: ""
services: []
"#;

        let loader = ManifestLoader::new(true);
        let result = loader.load_from_string(yaml).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ManifestError::ValidationError { .. }));
    }

    #[test]
    fn test_biome_runtime_creation() {
        let manifest = BiomeManifest {
            api_version: "biomeOS/v1".to_string(),
            kind: "Biome".to_string(),
            metadata: BiomeMetadata {
                name: "test-biome".to_string(),
                namespace: None,
                labels: HashMap::new(),
                annotations: HashMap::new(),
                version: None,
                description: None,
            },
            primals: HashMap::new(),
            services: Vec::new(),
            federation: None,
            resources: None,
            health_checks: Vec::new(),
            security: None,
            networking: None,
        };

        let runtime = BiomeRuntime::new(manifest);
        assert_eq!(runtime.name, "test-biome");
        assert!(matches!(runtime.status, BiomeStatus::Created));
        assert!(!runtime.is_running());
        assert!(!runtime.is_stopped());
    }
} 