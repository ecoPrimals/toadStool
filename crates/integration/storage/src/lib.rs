// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! # `ToadStool` `Storage` Integration
//!
//! Integration module for connecting `ToadStool` with `Storage` storage and data management.
//! `Storage` provides intelligent storage orchestration, data pipelines, and distributed
//! cache management for the universal compute ecosystem.
//!
//! ## Features
//!
//! - **Artifact Storage**: Secure storage and retrieval of execution artifacts
//! - **Data Pipeline Management**: Automated data processing workflows
//! - **Distributed File System**: Access to `Storage`'s distributed storage
//! - **Metadata Management**: Rich metadata support for stored assets
//! - **Cache Optimization**: Intelligent caching strategies
//!
//! ## Usage
//!
//! ```ignore
//! use toadstool_integration_storage::{StorageClient, ArtifactType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Prefer: StorageClient::discover().await? (capability-based)
//! // Or set TOADSTOOL_STORAGE_ENDPOINT (legacy: NESTGATE_ENDPOINT)
//! let client = StorageClient::connect(
//!     std::env::var("TOADSTOOL_STORAGE_ENDPOINT").expect("set storage endpoint or use StorageClient::discover")
//! ).await?;
//!
//! // Store an artifact
//! let metadata = client.store_artifact(
//!     "my-artifact",
//!     b"Hello, world!",
//!     ArtifactType::Binary,
//! ).await?;
//!
//! // Retrieve the artifact
//! let data = client.retrieve_artifact("my-artifact").await?;
//! # Ok(())
//! # }
//! ```

// Module declarations
mod artifacts;
pub mod client;
pub mod config;
pub mod pipeline;
mod pipelines;
pub mod types;
mod utils;

// Re-export core types and functionality
pub use types::{
    ArtifactFilters, ArtifactMetadata, ArtifactType, CachedArtifact, CompressionType,
    EncryptionType, StorageError, StorageInfo, StorageServiceResult, StorageTier, VersionInfo,
};

pub use config::{CacheConfig, StorageConfig, StoragePreferences};

pub use pipeline::{
    InputType, OutputType, PipelineConfig, PipelineExecutionStatus, PipelineInput, PipelineOutput,
    PipelineProgress, PipelineResources, PipelineSchedule, PipelineStatus, PipelineStep,
    ScheduleType, StepExecutionStatus, StepStatus, StepType,
};

// Capability-based storage client (vendor-agnostic!)
pub use client::StorageClient;

// Tests module
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_storage_ext_config_default() {
        let config = StorageConfig::default();
        // Uses environment-aware configuration (storage service port defaults near 8082)
        assert!(config.endpoint.starts_with("http://"));
        assert!(config.endpoint.contains(":808")); // Port in 8080-8089 range
        assert!(config.cache.as_ref().is_some_and(|c| c.enabled));
        // Future enhancement: Add storage preferences configuration
    }

    #[test]
    fn test_artifact_metadata_creation() {
        let metadata = ArtifactMetadata {
            id: "test-artifact".to_string(),
            artifact_type: ArtifactType::ExecutionOutput,
            content_type: "application/json".to_string(),
            size_bytes: 1024,
            checksum: "abc123".to_string(),
            created_at: std::time::SystemTime::now(),
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
