// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! Legacy primal name to capability mapping (MIGRATION ONLY)
//!
//! This module provides TEMPORARY helpers for migrating from hardcoded
//! primal names to capability-based discovery.
//!
//! **DO NOT USE IN NEW CODE** - Use `infant_discovery::capabilities` directly.

use crate::infant_discovery::capabilities::capabilities::{
    AI_PROCESSING, AUTHENTICATION, AUTHORIZATION, CACHE, EVENT_STREAM, KEY_VALUE_STORE,
    LOAD_BALANCING, MESSAGE_QUEUE, MONITORING, NLP, ORCHESTRATION, PKI, SEARCH, SECRETS,
    SERVICE_MESH, STORAGE, TRACING,
};

/// Map legacy primal names to their primary capabilities
///
/// # Migration Helper
///
/// This function exists ONLY to help migrate old hardcoded code.
/// New code should use capability constants directly.
///
/// # Example
/// ```ignore
/// // OLD (being migrated away from):
/// // let beardog = BeardogClient::new("http://localhost:8080");
///
/// // TEMPORARY migration step:
/// let caps = legacy_primal_to_capabilities("beardog");
/// let service = discovery.discover(caps[0]).await?;
///
/// // FINAL (what all code should look like):
/// use toadstool_common::infant_discovery::capabilities::PKI;
/// let service = discovery.discover(PKI).await?;
/// ```
#[deprecated(
    since = "0.7.0",
    note = "Use infant_discovery::capabilities directly. This is a migration helper only."
)]
#[must_use]
pub fn legacy_primal_to_capabilities(primal: &str) -> Vec<&'static str> {
    match primal.to_lowercase().as_str() {
        "beardog" => vec![
            PKI,            // Primary: Certificate authority
            SECRETS,        // Secret management
            AUTHENTICATION, // Authentication services
            AUTHORIZATION,  // Authorization/access control
        ],
        "songbird" => vec![
            ORCHESTRATION,  // Primary: Service orchestration
            SERVICE_MESH,   // Service mesh coordination
            LOAD_BALANCING, // Load balancing
        ],
        "nestgate" => vec![
            STORAGE,         // Primary: Persistent storage
            KEY_VALUE_STORE, // Key-value storage
            CACHE,           // Caching services
        ],
        "squirrel" => vec![
            AI_PROCESSING, // Primary: AI processing
            NLP,           // Natural language processing
        ],
        "biomeos" => vec![
            MONITORING, // Primary: Monitoring
            TRACING,    // Distributed tracing
        ],
        _ => vec![],
    }
}

/// Get primary capability for a legacy primal name
///
/// Returns the most important capability provided by this primal.
#[deprecated(since = "0.7.0", note = "Use infant_discovery::capabilities directly")]
#[must_use]
#[allow(deprecated)]
pub fn legacy_primal_primary_capability(primal: &str) -> Option<&'static str> {
    legacy_primal_to_capabilities(primal).first().copied()
}

/// Standard capability-to-primal mapping (for documentation/migration reference only)
///
/// Shows which primal typically provides each capability in the ecoPrimals ecosystem.
/// **NOTE**: This is for reference only! Discovery should be capability-based, not name-based.
#[must_use]
#[allow(clippy::match_same_arms)] // Intentionally separate for documentation and extensibility
pub fn capability_typical_provider(capability: &str) -> Option<&'static str> {
    match capability {
        // Crypto & Security (BearDog)
        PKI => Some("BearDog"),
        SECRETS => Some("BearDog"),
        AUTHENTICATION => Some("BearDog"),
        AUTHORIZATION => Some("BearDog"),

        // Orchestration (Songbird)
        ORCHESTRATION => Some("Songbird"),
        SERVICE_MESH => Some("Songbird"),
        LOAD_BALANCING => Some("Songbird"),

        // Storage (NestGate)
        STORAGE => Some("NestGate"),
        KEY_VALUE_STORE => Some("NestGate"),
        CACHE => Some("NestGate"),
        SEARCH => Some("NestGate"),

        // AI (Squirrel)
        AI_PROCESSING => Some("Squirrel"),
        NLP => Some("Squirrel"),

        // Observability (BiomeOS)
        MONITORING => Some("BiomeOS"),
        TRACING => Some("BiomeOS"),

        // Message Queue (could be multiple providers)
        MESSAGE_QUEUE => Some("TBD"),
        EVENT_STREAM => Some("TBD"),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn test_legacy_primal_capabilities() {
        let beardog_caps = legacy_primal_to_capabilities("beardog");
        assert!(!beardog_caps.is_empty());
        assert_eq!(beardog_caps[0], PKI); // Primary capability

        let songbird_caps = legacy_primal_to_capabilities("songbird");
        assert!(!songbird_caps.is_empty());
        assert_eq!(songbird_caps[0], ORCHESTRATION);

        let nestgate_caps = legacy_primal_to_capabilities("nestgate");
        assert!(!nestgate_caps.is_empty());
        assert_eq!(nestgate_caps[0], STORAGE);
    }

    #[test]
    #[allow(deprecated)]
    fn test_legacy_primary_capability() {
        assert_eq!(legacy_primal_primary_capability("beardog"), Some(PKI));
        assert_eq!(
            legacy_primal_primary_capability("songbird"),
            Some(ORCHESTRATION)
        );
        assert_eq!(legacy_primal_primary_capability("nestgate"), Some(STORAGE));
        assert_eq!(
            legacy_primal_primary_capability("squirrel"),
            Some(AI_PROCESSING)
        );
        assert_eq!(
            legacy_primal_primary_capability("biomeos"),
            Some(MONITORING)
        );
        assert_eq!(legacy_primal_primary_capability("unknown"), None);
    }

    #[test]
    fn test_capability_provider_mapping() {
        assert_eq!(capability_typical_provider(PKI), Some("BearDog"));
        assert_eq!(capability_typical_provider(ORCHESTRATION), Some("Songbird"));
        assert_eq!(capability_typical_provider(STORAGE), Some("NestGate"));
        assert_eq!(capability_typical_provider(AI_PROCESSING), Some("Squirrel"));
        assert_eq!(capability_typical_provider(MONITORING), Some("BiomeOS"));
        assert_eq!(capability_typical_provider("unknown_capability"), None);
    }

    #[test]
    #[allow(deprecated)]
    fn test_case_insensitive() {
        let caps1 = legacy_primal_to_capabilities("beardog");
        let caps2 = legacy_primal_to_capabilities("BearDog");
        let caps3 = legacy_primal_to_capabilities("BEARDOG");

        assert_eq!(caps1, caps2);
        assert_eq!(caps2, caps3);
    }
}
