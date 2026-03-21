// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Capability-to-primal reference mapping.
//!
//! This module provides a reference mapping of which primal typically provides
//! each capability in the ecoPrimals ecosystem. Use `infant_discovery::capabilities`
//! for capability constants and discovery.

use crate::infant_discovery::capabilities::capabilities::{
    AI_PROCESSING, AUTHENTICATION, AUTHORIZATION, CACHE, EVENT_STREAM, KEY_VALUE_STORE,
    LOAD_BALANCING, MESSAGE_QUEUE, MONITORING, NLP, ORCHESTRATION, PKI, SEARCH, SECRETS,
    SERVICE_MESH, STORAGE, TRACING,
};

/// Standard capability-to-primal mapping (for documentation/reference only).
///
/// Shows which primal typically provides each capability in the ecoPrimals ecosystem.
/// **NOTE**: This is for reference only! Discovery should be capability-based, not name-based.
///
/// Production code should discover providers at runtime via `infant_discovery` instead
/// of relying on this static mapping.
#[must_use]
#[deprecated(
    since = "0.92.0",
    note = "Use capability-based discovery via infant_discovery instead of static primal-name mappings"
)]
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
    #[allow(deprecated)]
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn test_capability_provider_mapping() {
        assert_eq!(capability_typical_provider(PKI), Some("BearDog"));
        assert_eq!(capability_typical_provider(ORCHESTRATION), Some("Songbird"));
        assert_eq!(capability_typical_provider(STORAGE), Some("NestGate"));
        assert_eq!(capability_typical_provider(AI_PROCESSING), Some("Squirrel"));
        assert_eq!(capability_typical_provider(MONITORING), Some("BiomeOS"));
        assert_eq!(capability_typical_provider("unknown_capability"), None);
    }
}
