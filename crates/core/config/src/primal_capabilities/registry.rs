// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lookup, matching, and endpoint construction for [`super::parsing::PrimalCapabilitiesRegistry`].

use super::parsing::{CapabilityError, CapabilityResult, PrimalCapabilitiesRegistry};
use std::collections::HashMap;

impl PrimalCapabilitiesRegistry {
    /// Find primal names that have a specific capability
    #[must_use]
    pub fn find_by_capability(&self, capability: &str) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| def.capabilities.iter().any(|c| c == capability))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Find primals that have ALL of the specified capabilities
    #[must_use]
    pub fn find_by_capabilities(&self, capabilities: &[&str]) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| {
                capabilities
                    .iter()
                    .all(|cap| def.capabilities.contains(&(*cap).to_string()))
            })
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Find primals by role
    #[must_use]
    pub fn find_by_role(&self, role: &str) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| def.primary_role == role)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get primal definition
    #[must_use]
    pub fn get_primal(&self, name: &str) -> Option<&super::parsing::PrimalDefinition> {
        self.primals.get(name)
    }

    /// Get endpoint for a primal
    ///
    /// Constructs endpoint from host and `default_port`
    /// In production, this should query actual service discovery (mDNS, Consul, etc.)
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityError`] if the primal is not found.
    pub fn get_endpoint(&self, primal_name: &str, host: &str) -> CapabilityResult<String> {
        let primal = self
            .primals
            .get(primal_name)
            .ok_or_else(|| CapabilityError::PrimalNotFound(primal_name.to_string()))?;

        // In development/local: use host:port
        // In production: should use real service discovery
        let protocol = if primal.protocols.iter().any(|p| p == "http") {
            "http"
        } else {
            "https"
        };

        Ok(format!("{protocol}://{}:{}", host, primal.default_port))
    }

    /// Get migration fallback URL (deprecated)
    #[deprecated(note = "Use capability discovery instead of migration fallbacks")]
    #[must_use]
    pub fn get_migration_fallback(&self, primal_name: &str) -> Option<&str> {
        self.migration
            .get(primal_name)
            .map(|m| m.fallback_url.as_str())
    }

    /// Get all primals with their endpoints
    ///
    /// Returns a map of `primal_name` -> endpoint
    #[must_use]
    pub fn get_all_endpoints(&self, host: &str) -> HashMap<String, String> {
        self.primals
            .iter()
            .map(|(name, primal)| {
                let protocol = if primal.protocols.iter().any(|p| p == "http") {
                    "http"
                } else {
                    "https"
                };
                (
                    name.clone(),
                    format!("{protocol}://{}:{}", host, primal.default_port),
                )
            })
            .collect()
    }
}

/// Helper function to get self-knowledge (Toadstool's own capabilities)
///
/// This is the ONLY place where hardcoding is acceptable:
/// **"Know thyself"** - a primal should know its own capabilities
#[must_use]
pub fn get_self_capabilities(
    registry: &PrimalCapabilitiesRegistry,
) -> Option<&super::parsing::PrimalDefinition> {
    let self_name = toadstool_common::constants::primal_identity::PRIMAL_NAME;
    registry.get_primal(self_name)
}
