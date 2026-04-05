// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

use super::discovery_traits::DiscoveryError;
use serde::{Deserialize, Serialize};

/// Substrate capability detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubstrateCapability {
    /// Container orchestration (k8s, nomad, etc.)
    ContainerOrchestration,

    /// Container runtime (docker, podman, containerd)
    ContainerRuntime,

    /// Service mesh (consul, linkerd, istio)
    ServiceMesh,

    /// Service discovery (consul, etcd, zookeeper)
    ServiceDiscovery,

    /// Cloud compute (AWS, GCP, Azure, etc.)
    CloudCompute,

    /// Bare metal / no orchestration
    BareMetal,
}

/// Detected substrate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedSubstrate {
    /// Type of substrate detected
    pub substrate_type: SubstrateType,

    /// Capabilities this substrate provides
    pub capabilities: Vec<SubstrateCapability>,

    /// Substrate-specific metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Types of substrates (detected, not hardcoded!)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubstrateType {
    /// Container orchestrator detected (could be k8s, nomad, etc.)
    ContainerOrchestrator,

    /// Container runtime detected (docker, podman, etc.)
    ContainerRuntime,

    /// Cloud environment detected
    Cloud,

    /// Bare metal / direct execution
    Bare,
}

impl DetectedSubstrate {
    /// Check if substrate has a capability
    #[must_use]
    pub fn has_capability(&self, capability: &SubstrateCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get substrate-specific metadata value
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Substrate detection trait - implemented by specific detectors
///
/// Migrated from `async_trait` to native async for zero-cost abstraction.
pub trait SubstrateDetector: Send + Sync {
    /// Try to detect this substrate type
    fn detect(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>>
                + Send
                + '_,
        >,
    >;

    /// Name of this detector (for logging)
    fn name(&self) -> &str;
}
