// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based template helpers
//!
//! This module provides helpers for migrating from hardcoded service names
//! to capability-based service dependencies in templates.

use std::collections::HashMap;

/// Map service names to their primary capabilities
///
/// This is used during the migration period to translate between legacy
/// service names and capability-based discovery.
pub fn service_to_capability(service_name: &str) -> &'static str {
    match service_name.to_lowercase().as_str() {
        "beardog" => "pki",
        "songbird" => "orchestration",
        "nestgate" => "storage",
        "squirrel" => "ai",
        "toadstool" => "compute:execution",
        "biomeos" => "os",
        _ => "unknown",
    }
}

/// Map capabilities to default service names (for backward compatibility)
pub fn capability_to_service(capability: &str) -> &'static str {
    match capability {
        "pki" => "beardog",
        "orchestration" => "songbird",
        "storage" => "nestgate",
        "ai" => "squirrel",
        "compute:execution" => "toadstool",
        "os" => "biomeos",
        _ => "unknown",
    }
}

/// Convert legacy service dependencies to capability-based
///
/// # Example
/// ```
/// use toadstool_cli::templates::capability_helpers::dependencies_to_capabilities;
///
/// let deps = vec!["beardog".to_string(), "nestgate".to_string()];
/// let caps = dependencies_to_capabilities(&deps);
/// assert_eq!(caps, vec!["pki", "storage"]);
/// ```
pub fn dependencies_to_capabilities(service_names: &[String]) -> Vec<&'static str> {
    service_names
        .iter()
        .map(|name| service_to_capability(name))
        .collect()
}

/// Convert capability-based dependencies to legacy service names
///
/// Used for backward compatibility with orchestrators that expect service names.
pub fn capabilities_to_dependencies(capabilities: &[&str]) -> Vec<String> {
    capabilities
        .iter()
        .map(|cap| capability_to_service(cap).to_string())
        .collect()
}

/// Get all known capability mappings
pub fn get_capability_mappings() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert("beardog", "pki");
    map.insert("songbird", "orchestration");
    map.insert("nestgate", "storage");
    map.insert("squirrel", "ai");
    map.insert("toadstool", "compute:execution");
    map.insert("biomeos", "os");
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_to_capability() {
        assert_eq!(service_to_capability("beardog"), "pki");
        assert_eq!(service_to_capability("songbird"), "orchestration");
        assert_eq!(service_to_capability("nestgate"), "storage");
        assert_eq!(service_to_capability("squirrel"), "ai");
    }

    #[test]
    fn test_capability_to_service() {
        assert_eq!(capability_to_service("pki"), "beardog");
        assert_eq!(capability_to_service("orchestration"), "songbird");
        assert_eq!(capability_to_service("storage"), "nestgate");
        assert_eq!(capability_to_service("ai"), "squirrel");
    }

    #[test]
    fn test_dependencies_conversion() {
        let deps = vec!["beardog".to_string(), "nestgate".to_string()];
        let caps = dependencies_to_capabilities(&deps);
        assert_eq!(caps, vec!["pki", "storage"]);

        let back = capabilities_to_dependencies(&caps);
        assert_eq!(back, deps);
    }

    #[test]
    fn test_get_capability_mappings() {
        let mappings = get_capability_mappings();
        assert_eq!(mappings.get("beardog"), Some(&"pki"));
        assert_eq!(mappings.get("songbird"), Some(&"orchestration"));
        assert!(mappings.len() >= 6);
    }
}
