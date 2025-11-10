//! # toadStool - Universal Runtime for ecoPrimals
//!
//! Docker-free, sovereignty-focused container orchestration with WASM-first execution.
//! 
//! ## Features
//! 
//! - **WASM-first execution**: WebAssembly as the primary runtime with container fallback
//! - **Capability-based security**: Fine-grained permissions and sandboxing
//! - **Federation support**: Peer-to-peer networking and resource sharing
//! - **Resource management**: Comprehensive resource allocation and monitoring
//! - **biomeOS integration**: Native support for biome.yaml manifests
//! - **Multi-runtime support**: WASM, containers, native processes, and Python
//! 
//! ## Quick Start
//! 
//! ```rust,no_run
//! use toadstool::*;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load a biome manifest
//!     let manifest = manifest::ManifestLoader::new(false)
//!         .load_from_file("biome.yaml")
//!         .await?;
//!     
//!     // Create a scheduler
//!     let scheduler = scheduler::WorkloadScheduler::new().await?;
//!     
//!     // Create a biome runtime
//!     let biome_runtime = manifest::BiomeRuntime::new(manifest);
//!     
//!     // Schedule the biome
//!     scheduler.schedule_biome(&biome_runtime).await?;
//!     
//!     // Wait for completion
//!     let exit_code = scheduler.wait_for_biome(biome_runtime.id).await?;
//!     println!("Biome completed with exit code: {}", exit_code);
//!     
//!     Ok(())
//! }
//! ```
//! 
//! ## Architecture
//! 
//! toadStool is built around several core components:
//! 
//! - **Manifest System**: Parses and validates biome.yaml files
//! - **Scheduler**: Manages workload scheduling and execution
//! - **Runtime Manager**: Handles different execution runtimes
//! - **Security Manager**: Enforces capability-based security
//! - **Resource Manager**: Manages system resources and allocation
//! - **Federation Manager**: Handles peer-to-peer networking
//! 
//! ## Security Model
//! 
//! toadStool uses a capability-based security model where services must explicitly
//! declare their required capabilities:
//! 
//! ```yaml
//! services:
//!   - name: web-service
//!     runtime: wasm
//!     capabilities:
//!       - "network.client"
//!       - "fs.read:/app/data"
//!       - "fs.write:/tmp"
//! ```

pub mod manifest;
pub mod scheduler;
pub mod runtimes;
pub mod security;
pub mod resources;
pub mod federation;
pub mod cli;

// Re-export commonly used types
pub use manifest::{
    BiomeManifest, BiomeRuntime, BiomeStatus, ManifestLoader, ManifestError,
    ServiceConfig, PrimalConfig, FederationConfig, ResourceConfig,
};

pub use scheduler::{
    WorkloadScheduler, SchedulerError, ScheduledTask, TaskStatus, LogEntry,
};

pub use runtimes::{
    RuntimeManager, RuntimeError, ExecutionResult, RuntimeResourceUsage,
};

pub use security::{
    SecurityManager, SecurityError, SecurityContext, Capability, 
    SecurityOperation, SecurityStats,
};

pub use resources::{
    ResourceManager, ResourceError, ResourceAllocation, ResourceUsage,
    SystemInfo, ResourceAllocationSummary,
};

pub use federation::{
    FederationManager, FederationError, FederationStatus, PeerInfo,
    FederationMessage, PeerStatus, TrustLevel,
};

pub use cli::{
    CliHandler, CliError,
};

/// toadStool version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Default configuration values
pub mod defaults {
    use std::time::Duration;
    
    /// Default CPU allocation for services without explicit limits
    pub const DEFAULT_CPU_ALLOCATION: f64 = 0.1;
    
    /// Default memory allocation for services without explicit limits (128MB)
    pub const DEFAULT_MEMORY_ALLOCATION: u64 = 128 * 1024 * 1024;
    
    /// Default disk allocation for services without explicit limits (1GB)
    pub const DEFAULT_DISK_ALLOCATION: u64 = 1024 * 1024 * 1024;
    
    /// Default task timeout
    pub const DEFAULT_TASK_TIMEOUT: Duration = Duration::from_secs(300);
    
    /// Default heartbeat interval for federation
    pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
    
    /// Default resource monitoring interval
    pub const DEFAULT_MONITORING_INTERVAL: Duration = Duration::from_secs(30);
    
    /// Maximum concurrent tasks per runtime
    pub const MAX_CONCURRENT_TASKS: usize = 100;
    
    /// Default federation port
    pub const DEFAULT_FEDERATION_PORT: u16 = 7777;
    
    /// Default trust policy
    pub const DEFAULT_TRUST_POLICY: &str = "beardog_verified";
}

/// Common error types used throughout toadStool
pub mod errors {
    pub use crate::manifest::ManifestError;
    pub use crate::scheduler::SchedulerError;
    pub use crate::runtimes::RuntimeError;
    pub use crate::security::SecurityError;
    pub use crate::resources::ResourceError;
    pub use crate::federation::FederationError;
    pub use crate::cli::CliError;
}

/// Utility functions and helpers
pub mod utils {
    use std::time::Duration;
    
    /// Parse a duration string (e.g., "30s", "5m", "1h")
    pub fn parse_duration(duration_str: &str) -> Result<Duration, String> {
        let duration_str = duration_str.trim();
        
        if duration_str.is_empty() {
            return Err("Duration string cannot be empty".to_string());
        }
        
        let (value_str, unit) = if duration_str.ends_with("ms") {
            (&duration_str[..duration_str.len()-2], "ms")
        } else if duration_str.ends_with('s') {
            (&duration_str[..duration_str.len()-1], "s")
        } else if duration_str.ends_with('m') {
            (&duration_str[..duration_str.len()-1], "m")
        } else if duration_str.ends_with('h') {
            (&duration_str[..duration_str.len()-1], "h")
        } else {
            (duration_str, "s") // Default to seconds
        };
        
        let value: u64 = value_str.parse()
            .map_err(|_| format!("Invalid duration value: {}", value_str))?;
        
        let duration = match unit {
            "ms" => Duration::from_millis(value),
            "s" => Duration::from_secs(value),
            "m" => Duration::from_secs(value * 60),
            "h" => Duration::from_secs(value * 3600),
            _ => return Err(format!("Unknown duration unit: {}", unit)),
        };
        
        Ok(duration)
    }
    
    /// Format a duration as a human-readable string
    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        
        if total_seconds < 60 {
            format!("{}s", total_seconds)
        } else if total_seconds < 3600 {
            let minutes = total_seconds / 60;
            let seconds = total_seconds % 60;
            if seconds == 0 {
                format!("{}m", minutes)
            } else {
                format!("{}m{}s", minutes, seconds)
            }
        } else {
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            
            let mut result = format!("{}h", hours);
            if minutes > 0 {
                result.push_str(&format!("{}m", minutes));
            }
            if seconds > 0 {
                result.push_str(&format!("{}s", seconds));
            }
            result
        }
    }
    
    /// Validate a service name according to DNS-1123 rules
    pub fn validate_service_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Service name cannot be empty".to_string());
        }
        
        if name.len() > 63 {
            return Err("Service name cannot be longer than 63 characters".to_string());
        }
        
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.') {
            return Err("Service name must contain only alphanumeric characters, hyphens, and dots".to_string());
        }
        
        if name.starts_with('-') || name.ends_with('-') {
            return Err("Service name cannot start or end with a hyphen".to_string());
        }
        
        if name.starts_with('.') || name.ends_with('.') {
            return Err("Service name cannot start or end with a dot".to_string());
        }
        
        Ok(())
    }
    
    /// Generate a unique identifier for a biome or service
    pub fn generate_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Check if a port is in the valid range
    pub fn validate_port(port: u16) -> Result<(), String> {
        if port == 0 {
            return Err("Port cannot be 0".to_string());
        }
        
        if port < 1024 {
            return Err("Port must be >= 1024 (non-privileged ports only)".to_string());
        }
        
        Ok(())
    }
}

/// Integration helpers for biomeOS
pub mod biomeos {
    use crate::manifest::{BiomeManifest, ManifestLoader};
    use std::path::Path;
    
    /// Load a biome manifest from a biomeOS-compatible file
    pub async fn load_biome_manifest<P: AsRef<Path>>(path: P) -> Result<BiomeManifest, crate::ManifestError> {
        let loader = ManifestLoader::new(false);
        loader.load_from_file(path).await
    }
    
    /// Validate a biome manifest for biomeOS compatibility
    pub async fn validate_biome_manifest(manifest: &BiomeManifest) -> Result<(), crate::ManifestError> {
        let loader = ManifestLoader::new(true);
        loader.validate(manifest).await
    }
    
    /// Convert a biome manifest to biomeOS format
    pub fn to_biomeos_format(manifest: &BiomeManifest) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(manifest)
    }
    
    /// Parse a biome manifest from biomeOS format
    pub fn from_biomeos_format(yaml: &str) -> Result<BiomeManifest, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }
}

/// Testing utilities
#[cfg(test)]
pub mod test_utils {
    use crate::manifest::{BiomeManifest, BiomeMetadata, ServiceConfig};
    use std::collections::HashMap;
    
    /// Create a minimal test biome manifest
    pub fn create_test_manifest(name: &str) -> BiomeManifest {
        BiomeManifest {
            api_version: "biomeOS/v1".to_string(),
            kind: "Biome".to_string(),
            metadata: BiomeMetadata {
                name: name.to_string(),
                namespace: None,
                labels: HashMap::new(),
                annotations: HashMap::new(),
                version: Some("1.0.0".to_string()),
                description: Some("Test biome".to_string()),
            },
            primals: HashMap::new(),
            services: vec![
                ServiceConfig {
                    name: "test-service".to_string(),
                    runtime: "wasm".to_string(),
                    source: None,
                    command: None,
                    args: Vec::new(),
                    environment: HashMap::new(),
                    ports: Vec::new(),
                    volumes: Vec::new(),
                    capabilities: vec!["fs.read:/tmp".to_string()],
                    resources: None,
                    health_check: None,
                    restart_policy: None,
                    dependencies: Vec::new(),
                }
            ],
            federation: None,
            resources: None,
            health_checks: Vec::new(),
            security: None,
            networking: None,
        }
    }
    
    /// Create a test service configuration
    pub fn create_test_service(name: &str, runtime: &str) -> ServiceConfig {
        ServiceConfig {
            name: name.to_string(),
            runtime: runtime.to_string(),
            source: None,
            command: None,
            args: Vec::new(),
            environment: HashMap::new(),
            ports: Vec::new(),
            volumes: Vec::new(),
            capabilities: Vec::new(),
            resources: None,
            health_check: None,
            restart_policy: None,
            dependencies: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "toadstool");
        assert!(!DESCRIPTION.is_empty());
    }
    
    #[test]
    fn test_duration_parsing() {
        assert_eq!(utils::parse_duration("30s").unwrap(), std::time::Duration::from_secs(30));
        assert_eq!(utils::parse_duration("5m").unwrap(), std::time::Duration::from_secs(300));
        assert_eq!(utils::parse_duration("1h").unwrap(), std::time::Duration::from_secs(3600));
        assert_eq!(utils::parse_duration("500ms").unwrap(), std::time::Duration::from_millis(500));
        
        assert!(utils::parse_duration("").is_err());
        assert!(utils::parse_duration("invalid").is_err());
    }
    
    #[test]
    fn test_duration_formatting() {
        assert_eq!(utils::format_duration(std::time::Duration::from_secs(30)), "30s");
        assert_eq!(utils::format_duration(std::time::Duration::from_secs(90)), "1m30s");
        assert_eq!(utils::format_duration(std::time::Duration::from_secs(3661)), "1h1m1s");
        assert_eq!(utils::format_duration(std::time::Duration::from_secs(3600)), "1h");
    }
    
    #[test]
    fn test_service_name_validation() {
        assert!(utils::validate_service_name("valid-service").is_ok());
        assert!(utils::validate_service_name("service.name").is_ok());
        assert!(utils::validate_service_name("service123").is_ok());
        
        assert!(utils::validate_service_name("").is_err());
        assert!(utils::validate_service_name("-invalid").is_err());
        assert!(utils::validate_service_name("invalid-").is_err());
        assert!(utils::validate_service_name(".invalid").is_err());
        assert!(utils::validate_service_name("invalid.").is_err());
        assert!(utils::validate_service_name("invalid_service").is_err());
    }
    
    #[test]
    fn test_port_validation() {
        assert!(utils::validate_port(8080).is_ok());
        assert!(utils::validate_port(1024).is_ok());
        assert!(utils::validate_port(65535).is_ok());
        
        assert!(utils::validate_port(0).is_err());
        assert!(utils::validate_port(80).is_err());
        assert!(utils::validate_port(443).is_err());
    }
    
    #[test]
    fn test_id_generation() {
        let id1 = utils::generate_id();
        let id2 = utils::generate_id();
        
        assert_ne!(id1, id2);
        assert!(uuid::Uuid::parse_str(&id1).is_ok());
        assert!(uuid::Uuid::parse_str(&id2).is_ok());
    }
    
    #[tokio::test]
    async fn test_biomeos_integration() {
        let manifest = test_utils::create_test_manifest("test-biome");
        
        // Test serialization
        let yaml = biomeos::to_biomeos_format(&manifest).unwrap();
        assert!(yaml.contains("apiVersion: biomeOS/v1"));
        assert!(yaml.contains("kind: Biome"));
        assert!(yaml.contains("name: test-biome"));
        
        // Test deserialization
        let parsed_manifest = biomeos::from_biomeos_format(&yaml).unwrap();
        assert_eq!(parsed_manifest.metadata.name, "test-biome");
        assert_eq!(parsed_manifest.api_version, "biomeOS/v1");
        assert_eq!(parsed_manifest.kind, "Biome");
    }
    
    #[test]
    fn test_test_utils() {
        let manifest = test_utils::create_test_manifest("test");
        assert_eq!(manifest.metadata.name, "test");
        assert_eq!(manifest.services.len(), 1);
        assert_eq!(manifest.services[0].name, "test-service");
        
        let service = test_utils::create_test_service("test-svc", "wasm");
        assert_eq!(service.name, "test-svc");
        assert_eq!(service.runtime, "wasm");
    }
    
    // Additional comprehensive tests - Sprint 14
    
    #[test]
    fn test_version_string_format() {
        assert_eq!(VERSION.trim(), VERSION);
        assert!(VERSION.contains('.'));
    }
    
    #[test]
    fn test_default_allocations_sanity() {
        assert!(defaults::DEFAULT_CPU_ALLOCATION > 0.0);
        assert!(defaults::DEFAULT_CPU_ALLOCATION <= 1.0);
        assert!(defaults::DEFAULT_MEMORY_ALLOCATION > 0);
        assert!(defaults::DEFAULT_DISK_ALLOCATION > defaults::DEFAULT_MEMORY_ALLOCATION);
    }
    
    #[test]
    fn test_default_timeouts_sanity() {
        assert!(defaults::DEFAULT_TASK_TIMEOUT.as_secs() > 0);
        assert!(defaults::DEFAULT_HEARTBEAT_INTERVAL.as_secs() > 0);
        assert!(defaults::DEFAULT_MONITORING_INTERVAL.as_secs() > 0);
    }
    
    #[test]
    fn test_max_concurrent_tasks_valid() {
        assert_eq!(defaults::MAX_CONCURRENT_TASKS, 100);
        assert!(defaults::MAX_CONCURRENT_TASKS > 0);
    }
    
    #[test]
    fn test_default_federation_port_valid() {
        assert!(defaults::DEFAULT_FEDERATION_PORT > 1024);
    }
    
    #[test]
    fn test_duration_parsing_edge_cases() {
        assert_eq!(utils::parse_duration("0s").unwrap(), Duration::from_secs(0));
        assert_eq!(utils::parse_duration("  30s  ").unwrap(), Duration::from_secs(30));
        assert!(utils::parse_duration("30x").is_err());
    }
    
    #[test]
    fn test_duration_formatting_edge_cases() {
        assert_eq!(utils::format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(utils::format_duration(Duration::from_secs(3660)), "1h1m");
    }
    
    #[test]
    fn test_service_name_boundary_conditions() {
        let max_valid = "a".repeat(63);
        assert!(utils::validate_service_name(&max_valid).is_ok());
        
        let too_long = "a".repeat(64);
        assert!(utils::validate_service_name(&too_long).is_err());
    }
    
    #[test]
    fn test_port_validation_boundaries() {
        assert!(utils::validate_port(1023).is_err());
        assert!(utils::validate_port(1024).is_ok());
        assert!(utils::validate_port(65535).is_ok());
    }
    
    #[test]
    fn test_id_generation_uniqueness() {
        let ids: Vec<String> = (0..5).map(|_| utils::generate_id()).collect();
        for i in 0..ids.len() {
            for j in (i+1)..ids.len() {
                assert_ne!(ids[i], ids[j]);
            }
        }
    }
    
    #[test]
    fn test_id_generation_format() {
        let id = utils::generate_id();
        assert!(uuid::Uuid::parse_str(&id).is_ok());
        assert_eq!(id.len(), 36);
    }
    
    #[tokio::test]
    async fn test_biomeos_serialization_roundtrip() {
        let original = test_utils::create_test_manifest("roundtrip");
        let yaml = biomeos::to_biomeos_format(&original).unwrap();
        let parsed = biomeos::from_biomeos_format(&yaml).unwrap();
        
        assert_eq!(parsed.metadata.name, original.metadata.name);
        assert_eq!(parsed.api_version, original.api_version);
    }
    
    #[tokio::test]
    async fn test_biomeos_invalid_yaml() {
        let invalid = "this is not: valid:: yaml:::";
        assert!(biomeos::from_biomeos_format(invalid).is_err());
    }
    
    #[test]
    fn test_constants_not_empty() {
        assert!(!VERSION.is_empty());
        assert!(!NAME.is_empty());
        assert!(!DESCRIPTION.is_empty());
    }
    
    #[test]
    fn test_defaults_consistency() {
        assert!(defaults::DEFAULT_HEARTBEAT_INTERVAL <= defaults::DEFAULT_TASK_TIMEOUT);
        assert!(defaults::DEFAULT_MONITORING_INTERVAL <= defaults::DEFAULT_TASK_TIMEOUT);
    }
} 