// SPDX-License-Identifier: AGPL-3.0-or-later
//! DRM node observation for a PCI device.
//!
//! A seeder driver that registers a DRM node on the target is the single most
//! dangerous thing that can happen during a handoff on a machine with a live
//! desktop. The node appears, the display server's udev monitor hot-adds it,
//! and the display server initializes a display driver against a GPU that is
//! mid-rotation.
//!
//! `nouveau modeset=2` does *not* prevent this. It suppresses display output
//! (the card reports no CRTCs) but still calls `drm_dev_register`, so
//! `/dev/dri/cardN` appears exactly as it would in full KMS mode.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// The set of DRM nodes a PCI device currently exposes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrmNodes {
    /// Node names as they appear under the device's `drm/` directory,
    /// e.g. `card0`, `renderD128`.
    pub nodes: BTreeSet<String>,
}

impl DrmNodes {
    /// Read the DRM nodes currently registered for `bdf`.
    ///
    /// An absent `drm/` directory is not an error: it is the normal state for
    /// a device bound to `vfio-pci`, or to no driver at all.
    #[must_use]
    pub fn for_device(bdf: &str) -> Self {
        let dir = PathBuf::from("/sys/bus/pci/devices").join(bdf).join("drm");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Self::default();
        };

        let nodes = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            // The drm/ directory also carries controlD* legacy links and
            // per-node subdirectories; card/render are the ones a display
            // server will act on.
            .filter(|n| n.starts_with("card") || n.starts_with("renderD"))
            .collect();

        Self { nodes }
    }

    /// True when the device exposes a `card*` node.
    ///
    /// Render nodes (`renderD*`) are compute-only and are not hot-added as
    /// GPU devices by X, so they are not by themselves a session hazard.
    #[must_use]
    pub fn has_card_node(&self) -> bool {
        self.nodes.iter().any(|n| n.starts_with("card"))
    }

    /// Nodes present in `self` but absent from `earlier`.
    #[must_use]
    pub fn appeared_since(&self, earlier: &Self) -> Vec<String> {
        self.nodes.difference(&earlier.nodes).cloned().collect()
    }
}

/// Watches a device for DRM nodes appearing after a baseline was taken.
///
/// Armed before a seeder module is loaded and polled during settle, so a
/// rotation can be aborted while the hazard is still only a device node,
/// rather than after a display server has bound to it and crashed.
#[derive(Debug, Clone)]
pub struct DrmNodeWatch {
    bdf: String,
    baseline: DrmNodes,
}

impl DrmNodeWatch {
    /// Capture the device's current DRM nodes as the baseline.
    #[must_use]
    pub fn arm(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            baseline: DrmNodes::for_device(bdf),
        }
    }

    /// The nodes recorded when the watch was armed.
    #[must_use]
    pub const fn baseline(&self) -> &DrmNodes {
        &self.baseline
    }

    /// Nodes that have appeared since arming.
    #[must_use]
    pub fn poll(&self) -> Vec<String> {
        DrmNodes::for_device(&self.bdf).appeared_since(&self.baseline)
    }

    /// A newly appeared `card*` node, if any.
    ///
    /// This is the abort condition: a card node on the target means a display
    /// server may hot-add it at any moment.
    #[must_use]
    pub fn breached(&self) -> Option<String> {
        self.poll().into_iter().find(|n| n.starts_with("card"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(items: &[&str]) -> DrmNodes {
        DrmNodes {
            nodes: items.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn absent_drm_directory_yields_no_nodes() {
        // A BDF that cannot exist; models a vfio-pci or unbound device.
        let observed = DrmNodes::for_device("0000:ff:ff.9");
        assert!(observed.nodes.is_empty());
        assert!(!observed.has_card_node());
    }

    #[test]
    fn card_node_detected_render_node_alone_is_not_a_card() {
        assert!(nodes(&["card0"]).has_card_node());
        assert!(nodes(&["card1", "renderD128"]).has_card_node());
        assert!(!nodes(&["renderD128"]).has_card_node());
        assert!(!nodes(&[]).has_card_node());
    }

    #[test]
    fn appeared_since_reports_only_new_nodes() {
        let before = nodes(&["renderD128"]);
        let after = nodes(&["card0", "renderD128"]);
        assert_eq!(after.appeared_since(&before), vec!["card0".to_string()]);
        assert!(before.appeared_since(&after).is_empty());
        assert!(after.appeared_since(&after).is_empty());
    }

    /// The exact shape of the 2026-08-16 Titan V failure: the device had no
    /// DRM nodes while on vfio-pci, then nouveau registered card0.
    #[test]
    fn detects_the_titan_v_hot_add_shape() {
        let watch = DrmNodeWatch {
            bdf: "0000:21:00.0".into(),
            baseline: DrmNodes::default(),
        };
        let after_seed = nodes(&["card0", "renderD128"]);
        let new = after_seed.appeared_since(watch.baseline());
        assert!(new.contains(&"card0".to_string()));
        assert!(new.iter().any(|n| n.starts_with("card")));
    }
}
