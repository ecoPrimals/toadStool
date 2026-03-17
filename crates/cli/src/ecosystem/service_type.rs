// SPDX-License-Identifier: AGPL-3.0-only
//! Service type identification - capability-based replacement for hardcoded enums
//!
//! This module replaces the deprecated `EcosystemService` enum with a capability-based
//! approach where services are identified by their capabilities, not hardcoded names.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashSet;

use crate::ecosystem::capabilities::CapabilityId;

/// Service type identified by capabilities, not hardcoded names
///
/// This replaces the deprecated `EcosystemService` enum with a flexible
/// capability-based approach.
///
/// # Migration from `EcosystemService`
/// ```rust,ignore
/// // ❌ OLD: Hardcoded enum
/// let service_type = EcosystemService::BearDog;
/// match service_type {
///     EcosystemService::BearDog => handle_crypto(),
///     EcosystemService::Songbird => handle_coordination(),
///     EcosystemService::NestGate => handle_storage(),
///     _ => {}
/// }
///
/// // ✅ NEW: Capability-based
/// let service_type = ServiceType::from_capabilities(&capabilities);
/// if service_type.provides_crypto() {
///     handle_crypto();
/// }
/// if service_type.provides_coordination() {
///     handle_coordination();
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceType {
    /// Capabilities provided by this service
    #[serde(default)]
    capabilities: HashSet<CapabilityId>,

    /// Optional service name (for legacy compatibility)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    legacy_name: Option<String>,
}

impl ServiceType {
    /// Create a service type from capabilities
    #[must_use]
    pub const fn from_capabilities(capabilities: HashSet<CapabilityId>) -> Self {
        Self {
            capabilities,
            legacy_name: None,
        }
    }

    /// Create from capability list
    #[must_use]
    pub fn from_capability_list(capabilities: Vec<CapabilityId>) -> Self {
        Self {
            capabilities: capabilities.into_iter().collect(),
            legacy_name: None,
        }
    }

    /// Create with legacy name for backward compatibility
    ///
    /// Zero-Copy Optimization: Takes `&str` to avoid allocation at call site.
    #[must_use]
    pub fn with_legacy_name(mut self, name: &str) -> Self {
        self.legacy_name = Some(name.to_string());
        self
    }

    /// Check if service provides cryptographic capabilities
    #[must_use]
    pub fn provides_crypto(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.as_str().starts_with("crypto."))
    }

    /// Check if service provides coordination capabilities
    #[must_use]
    pub fn provides_coordination(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.as_str().starts_with("coordination."))
    }

    /// Check if service provides storage capabilities
    #[must_use]
    pub fn provides_storage(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.as_str().starts_with("storage."))
    }

    /// Check if service provides a specific capability
    #[must_use]
    pub fn has_capability(&self, capability: &CapabilityId) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get all capabilities
    #[must_use]
    pub const fn capabilities(&self) -> &HashSet<CapabilityId> {
        &self.capabilities
    }

    /// Get legacy name (if set)
    #[must_use]
    pub fn legacy_name(&self) -> Option<&str> {
        self.legacy_name.as_deref()
    }

    /// Get a display name for this service type (zero-copy when possible)
    ///
    /// Returns `Cow<str>` to avoid allocations for standard service types.
    /// Only allocates when using legacy names or custom capabilities with dots.
    #[must_use]
    pub fn display_name(&self) -> Cow<'_, str> {
        if let Some(name) = &self.legacy_name {
            return Cow::Borrowed(name);
        }

        // Generate name based on primary capability (zero-copy for standard types)
        if self.provides_crypto() {
            Cow::Borrowed("crypto-service")
        } else if self.provides_coordination() {
            Cow::Borrowed("coordination-service")
        } else if self.provides_storage() {
            Cow::Borrowed("storage-service")
        } else if let Some(first_cap) = self.capabilities.iter().next() {
            let cap_str = first_cap.as_str();
            if cap_str.contains('.') {
                Cow::Owned(cap_str.replace('.', "-"))
            } else {
                Cow::Borrowed(cap_str)
            }
        } else {
            Cow::Borrowed("unknown-service")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecosystem::capabilities::StandardCapability;

    #[test]
    fn test_service_type_crypto() {
        let capabilities = vec![
            StandardCapability::CryptoSignatureEd25519.id(),
            StandardCapability::CryptoEncryptionAes256.id(),
        ];
        let service_type = ServiceType::from_capability_list(capabilities);

        assert!(service_type.provides_crypto());
        assert!(!service_type.provides_coordination());
        assert!(!service_type.provides_storage());
    }

    #[test]
    fn test_service_type_coordination() {
        let capabilities = vec![
            StandardCapability::CoordinationServiceRegistry.id(),
            StandardCapability::CoordinationPeerDiscovery.id(),
        ];
        let service_type = ServiceType::from_capability_list(capabilities);

        assert!(!service_type.provides_crypto());
        assert!(service_type.provides_coordination());
        assert!(!service_type.provides_storage());
    }

    #[test]
    fn test_service_type_storage() {
        let capabilities = vec![
            StandardCapability::StorageDistributedFilesystem.id(),
            StandardCapability::StorageObjectS3.id(),
        ];
        let service_type = ServiceType::from_capability_list(capabilities);

        assert!(!service_type.provides_crypto());
        assert!(!service_type.provides_coordination());
        assert!(service_type.provides_storage());
    }

    #[test]
    fn test_service_type_with_legacy_name() {
        let service_type = ServiceType::default().with_legacy_name("beardog");

        assert_eq!(service_type.legacy_name(), Some("beardog"));
        assert_eq!(service_type.display_name(), "beardog");
    }

    #[test]
    fn test_service_type_display_name() {
        let capabilities = vec![StandardCapability::CryptoSignatureEd25519.id()];
        let service_type = ServiceType::from_capability_list(capabilities);

        assert_eq!(service_type.display_name(), "crypto-service");
    }

    #[test]
    fn test_has_specific_capability() {
        let cap = StandardCapability::CryptoSignatureEd25519.id();
        let capabilities = vec![cap.clone()];
        let service_type = ServiceType::from_capability_list(capabilities);

        assert!(service_type.has_capability(&cap));
        assert!(!service_type.has_capability(&StandardCapability::StorageObjectS3.id()));
    }
}
