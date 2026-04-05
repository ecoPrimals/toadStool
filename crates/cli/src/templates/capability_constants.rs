// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-Based Template Constants
//!
//! Modern, Deep Debt-compliant constants that use capabilities instead of primal names.
//!
//! # Philosophy
//!
//! **Old approach** (hardcoded primals):
//! ```rust,ignore
//! let storage = ServiceDependency::Primal("nestgate");  // ❌ Who
//! ```
//!
//! **New approach** (capability-based):
//! ```rust,ignore
//! let storage = ServiceDependency::Capability("storage");  // ✅ What
//! ```
//!
//! This allows ANY provider to satisfy the dependency, not just a specific primal.

use serde::{Deserialize, Serialize};

/// Capability types for service dependencies
///
/// Use these instead of hardcoded primal names!
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityDependency {
    /// Security capabilities (encryption, signing, key management)
    ///
    /// Could be satisfied by: any security/PKI provider, HSM, cloud KMS, local keyring, etc.
    Security,

    /// Storage capabilities (persistence, compression, versioning)
    ///
    /// Could be satisfied by: any storage provider, S3, Azure Blob, local filesystem, etc.
    Storage,

    /// Coordination capabilities (service mesh, discovery, orchestration)
    ///
    /// Could be satisfied by: any coordination layer, Kubernetes, Consul, etcd, etc.
    Coordination,

    /// AI/ML capabilities (inference, training, natural language)
    ///
    /// Could be satisfied by: any intelligence/ML provider, OpenAI, local models, Hugging Face, etc.
    Intelligence,

    /// Compute capabilities (CPU, GPU, specialized hardware)
    ///
    /// Could be satisfied by: local compute, cloud instances, edge devices, etc.
    Compute,

    /// Monitoring capabilities (metrics, logging, tracing)
    ///
    /// Could be satisfied by: Prometheus, Grafana, Datadog, etc.
    Monitoring,

    /// Networking capabilities (routing, tunneling, VPN)
    ///
    /// Could be satisfied by: Envoy, Istio, Cilium, etc.
    Networking,
}

impl CapabilityDependency {
    /// Convert to dependency string for templates
    ///
    /// Format: "capability:{type}"
    ///
    /// # Examples
    /// ```
    /// use toadstool_cli::templates::capability_constants::CapabilityDependency;
    ///
    /// assert_eq!(
    ///     CapabilityDependency::Security.to_dependency_string(),
    ///     "capability:security"
    /// );
    /// ```
    pub fn to_dependency_string(&self) -> String {
        match self {
            Self::Security => "capability:security",
            Self::Storage => "capability:storage",
            Self::Coordination => "capability:coordination",
            Self::Intelligence => "capability:intelligence",
            Self::Compute => "capability:compute",
            Self::Monitoring => "capability:monitoring",
            Self::Networking => "capability:networking",
        }
        .to_string()
    }

    /// Get human-readable description
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Security => "Security and cryptography services",
            Self::Storage => "Persistent storage and data management",
            Self::Coordination => "Service coordination and discovery",
            Self::Intelligence => "AI, ML, and natural language processing",
            Self::Compute => "Computational resources (CPU/GPU)",
            Self::Monitoring => "Monitoring, metrics, and observability",
            Self::Networking => "Networking, routing, and connectivity",
        }
    }
}

/// Capability features for fine-grained dependency specification
pub mod capabilities {
    /// Security capability features
    pub mod security {
        /// Encryption feature
        pub const ENCRYPTION: &str = "capability:security:encryption";
        /// Digital signing feature
        pub const SIGNING: &str = "capability:security:signing";
        /// Key management feature
        pub const KEY_MANAGEMENT: &str = "capability:security:key-management";
        /// PKI feature
        pub const PKI: &str = "capability:pki";
        /// Audit logging feature
        pub const AUDIT: &str = "capability:security:audit";
    }

    /// Storage capability features
    pub mod storage {
        /// Persistent storage feature
        pub const PERSISTENCE: &str = "capability:storage:persistence";
        /// Compression feature
        pub const COMPRESSION: &str = "capability:storage:compression";
        /// Versioning feature
        pub const VERSIONING: &str = "capability:storage:versioning";
        /// Backup feature
        pub const BACKUP: &str = "capability:storage:backup";
        /// Replication feature
        pub const REPLICATION: &str = "capability:storage:replication";
    }

    /// Coordination capability features
    pub mod coordination {
        /// Service mesh feature
        pub const SERVICE_MESH: &str = "capability:coordination:service-mesh";
        /// Service discovery feature
        pub const DISCOVERY: &str = "capability:coordination:discovery";
        /// Load balancing feature
        pub const LOAD_BALANCING: &str = "capability:coordination:load-balancing";
        /// Health check feature
        pub const HEALTH_CHECK: &str = "capability:coordination:health-check";
    }

    /// Intelligence capability features
    pub mod intelligence {
        /// Inference feature
        pub const INFERENCE: &str = "capability:intelligence:inference";
        /// Training feature
        pub const TRAINING: &str = "capability:intelligence:training";
        /// NLP feature
        pub const NLP: &str = "capability:intelligence:nlp";
        /// Vision feature
        pub const VISION: &str = "capability:intelligence:vision";
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_dependency_string() {
        assert_eq!(
            CapabilityDependency::Security.to_dependency_string(),
            "capability:security"
        );
        assert_eq!(
            CapabilityDependency::Storage.to_dependency_string(),
            "capability:storage"
        );
    }

    #[test]
    fn test_capability_features() {
        assert_eq!(
            capabilities::security::ENCRYPTION,
            "capability:security:encryption"
        );
        assert_eq!(
            capabilities::storage::COMPRESSION,
            "capability:storage:compression"
        );
    }

    #[test]
    fn test_capability_descriptions() {
        assert!(!CapabilityDependency::Security.description().is_empty());
        assert!(!CapabilityDependency::Intelligence.description().is_empty());
    }
}
