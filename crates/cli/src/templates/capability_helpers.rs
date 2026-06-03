// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based template helpers
//!
//! This module provides helpers for capability-first service dependencies in templates.
//! Legacy **orchestrator labels** in manifests are normalized via [`CapabilityDomain::from_label`].

use toadstool_common::interned_strings::CapabilityDomain;
use toadstool_common::interned_strings::capabilities;
use toadstool_common::interned_strings::runtime_types;

/// Resolve a manifest or template dependency label to a canonical capability id.
///
/// Delegates to [`CapabilityDomain::from_label`] for legacy orchestrator labels and capability
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

/// Convert dependency labels to capability ids (capability strings pass through; legacy
/// orchestrator labels are normalized — see [`CapabilityDomain::from_label`]).
///
/// # Example
/// ```
/// use toadstool_cli::templates::capability_helpers::dependencies_to_capabilities;
///
/// assert_eq!(
///     dependencies_to_capabilities(&["crypto".to_string(), "storage".to_string()]),
///     vec!["crypto", "storage"]
/// );
/// ```
pub fn dependencies_to_capabilities(service_names: &[String]) -> Vec<&'static str> {
    service_names
        .iter()
        .map(|name| dependency_label_to_capability(name))
        .collect()
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
    fn test_dependencies_conversion() {
        let deps = vec!["beardog".to_string(), "nestgate".to_string()];
        let caps = dependencies_to_capabilities(&deps);
        assert_eq!(caps, vec!["crypto", "storage"]);
    }
}
