// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Capability-to-typical-provider mapping (human-readable reference only).
//!
//! This module maps each capability id to a short **capability-based** description of
//! which *kind* of service usually satisfies it. Use `infant_discovery::capabilities`
//! for capability constants and discovery.

use crate::infant_discovery::capabilities::capabilities::{
    AI_PROCESSING, AUTHENTICATION, AUTHORIZATION, CACHE, EVENT_STREAM, KEY_VALUE_STORE,
    LOAD_BALANCING, MESSAGE_QUEUE, MONITORING, NLP, ORCHESTRATION, PKI, SEARCH, SECRETS,
    SERVICE_MESH, STORAGE, TRACING,
};

/// Typical provider category for a capability (for documentation/reference only).
///
/// Returns a short capability-based label (for example `"security service"`,
/// `"coordination service"`). **NOTE**: This is for reference only; discovery must be
/// capability-based at runtime, not name-based.
///
/// Production code should discover providers at runtime via `infant_discovery` instead
/// of relying on this static mapping.
#[must_use]
#[deprecated(
    since = "0.92.0",
    note = "Use capability-based discovery via infant_discovery instead of static primal-name mappings"
)]
#[expect(clippy::match_same_arms)] // Intentionally separate for documentation and extensibility
pub fn capability_typical_provider(capability: &str) -> Option<&'static str> {
    match capability {
        // Crypto & security capability
        PKI => Some("security service"),
        SECRETS => Some("security service"),
        AUTHENTICATION => Some("security service"),
        AUTHORIZATION => Some("security service"),

        // Coordination / orchestration capability
        ORCHESTRATION => Some("coordination service"),
        SERVICE_MESH => Some("coordination service"),
        LOAD_BALANCING => Some("coordination service"),

        // Storage capability
        STORAGE => Some("storage service"),
        KEY_VALUE_STORE => Some("storage service"),
        CACHE => Some("storage service"),
        SEARCH => Some("storage service"),

        // Intelligence / routing workload capability
        AI_PROCESSING => Some("intelligence service"),
        NLP => Some("intelligence service"),

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
    #[expect(deprecated)]
    fn test_capability_provider_mapping() {
        assert_eq!(capability_typical_provider(PKI), Some("security service"));
        assert_eq!(
            capability_typical_provider(ORCHESTRATION),
            Some("coordination service")
        );
        assert_eq!(
            capability_typical_provider(STORAGE),
            Some("storage service")
        );
        assert_eq!(
            capability_typical_provider(AI_PROCESSING),
            Some("intelligence service")
        );
        assert_eq!(capability_typical_provider(MONITORING), Some("BiomeOS"));
        assert_eq!(capability_typical_provider("unknown_capability"), None);
    }
}
