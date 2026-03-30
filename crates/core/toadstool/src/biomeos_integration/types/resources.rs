// SPDX-License-Identifier: AGPL-3.0-only
//! Resource allocation, GPU, and health check configuration types.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Health check configuration for biome services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeHealthCheckConfig {
    /// Health check interval
    pub interval: Duration,
    /// Health check timeout
    pub timeout: Duration,
    /// Number of retries before marking as unhealthy
    pub retries: u32,
    /// Initial delay before first check (startup grace period)
    pub initial_delay: Duration,
}

impl Default for BiomeHealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            retries: 3,
            initial_delay: Duration::from_secs(30),
        }
    }
}

/// Token propagation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPropagationConfig {
    /// Enable cross-Primal token propagation
    pub enabled: bool,
    /// Token refresh interval
    pub refresh_interval: Duration,
    /// Token validation settings
    pub validation: TokenValidationConfig,
}

/// Token validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidationConfig {
    /// Require signature validation
    pub require_signature: bool,
    /// Timestamp validation window
    pub timestamp_window: Duration,
    /// Replay attack protection
    pub replay_protection: bool,
}

/// Volume configuration for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Volume name
    pub name: String,
    /// Volume size (e.g., "100Gi", "1TB")
    pub size: String,
    /// Storage class
    pub storage_class: Option<String>,
    /// Access modes
    pub access_modes: Vec<String>,
    /// Mount path
    pub mount_path: Option<String>,
    /// Backup policy
    pub backup_policy: Option<String>,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automated backups
    pub enabled: bool,
    /// Backup schedule (cron format)
    pub schedule: String,
    /// Retention policy
    pub retention: String,
    /// Backup destination
    pub destination: String,
}

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Enable replication
    pub enabled: bool,
    /// Replication factor
    pub factor: u32,
    /// Replication strategy
    pub strategy: String,
}

// Agent, Model, MCP, and Boot configurations moved to agent.rs module

/// Storage configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorageConfig {
    /// Enable storage
    pub enabled: bool,
    /// Storage backend type
    pub backend: String,
    /// Storage capacity
    pub capacity: Option<String>,
}

/// Primal resource allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResources {
    /// CPU cores allocation
    pub cpu_cores: Option<f64>,
    /// Memory allocation in GB
    pub memory_gb: Option<f64>,
    /// Storage allocation in GB
    pub storage_gb: Option<f64>,
    /// GPU allocation
    pub gpu: Option<GpuAllocation>,
    /// Network bandwidth
    pub network_bandwidth: Option<String>,
}

/// GPU allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuAllocation {
    /// Number of GPUs
    pub count: u32,
    /// GPU type/model
    pub gpu_type: Option<String>,
    /// GPU memory requirement
    pub memory: Option<String>,
}

/// Resource configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeResources {
    /// CPU limits
    pub cpu_limit: Option<f64>,
    /// Memory limits
    pub memory_limit: Option<String>,
    /// Storage limits
    pub storage_limit: Option<String>,
    /// GPU limits
    pub gpu_limit: Option<u32>,
    /// Network bandwidth
    pub network_bandwidth: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        BackupConfig, BiomeHealthCheckConfig, BiomeResources, BiomeStorageConfig, GpuAllocation,
        PrimalResources, ReplicationConfig, TokenPropagationConfig, TokenValidationConfig,
        VolumeConfig,
    };

    #[test]
    fn biome_health_check_default_and_serde_roundtrip() {
        let d = BiomeHealthCheckConfig::default();
        assert_eq!(d.interval, Duration::from_secs(30));
        assert_eq!(d.timeout, Duration::from_secs(10));
        assert_eq!(d.retries, 3);
        assert_eq!(d.initial_delay, Duration::from_secs(30));

        let json = serde_json::to_string(&d).expect("serialize");
        let back: BiomeHealthCheckConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.interval, d.interval);
        assert_eq!(back.timeout, d.timeout);
        assert_eq!(back.retries, d.retries);
        assert_eq!(back.initial_delay, d.initial_delay);
        assert!(format!("{d:?}").contains("BiomeHealthCheckConfig"));
    }

    #[test]
    fn token_propagation_and_validation_serde_roundtrip_clone() {
        let tv = TokenValidationConfig {
            require_signature: true,
            timestamp_window: Duration::from_secs(300),
            replay_protection: true,
        };
        let tp = TokenPropagationConfig {
            enabled: true,
            refresh_interval: Duration::from_secs(600),
            validation: tv.clone(),
        };

        let json = serde_json::to_string(&tp).expect("serialize tp");
        let back: TokenPropagationConfig = serde_json::from_str(&json).expect("deserialize tp");
        assert_eq!(back.enabled, tp.enabled);
        assert_eq!(back.refresh_interval, tp.refresh_interval);
        assert_eq!(back.validation.require_signature, tv.require_signature);
        assert_eq!(back.validation.replay_protection, tv.replay_protection);

        let cloned = tp.clone();
        assert_eq!(cloned.validation.timestamp_window, tv.timestamp_window);
        assert!(matches!(format!("{tp:?}"), s if s.contains("TokenPropagationConfig")));
    }

    #[test]
    fn volume_config_serde_roundtrip() {
        let v = VolumeConfig {
            name: "data".to_string(),
            size: "100Gi".to_string(),
            storage_class: Some("fast".to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: Some("/data".to_string()),
            backup_policy: Some("daily".to_string()),
        };
        let json = serde_json::to_string(&v).expect("serialize");
        let back: VolumeConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, v.name);
        assert_eq!(back.access_modes, v.access_modes);
        assert_eq!(back.mount_path, v.mount_path);
    }

    #[test]
    fn backup_and_replication_config_serde_roundtrip() {
        let b = BackupConfig {
            enabled: true,
            schedule: "0 2 * * *".to_string(),
            retention: "30d".to_string(),
            destination: "s3://bucket/backups".to_string(),
        };
        let r = ReplicationConfig {
            enabled: false,
            factor: 3,
            strategy: "raft".to_string(),
        };

        let jb = serde_json::to_string(&b).unwrap();
        let bb: BackupConfig = serde_json::from_str(&jb).unwrap();
        assert_eq!(bb.schedule, b.schedule);

        let jr = serde_json::to_string(&r).unwrap();
        let rr: ReplicationConfig = serde_json::from_str(&jr).unwrap();
        assert_eq!(rr.factor, r.factor);
        assert!(!rr.enabled);
    }

    #[test]
    fn biome_storage_config_serde_roundtrip() {
        let s = BiomeStorageConfig {
            enabled: true,
            backend: "local".to_string(),
            capacity: Some("500Gi".to_string()),
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: BiomeStorageConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.backend, s.backend);
        assert_eq!(back.capacity, s.capacity);
    }

    #[test]
    fn primal_resources_and_gpu_serde_roundtrip() {
        let gpu = GpuAllocation {
            count: 2,
            gpu_type: Some("H100".to_string()),
            memory: Some("80GB".to_string()),
        };
        let pr = PrimalResources {
            cpu_cores: Some(8.0),
            memory_gb: Some(64.0),
            storage_gb: Some(500.0),
            gpu: Some(gpu.clone()),
            network_bandwidth: Some("10Gbps".to_string()),
        };

        let json = serde_json::to_string(&pr).expect("serialize");
        let back: PrimalResources = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.cpu_cores, pr.cpu_cores);
        assert_eq!(back.gpu.as_ref().unwrap().count, 2);
        assert_eq!(
            back.gpu.as_ref().unwrap().gpu_type,
            Some("H100".to_string())
        );

        let jg = serde_json::to_string(&gpu).expect("serialize gpu");
        let gg: GpuAllocation = serde_json::from_str(&jg).expect("deserialize gpu");
        assert_eq!(gg.count, gpu.count);
    }

    #[test]
    fn biome_resources_serde_roundtrip_optional_fields() {
        let br = BiomeResources {
            cpu_limit: Some(4.0),
            memory_limit: Some("8Gi".to_string()),
            storage_limit: None,
            gpu_limit: Some(1),
            network_bandwidth: None,
        };
        let json = serde_json::to_string(&br).expect("serialize");
        let back: BiomeResources = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.cpu_limit, Some(4.0));
        assert!(matches!(back.gpu_limit, Some(1)));
        assert!(back.storage_limit.is_none());
    }
}
