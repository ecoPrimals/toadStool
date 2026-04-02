// SPDX-License-Identifier: AGPL-3.0-only
//! Capability-based template helpers
//!
//! This module provides helpers for capability-first service dependencies in templates.
//! Legacy primal names are accepted only as a compatibility shim.

use std::collections::HashMap;
use toadstool_common::constants::PRIMAL_NAME;
use toadstool_common::interned_strings::CapabilityDomain;
use toadstool_common::interned_strings::capabilities;
use toadstool_common::interned_strings::runtime_types;

/// Resolve a manifest or template dependency label to a canonical capability id.
///
/// Delegates to [`CapabilityDomain::from_label`] for legacy primal names and capability
/// strings, with special-cases for `biomeos` -> `"os"`.
#[must_use]
pub fn dependency_label_to_capability(label: &str) -> &'static str {
    if let Some(domain) = CapabilityDomain::from_label(label) {
        return domain.as_str();
    }
    match label.to_lowercase().as_str() {
        capabilities::SECURITY => capabilities::SECURITY,
        runtime_types::BIOMEOS => "os",
        _ => "unknown",
    }
}

/// Legacy alias for [`dependency_label_to_capability`].
#[deprecated(note = "Use dependency_label_to_capability — capabilities are primary.")]
#[must_use]
pub fn service_to_capability(service_name: &str) -> &'static str {
    dependency_label_to_capability(service_name)
}

/// Capability-first map: canonical capability id → optional legacy orchestrator label.
#[must_use]
pub fn get_capability_to_legacy_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert(capabilities::CRYPTO, "beardog");
    map.insert(capabilities::COORDINATION, "songbird");
    map.insert(capabilities::STORAGE, "nestgate");
    map.insert(capabilities::INTELLIGENCE, "squirrel");
    map.insert(capabilities::ROUTING, "squirrel");
    map.insert(capabilities::COMPUTE, PRIMAL_NAME);
    map.insert("os", runtime_types::BIOMEOS);
    map
}

/// Optional legacy orchestrator service name for a capability (compatibility only).
#[deprecated(note = "Prefer capability ids in manifests; primal names are not stable identities.")]
#[must_use]
pub fn legacy_service_name_for_capability(capability: &str) -> Option<&'static str> {
    get_capability_to_legacy_map().get(capability).copied()
}

/// Legacy: map capability → default service name string (inverse of [`dependency_label_to_capability`]).
#[deprecated(note = "Use capability ids; see get_capability_to_legacy_map if required.")]
#[must_use]
pub fn capability_to_service(capability: &str) -> &'static str {
    legacy_service_name_for_capability(capability).unwrap_or("unknown")
}

/// Convert legacy service dependencies to capability ids
///
/// # Example
/// ```
/// use toadstool_cli::templates::capability_helpers::dependencies_to_capabilities;
///
/// let deps = vec!["beardog".to_string(), "nestgate".to_string()];
/// let caps = dependencies_to_capabilities(&deps);
/// assert_eq!(caps, vec!["crypto", "storage"]);
/// ```
pub fn dependencies_to_capabilities(service_names: &[String]) -> Vec<&'static str> {
    service_names
        .iter()
        .map(|name| dependency_label_to_capability(name))
        .collect()
}

/// Convert capability ids to legacy service names for older orchestrators
pub fn capabilities_to_dependencies(capabilities: &[&str]) -> Vec<String> {
    let m = get_capability_to_legacy_map();
    capabilities
        .iter()
        .map(|cap| {
            m.get(cap)
                .map(|s| (*s).to_string())
                .unwrap_or_else(|| (*cap).to_string())
        })
        .collect()
}

/// Legacy: primal name → capability (use [`get_capability_to_legacy_map`] and invert).
#[deprecated(note = "Use get_capability_to_legacy_map (capability-first).")]
#[must_use]
pub fn get_capability_mappings() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert("beardog", capabilities::CRYPTO);
    map.insert("songbird", capabilities::COORDINATION);
    map.insert("nestgate", capabilities::STORAGE);
    map.insert("squirrel", capabilities::ROUTING);
    map.insert(PRIMAL_NAME, capabilities::COMPUTE);
    map.insert(runtime_types::BIOMEOS, "os");
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_label_to_capability() {
        assert_eq!(dependency_label_to_capability("beardog"), "crypto");
        assert_eq!(dependency_label_to_capability("crypto"), "crypto");
        assert_eq!(dependency_label_to_capability("songbird"), "coordination");
        assert_eq!(
            dependency_label_to_capability("coordination"),
            "coordination"
        );
        assert_eq!(dependency_label_to_capability("nestgate"), "storage");
        assert_eq!(dependency_label_to_capability("squirrel"), "routing");
        assert_eq!(dependency_label_to_capability("routing"), "routing");
    }

    #[test]
    #[allow(deprecated)]
    fn test_legacy_service_name_for_capability() {
        assert_eq!(
            legacy_service_name_for_capability("crypto"),
            Some("beardog")
        );
        assert_eq!(
            legacy_service_name_for_capability("coordination"),
            Some("songbird")
        );
        assert_eq!(
            legacy_service_name_for_capability("routing"),
            Some("squirrel")
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_capability_to_service() {
        assert_eq!(capability_to_service("crypto"), "beardog");
        assert_eq!(capability_to_service("coordination"), "songbird");
        assert_eq!(capability_to_service("storage"), "nestgate");
        assert_eq!(capability_to_service("routing"), "squirrel");
    }

    #[test]
    fn test_dependencies_conversion() {
        let deps = vec!["beardog".to_string(), "nestgate".to_string()];
        let caps = dependencies_to_capabilities(&deps);
        assert_eq!(caps, vec!["crypto", "storage"]);

        let back = capabilities_to_dependencies(&caps);
        assert_eq!(back, vec!["beardog", "nestgate"]);
    }

    #[test]
    fn test_get_capability_to_legacy_map() {
        let mappings = get_capability_to_legacy_map();
        assert_eq!(mappings.get("crypto"), Some(&"beardog"));
        assert_eq!(mappings.get("coordination"), Some(&"songbird"));
        assert!(mappings.len() >= 6);
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_capability_mappings() {
        let mappings = get_capability_mappings();
        assert_eq!(mappings.get("beardog"), Some(&"crypto"));
        assert_eq!(mappings.get("songbird"), Some(&"coordination"));
        assert!(mappings.len() >= 6);
    }
}
