// SPDX-License-Identifier: AGPL-3.0-or-later
//! IPC surface constants for Neural API self-announcement.
//!
//! Provides the `compute.*`, `science.*`, and `inference.*` method names
//! that toadStool announces to biomeOS on startup (Wave 43).

/// Methods announced to biomeOS Neural API via `primal.announce`.
///
/// Filtered to the `compute.*`, `science.*`, and `inference.*` namespaces
/// that toadStool provides as a node-tier compute primal.
pub const ANNOUNCED_METHODS: &[&str] = &[
    "compute.cancel",
    "compute.capabilities",
    "compute.context.init",
    "compute.discover_capabilities",
    "compute.dispatch",
    "compute.dispatch.capabilities",
    "compute.dispatch.forward",
    "compute.dispatch.pipeline.status",
    "compute.dispatch.pipeline.submit",
    "compute.dispatch.result",
    "compute.dispatch.status",
    "compute.dispatch.submit",
    "compute.execute",
    "compute.fan_out",
    "compute.hardware.apply",
    "compute.hardware.auto_init",
    "compute.hardware.auto_init_all",
    "compute.hardware.distill",
    "compute.hardware.observe",
    "compute.hardware.share_recipe",
    "compute.hardware.status",
    "compute.hardware.vfio_devices",
    "compute.health",
    "compute.list",
    "compute.performance_surface.list",
    "compute.performance_surface.query",
    "compute.performance_surface.report",
    "compute.result",
    "compute.route.multi_unit",
    "compute.status",
    "compute.submit",
    "compute.version",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announced_methods_sorted() {
        let mut sorted = ANNOUNCED_METHODS.to_vec();
        sorted.sort_unstable();
        assert_eq!(ANNOUNCED_METHODS, &sorted[..]);
    }

    #[test]
    fn announced_methods_all_compute_namespace() {
        for m in ANNOUNCED_METHODS {
            assert!(
                m.starts_with("compute.") || m.starts_with("science.") || m.starts_with("inference."),
                "method {m} is not in compute/science/inference namespace"
            );
        }
    }
}
