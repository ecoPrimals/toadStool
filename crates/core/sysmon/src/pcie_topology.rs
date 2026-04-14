// SPDX-License-Identifier: AGPL-3.0-or-later
//! `PCIe` switch topology discovery via sysfs.
//!
//! Walks the PCI bus hierarchy to discover shared switches, parent bridges,
//! and the full interconnect graph. This enables:
//! - Detecting GPUs that share a `PCIe` switch (can P2P without root complex)
//! - Estimating effective bandwidth with contention for multi-GPU arrays
//! - Routing cooperating workloads to GPUs with fast interconnects
//!
//! ## Architecture
//!
//! A `PCIe` daisy-chain (e.g. 4x RTX 3050 behind a PLX switch on one x16 slot)
//! looks like this in sysfs:
//!
//! ```text
//! Root Complex
//!   └── Bridge (x16)
//!         └── PLX Switch
//!               ├── Port 0 → GPU 0 (x4)
//!               ├── Port 1 → GPU 1 (x4)
//!               ├── Port 2 → GPU 2 (x4)
//!               └── Port 3 → GPU 3 (x4)
//! ```
//!
//! Two GPUs sharing a switch can P2P through the switch fabric. GPUs on
//! different switches must traverse the root complex (higher latency, shared
//! bandwidth).
//!
//! ## Absorbed From
//!
//! groundSpring V61 `InterconnectTopology` concept, hotSpring v0.6.25 dual-GPU
//! `DevicePair` patterns, and toadStool S142 `PcieTransport` bandwidth model.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::gpu::{GpuDevice, discover_gpus};

/// A PCI bridge or switch in the bus hierarchy.
#[derive(Debug, Clone)]
pub struct PciBridge {
    /// PCI slot address of the bridge (e.g. `"0000:00:01.0"`).
    pub pci_slot: String,
    /// Sysfs device path.
    pub sysfs_path: PathBuf,
    /// PCI class code (0x0604 = PCI bridge, 0x0604 with prog-if for switch).
    pub class_code: u32,
    /// Depth from root complex (0 = root port).
    pub depth: u32,
}

/// Topology relationship between two GPUs.
#[derive(Debug, Clone)]
pub struct GpuPairTopology {
    /// Card index of first GPU.
    pub gpu_a: u32,
    /// Card index of second GPU.
    pub gpu_b: u32,
    /// Nearest common ancestor bridge (if on same branch).
    pub common_bridge: Option<PciBridge>,
    /// Number of hops through switches (0 = direct, 1 = shared switch, 2+ = cascaded).
    pub hops: u32,
    /// Whether both GPUs share the same IOMMU group (strong P2P indicator).
    pub same_iommu_group: bool,
    /// Whether both GPUs are on the same NUMA node.
    pub same_numa: bool,
    /// Estimated contention factor (1.0 = uncontested, 0.25 = 4 GPUs sharing one x16).
    pub contention_factor: f64,
}

/// Full `PCIe` topology graph for all discovered GPUs.
///
/// # Stability
///
/// This API is **stable** as of toadStool S146. Springs may depend on
/// `PcieTopologyGraph`, `pair()`, `switch_neighbors()`, and
/// `effective_bandwidth_bps()` without breakage risk. Fields may be added
/// (non-exhaustive struct) but existing methods are frozen.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PcieTopologyGraph {
    /// All discovered GPU devices.
    pub gpus: Vec<GpuDevice>,
    /// Bridge ancestry for each GPU (`card_index` -> ordered list from device to root).
    pub bridge_chains: HashMap<u32, Vec<PciBridge>>,
    /// Pairwise topology for each GPU pair.
    pub pairs: Vec<GpuPairTopology>,
    /// Number of GPUs sharing each bridge (bridge PCI slot -> count).
    pub bridge_fanout: HashMap<String, u32>,
}

impl PcieTopologyGraph {
    /// Construct an empty topology graph (for testing or manual assembly).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            gpus: Vec::new(),
            bridge_chains: HashMap::new(),
            pairs: Vec::new(),
            bridge_fanout: HashMap::new(),
        }
    }

    /// Get topology for a specific GPU pair.
    #[must_use]
    pub fn pair(&self, gpu_a: u32, gpu_b: u32) -> Option<&GpuPairTopology> {
        self.pairs.iter().find(|p| {
            (p.gpu_a == gpu_a && p.gpu_b == gpu_b) || (p.gpu_a == gpu_b && p.gpu_b == gpu_a)
        })
    }

    /// Get all GPUs that share a switch with the given GPU.
    #[must_use]
    pub fn switch_neighbors(&self, card_index: u32) -> Vec<u32> {
        self.pairs
            .iter()
            .filter(|p| {
                (p.gpu_a == card_index || p.gpu_b == card_index)
                    && p.common_bridge.is_some()
                    && p.hops <= 1
            })
            .map(|p| {
                if p.gpu_a == card_index {
                    p.gpu_b
                } else {
                    p.gpu_a
                }
            })
            .collect()
    }

    /// Effective bandwidth between two GPUs in bytes/sec, accounting for
    /// switch contention and topology.
    ///
    /// Uses the formula: `raw_link_bps * contention_factor * 0.78` where
    /// 0.78 accounts for `PCIe` encoding and protocol overhead.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "PCIe bandwidth values are well within f64 mantissa range"
    )]
    pub fn effective_bandwidth_bps(&self, gpu_a: u32, gpu_b: u32) -> u64 {
        let Some(pair) = self.pair(gpu_a, gpu_b) else {
            return 0;
        };

        let dev_a = self.gpus.iter().find(|g| g.card_index == gpu_a);
        let dev_b = self.gpus.iter().find(|g| g.card_index == gpu_b);

        let (gen_a, width_a) = dev_a.map_or((3, 16), |g: &GpuDevice| {
            let t = g.pcie_topology();
            (t.generation.unwrap_or(3), t.width.unwrap_or(16))
        });

        let (gen_b, width_b) = dev_b.map_or((3, 16), |g: &GpuDevice| {
            let t = g.pcie_topology();
            (t.generation.unwrap_or(3), t.width.unwrap_or(16))
        });

        let raw_a = raw_pcie_bandwidth_bps(gen_a, width_a);
        let raw_b = raw_pcie_bandwidth_bps(gen_b, width_b);
        let min_raw = raw_a.min(raw_b);

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bandwidth product is always positive and within u64 range"
        )]
        let effective = (min_raw as f64 * pair.contention_factor * 0.78) as u64;
        effective
    }
}

/// Discover the full `PCIe` topology graph for all GPUs.
///
/// Walks sysfs parent bridges for each GPU, builds bridge chains,
/// finds common ancestors, and computes contention factors.
#[must_use]
pub fn discover_topology() -> PcieTopologyGraph {
    let gpus = discover_gpus();

    let mut bridge_chains: HashMap<u32, Vec<PciBridge>> = HashMap::new();
    let mut bridge_fanout: HashMap<String, u32> = HashMap::new();

    for gpu in &gpus {
        let chain = discover_bridge_chain(&gpu.sysfs_device);
        for bridge in &chain {
            *bridge_fanout.entry(bridge.pci_slot.clone()).or_insert(0) += 1;
        }
        bridge_chains.insert(gpu.card_index, chain);
    }

    let mut pairs = Vec::new();
    for i in 0..gpus.len() {
        for j in (i + 1)..gpus.len() {
            let gpu_a = &gpus[i];
            let gpu_b = &gpus[j];

            let topo_a = gpu_a.pcie_topology();
            let topo_b = gpu_b.pcie_topology();

            let chain_a = bridge_chains.get(&gpu_a.card_index);
            let chain_b = bridge_chains.get(&gpu_b.card_index);

            let (common_bridge, hops) =
                find_common_bridge(chain_a.map(Vec::as_slice), chain_b.map(Vec::as_slice));

            let contention_factor = common_bridge
                .as_ref()
                .and_then(|b| bridge_fanout.get(&b.pci_slot))
                .map_or(0.5, |&fanout| {
                    if fanout <= 1 {
                        1.0
                    } else {
                        1.0 / f64::from(fanout)
                    }
                });

            let same_iommu = matches!(
                (topo_a.iommu_group, topo_b.iommu_group),
                (Some(a), Some(b)) if a == b
            );

            let same_numa = matches!(
                (topo_a.numa_node, topo_b.numa_node),
                (Some(a), Some(b)) if a == b && a >= 0
            );

            pairs.push(GpuPairTopology {
                gpu_a: gpu_a.card_index,
                gpu_b: gpu_b.card_index,
                common_bridge,
                hops,
                same_iommu_group: same_iommu,
                same_numa,
                contention_factor,
            });
        }
    }

    PcieTopologyGraph {
        gpus,
        bridge_chains,
        pairs,
        bridge_fanout,
    }
}

/// Walk the sysfs parent chain from a GPU device up to the root complex.
///
/// Returns ordered bridges from nearest parent to root port.
fn discover_bridge_chain(device_path: &Path) -> Vec<PciBridge> {
    let mut chain = Vec::new();
    let mut depth = 0u32;

    let Ok(resolved) = std::fs::canonicalize(device_path) else {
        return chain;
    };

    let mut current = resolved;
    while let Some(p) = current.parent() {
        let parent = p.to_path_buf();

        if !parent.join("class").exists() {
            if parent.as_os_str() == "/" || parent == current {
                break;
            }
            current = parent;
            continue;
        }

        let class_code = read_sysfs_hex_file(&parent.join("class")).unwrap_or(0) >> 8;
        // 0x0604 = PCI-to-PCI bridge
        if class_code != 0x0604 {
            current = parent;
            continue;
        }

        let pci_slot = parent
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        chain.push(PciBridge {
            pci_slot,
            sysfs_path: parent.clone(),
            class_code,
            depth,
        });
        depth += 1;

        current = parent;
    }

    chain
}

/// Find the nearest common bridge (ancestor) for two GPUs.
///
/// Returns the bridge and the total hop count (sum of depths from each GPU
/// to the common ancestor).
fn find_common_bridge(
    chain_a: Option<&[PciBridge]>,
    chain_b: Option<&[PciBridge]>,
) -> (Option<PciBridge>, u32) {
    let (Some(a), Some(b)) = (chain_a, chain_b) else {
        return (None, u32::MAX);
    };

    for (i, bridge_a) in a.iter().enumerate() {
        for (j, bridge_b) in b.iter().enumerate() {
            if bridge_a.pci_slot == bridge_b.pci_slot {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "bridge chains have at most ~8 entries (PCIe hierarchy depth)"
                )]
                let hops = (i as u32) + (j as u32);
                return (Some(bridge_a.clone()), hops);
            }
        }
    }

    (None, u32::MAX)
}

/// Raw unidirectional `PCIe` bandwidth in bytes/sec for a given gen and width.
#[must_use]
pub const fn raw_pcie_bandwidth_bps(generation: u32, width: u32) -> u64 {
    let lane_bps: u64 = match generation {
        1 => 250_000_000,
        2 => 500_000_000,
        4 => 1_969_000_000,
        5 => 3_938_000_000,
        6 => 7_563_000_000,
        // Gen 3 is the most common baseline; also used as fallback for unknown gens.
        _ => 984_600_000,
    };
    lane_bps * width as u64
}

fn read_sysfs_hex_file(path: &Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    let trimmed = s.trim().trim_start_matches("0x");
    u32::from_str_radix(trimmed, 16).ok()
}

#[cfg(test)]
#[path = "pcie_topology_tests.rs"]
mod tests;
