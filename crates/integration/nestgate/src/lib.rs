// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]

//! # `ToadStool` `NestGate` Integration
//!
//! Integration module for connecting `ToadStool` with `NestGate` storage and data management.
//! `NestGate` provides intelligent storage orchestration, data pipelines, and distributed
//! cache management for the universal compute ecosystem.
//!
//! ## Features
//!
//! - **Artifact Storage**: Secure storage and retrieval of execution artifacts
//! - **Data Pipeline Management**: Automated data processing workflows
//! - **Distributed File System**: Access to `NestGate`'s distributed storage
//! - **Metadata Management**: Rich metadata support for stored assets
//! - **Cache Optimization**: Intelligent caching strategies
//!
//! ## Usage
//!
//! ```ignore
//! use toadstool_integration_nestgate::{NestGateClient, ArtifactType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Prefer: NestGateClient::discover().await? (capability-based)
//! // Or set NESTGATE_ENDPOINT / TOADSTOOL_STORAGE_ENDPOINT env var
//! let client = NestGateClient::connect(
//!     std::env::var("NESTGATE_ENDPOINT")
//!         .unwrap_or_else(|_| toadstool_common::constants::network::http_url(
//!             toadstool_common::constants::network::DEFAULT_HOSTNAME,
//!             toadstool_common::constants::network::HEALTH_CHECK_PORT,
//!         ))
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
    EncryptionType, NestGateError, NestGateResult, StorageInfo, StorageTier, VersionInfo,
};

pub use config::{CacheConfig, NestGateConfig, StoragePreferences};

pub use pipeline::{
    InputType, OutputType, PipelineConfig, PipelineExecutionStatus, PipelineInput, PipelineOutput,
    PipelineProgress, PipelineResources, PipelineSchedule, PipelineStatus, PipelineStep,
    ScheduleType, StepExecutionStatus, StepStatus, StepType,
};

// Capability-based storage client (vendor-agnostic!)
pub use client::StorageClient;

// Legacy export for compatibility
pub use client::StorageClient as NestGateClient;

// Tests module
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_nestgate_config_default() {
        let config = NestGateConfig::default();
        // Uses environment-aware configuration (TOADSTOOL_NESTGATE_PORT defaults to 8082)
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
