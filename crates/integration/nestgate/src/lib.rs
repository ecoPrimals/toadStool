//! # ToadStool NestGate Integration
//!
//! Integration module for connecting ToadStool with NestGate storage and data management.
//! NestGate provides intelligent storage orchestration, data pipelines, and distributed
//! file system capabilities for the ecoPrimals ecosystem.
//!
//! ## Features
//!
//! - **Storage Orchestration**: Automatic storage provisioning and management
//! - **Data Pipeline Integration**: Stream processing and ETL capabilities  
//! - **Distributed File System**: Access to NestGate's distributed storage
//! - **Data Versioning**: Version control for datasets and artifacts
//! - **Caching Layer**: Intelligent caching for frequently accessed data
//! - **Security Integration**: Unified authentication with ecosystem services
//!
//! ## Quick Start
//!
//! ```rust
//! use toadstool_nestgate::{NestGateClient, NestGateConfig, ArtifactType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to NestGate
//!     let client = NestGateClient::connect("http://nestgate:4040").await?;
//!     
//!     // Store execution artifacts
//!     let metadata = client.store_artifact(
//!         "execution-123",
//!         b"result data",
//!         ArtifactType::ExecutionOutput
//!     ).await?;
//!     
//!     // Retrieve stored data
//!     let data = client.retrieve_artifact("execution-123").await?;
//!     
//!     Ok(())
//! }
//! ```

// Module declarations
pub mod client;
pub mod config;
pub mod pipeline;
pub mod types;

// Re-export core types and functionality
pub use types::{
    ArtifactFilters, ArtifactMetadata, ArtifactType, CachedArtifact, CompressionType,
    EncryptionType, NestGateError, NestGateResult, StorageInfo, StorageTier, VersionInfo,
};

pub use config::{CacheConfig, NestGateConfig, StoragePreferences};

pub use pipeline::{
    InputType, OutputType, PipelineConfig, PipelineExecutionStatus, PipelineInput, PipelineOutput,
    PipelineProgress, PipelineResources, PipelineSchedule, PipelineStatus, PipelineStep,
    ScheduleType, StepExecutionStatus, StepStatus, StepType,
};

pub use client::NestGateClient;

// Tests module
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_nestgate_config_default() {
        let config = NestGateConfig::default();
        assert_eq!(config.endpoint, "http://localhost:4040");
        assert!(config.cache_config.enabled);
        assert_eq!(config.storage_preferences.replication_factor, 3);
    }

    #[test]
    fn test_artifact_metadata_creation() {
        let metadata = ArtifactMetadata {
            id: "test-artifact".to_string(),
            artifact_type: ArtifactType::ExecutionOutput,
            content_type: "application/json".to_string(),
            size_bytes: 1024,
            checksum: "abc123".to_string(),
            created_at: Utc::now(),
            last_accessed: None,
            tags: HashMap::new(),
            execution_id: Some(Uuid::new_v4()),
            storage_info: StorageInfo {
                node_id: "node1".to_string(),
                path: "/test/path".to_string(),
                tier: StorageTier::Standard,
                replicated: true,
                compression: CompressionType::Auto,
                encryption: EncryptionType::Default,
            },
            version: None,
        };

        assert_eq!(metadata.id, "test-artifact");
        assert_eq!(metadata.size_bytes, 1024);
        assert!(matches!(
            metadata.artifact_type,
            ArtifactType::ExecutionOutput
        ));
    }

    #[test]
    fn test_pipeline_config_creation() {
        let config = PipelineConfig {
            pipeline_id: "test-pipeline".to_string(),
            name: "Test Pipeline".to_string(),
            inputs: vec![],
            outputs: vec![],
            steps: vec![],
            schedule: None,
            resources: None,
        };

        assert_eq!(config.pipeline_id, "test-pipeline");
        assert_eq!(config.name, "Test Pipeline");
        assert!(config.inputs.is_empty());
    }

    #[test]
    fn test_storage_preferences_default() {
        let prefs = StoragePreferences::default();
        assert!(matches!(prefs.storage_tier, StorageTier::Standard));
        assert_eq!(prefs.replication_factor, 3);
        assert!(matches!(prefs.compression, CompressionType::Auto));
        assert!(matches!(prefs.encryption, EncryptionType::Default));
    }
}
