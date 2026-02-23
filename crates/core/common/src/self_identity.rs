// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! Self-identity and capability announcement
//!
//! This module implements the **self-knowledge only** principle:
//! - Each primal knows only about itself
//! - Capabilities are announced, not hardcoded
//! - Other primals are discovered at runtime via capabilities
//! - Zero hardcoded ecosystem knowledge

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Self-identity: What this primal knows about itself
///
/// This is the ONLY place where a primal defines who it is.
/// No hardcoded knowledge of other primals exists.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfIdentity {
    /// Human-readable name (for logging/display only)
    display_name: String,

    /// Unique instance ID (generated at runtime)
    instance_id: String,

    /// Capabilities this instance provides
    capabilities: HashSet<String>,

    /// Version of this primal
    version: semver::Version,

    /// Optional metadata (for future extensibility)
    #[serde(default)]
    metadata: std::collections::HashMap<String, String>,
}

impl SelfIdentity {
    /// Create a new self-identity with the minimum required information
    ///
    /// # Philosophy
    /// This is the ONLY constructor. A primal must explicitly declare:
    /// - Who it is (display name)
    /// - What it can do (capabilities)
    /// - How to identify it (instance ID)
    ///
    /// It does NOT know about other primals. Discovery happens at runtime.
    #[must_use]
    pub fn new(
        display_name: impl Into<String>,
        instance_id: impl Into<String>,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            display_name: display_name.into(),
            instance_id: instance_id.into(),
            capabilities: capabilities.into_iter().map(Into::into).collect(),
            version: semver::Version::new(0, 1, 0),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Builder pattern for additional metadata
    #[must_use]
    pub fn with_version(mut self, version: semver::Version) -> Self {
        self.version = version;
        self
    }

    /// Add metadata key-value pair
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add a capability
    pub fn add_capability(&mut self, capability: impl Into<String>) {
        self.capabilities.insert(capability.into());
    }

    /// Remove a capability
    #[must_use]
    pub fn remove_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get display name (for logging/UI only)
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Get unique instance ID
    #[must_use]
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Check if this instance provides a capability
    #[must_use]
    pub fn provides(&self, capability: &str) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get all capabilities this instance provides
    #[must_use]
    pub const fn capabilities(&self) -> &HashSet<String> {
        &self.capabilities
    }

    /// Get version
    #[must_use]
    pub const fn version(&self) -> &semver::Version {
        &self.version
    }

    /// Get metadata
    #[must_use]
    pub const fn metadata(&self) -> &std::collections::HashMap<String, String> {
        &self.metadata
    }

    /// Get metadata value
    #[must_use]
    pub fn metadata_get(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Runtime-discovered service (not hardcoded)
///
/// This represents a service discovered via capability announcement.
/// We don't know what primal it is - we only know what it can do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Display name (provided by the service itself)
    pub display_name: String,

    /// Instance ID (unique identifier)
    pub instance_id: String,

    /// Capabilities it provides
    pub capabilities: HashSet<String>,

    /// Endpoint information (how to reach it)
    pub endpoints: Vec<ServiceEndpoint>,

    /// Version
    pub version: semver::Version,

    /// When it was discovered
    pub discovered_at: chrono::DateTime<chrono::Utc>,

    /// Last health check
    pub last_seen: chrono::DateTime<chrono::Utc>,

    /// Health status
    pub health: HealthStatus,
}

/// Service endpoint (protocol-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ServiceEndpoint {
    /// Protocol (http, grpc, websocket, etc.)
    pub protocol: String,

    /// URI (connection string)
    pub uri: String,

    /// Optional priority (lower = higher priority)
    #[serde(default)]
    pub priority: u32,
}

/// Health status of a discovered service
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum HealthStatus {
    /// Service is healthy and responding
    Healthy,

    /// Service is degraded but functional
    Degraded,

    /// Service is not responding
    Unhealthy,

    /// Service status is unknown
    #[default]
    Unknown,
}

/// Capability matcher for runtime discovery
///
/// This is how we find services WITHOUT hardcoding primal names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMatcher {
    /// Required capabilities (service must have ALL of these)
    #[serde(default)]
    pub required: HashSet<String>,

    /// Optional capabilities (service should have SOME of these)
    #[serde(default)]
    pub optional: HashSet<String>,

    /// Excluded capabilities (service must NOT have these)
    #[serde(default)]
    pub excluded: HashSet<String>,
}

impl CapabilityMatcher {
    /// Create a matcher for a single required capability
    #[must_use]
    pub fn requires(capability: impl Into<String>) -> Self {
        let mut required = HashSet::new();
        required.insert(capability.into());
        Self {
            required,
            optional: HashSet::new(),
            excluded: HashSet::new(),
        }
    }

    /// Create a matcher for multiple required capabilities
    #[must_use]
    pub fn requires_all(capabilities: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            required: capabilities.into_iter().map(Into::into).collect(),
            optional: HashSet::new(),
            excluded: HashSet::new(),
        }
    }

    /// Add optional capabilities
    #[must_use]
    pub fn with_optional(
        mut self,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.optional = capabilities.into_iter().map(Into::into).collect();
        self
    }

    /// Add excluded capabilities
    #[must_use]
    pub fn excluding(mut self, capabilities: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.excluded = capabilities.into_iter().map(Into::into).collect();
        self
    }

    /// Check if a service matches this matcher
    #[must_use]
    pub fn matches(&self, service_capabilities: &HashSet<String>) -> bool {
        // Must have all required capabilities
        if !self
            .required
            .iter()
            .all(|cap| service_capabilities.contains(cap))
        {
            return false;
        }

        // Must not have any excluded capabilities
        if self
            .excluded
            .iter()
            .any(|cap| service_capabilities.contains(cap))
        {
            return false;
        }

        true
    }

    /// Score a service match (0.0 = no match, 1.0 = perfect match)
    ///
    /// Takes into account required, optional, and excluded capabilities.
    #[must_use]
    pub fn score(&self, service_capabilities: &HashSet<String>) -> f64 {
        if !self.matches(service_capabilities) {
            return 0.0;
        }

        // Base score for matching all required capabilities
        let mut score = 0.7;

        // Bonus points for optional capabilities (up to 0.3)
        if self.optional.is_empty() {
            // If no optional capabilities specified, give full bonus
            score += 0.3;
        } else {
            let optional_matches = self
                .optional
                .iter()
                .filter(|cap| service_capabilities.contains(*cap))
                .count();
            // Note: count() is always small (< 100 typically), so precision loss is acceptable
            #[allow(clippy::cast_precision_loss)]
            let optional_ratio = optional_matches as f64 / self.optional.len() as f64;
            score += optional_ratio * 0.3;
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_identity_creation() {
        let identity = SelfIdentity::new(
            "ToadStool Runtime",
            "toadstool-instance-1",
            ["compute:execution", "compute:native"],
        );

        assert_eq!(identity.display_name(), "ToadStool Runtime");
        assert_eq!(identity.instance_id(), "toadstool-instance-1");
        assert!(identity.provides("compute:execution"));
        assert!(identity.provides("compute:native"));
        assert!(!identity.provides("storage:kv"));
    }

    #[test]
    fn test_capability_matcher_required() {
        let matcher = CapabilityMatcher::requires("pki");

        let mut capabilities = HashSet::new();
        capabilities.insert("pki".to_string());
        capabilities.insert("secrets".to_string());

        assert!(matcher.matches(&capabilities));
        assert!(matcher.score(&capabilities) > 0.9);
    }

    #[test]
    fn test_capability_matcher_excluded() {
        let matcher = CapabilityMatcher::requires("storage").excluding(["deprecated"]);

        let mut good_service = HashSet::new();
        good_service.insert("storage".to_string());

        let mut bad_service = HashSet::new();
        bad_service.insert("storage".to_string());
        bad_service.insert("deprecated".to_string());

        assert!(matcher.matches(&good_service));
        assert!(!matcher.matches(&bad_service));
    }

    #[test]
    fn test_capability_matcher_optional() {
        let matcher = CapabilityMatcher::requires("orchestration")
            .with_optional(["load-balancing", "service-mesh"]);

        let mut minimal = HashSet::new();
        minimal.insert("orchestration".to_string());

        let mut full = HashSet::new();
        full.insert("orchestration".to_string());
        full.insert("load-balancing".to_string());
        full.insert("service-mesh".to_string());

        assert!(matcher.matches(&minimal));
        assert!(matcher.matches(&full));
        assert!(matcher.score(&full) > matcher.score(&minimal));
    }
}
