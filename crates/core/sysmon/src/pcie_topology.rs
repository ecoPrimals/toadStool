// SPDX-License-Identifier: AGPL-3.0-only
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

use crate::gpu::{discover_gpus, GpuDevice};

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
#[derive(Debug, Clone)]
pub struct PcieTopologyGraph {
    /// All discovered GPU devices.
    pub gpus: Vec<GpuDevice>,
    /// Bridge ancestry for each GPU (card_index -> ordered list from device to root).
    pub bridge_chains: HashMap<u32, Vec<PciBridge>>,
    /// Pairwise topology for each GPU pair.
    pub pairs: Vec<GpuPairTopology>,
    /// Number of GPUs sharing each bridge (bridge PCI slot -> count).
    pub bridge_fanout: HashMap<String, u32>,
}

impl PcieTopologyGraph {
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
        let pair = match self.pair(gpu_a, gpu_b) {
            Some(p) => p,
            None => return 0,
        };

        let gpu_a_dev = self.gpus.iter().find(|g| g.card_index == gpu_a);
        let gpu_b_dev = self.gpus.iter().find(|g| g.card_index == gpu_b);

        let (gen_a, width_a) = gpu_a_dev
            .map(|g: &GpuDevice| {
                let t = g.pcie_topology();
                (t.gen.unwrap_or(3), t.width.unwrap_or(16))
            })
            .unwrap_or((3, 16));

        let (gen_b, width_b) = gpu_b_dev
            .map(|g: &GpuDevice| {
                let t = g.pcie_topology();
                (t.gen.unwrap_or(3), t.width.unwrap_or(16))
            })
            .unwrap_or((3, 16));

        let raw_a = raw_pcie_bandwidth_bps(gen_a, width_a);
        let raw_b = raw_pcie_bandwidth_bps(gen_b, width_b);
        let min_raw = raw_a.min(raw_b);

        (min_raw as f64 * pair.contention_factor * 0.78) as u64
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

    let resolved = match std::fs::canonicalize(device_path) {
        Ok(p) => p,
        Err(_) => return chain,
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
    let (a, b) = match (chain_a, chain_b) {
        (Some(a), Some(b)) => (a, b),
        _ => return (None, u32::MAX),
    };

    for (i, bridge_a) in a.iter().enumerate() {
        for (j, bridge_b) in b.iter().enumerate() {
            if bridge_a.pci_slot == bridge_b.pci_slot {
                let hops = (i as u32) + (j as u32);
                return (Some(bridge_a.clone()), hops);
            }
        }
    }

    (None, u32::MAX)
}

/// Raw unidirectional `PCIe` bandwidth in bytes/sec for a given gen and width.
#[must_use]
pub const fn raw_pcie_bandwidth_bps(gen: u32, width: u32) -> u64 {
    let lane_bps: u64 = match gen {
        1 => 250_000_000,
        2 => 500_000_000,
        3 => 984_600_000,
        4 => 1_969_000_000,
        5 => 3_938_000_000,
        6 => 7_563_000_000,
        _ => 984_600_000, // default to Gen 3
    };
    lane_bps * width as u64
}

fn read_sysfs_hex_file(path: &Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    let trimmed = s.trim().trim_start_matches("0x");
    u32::from_str_radix(trimmed, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_pcie_bandwidth() {
        assert_eq!(raw_pcie_bandwidth_bps(3, 16), 984_600_000 * 16);
        assert_eq!(raw_pcie_bandwidth_bps(4, 16), 1_969_000_000 * 16);
        assert_eq!(raw_pcie_bandwidth_bps(4, 8), 1_969_000_000 * 8);
        assert_eq!(raw_pcie_bandwidth_bps(5, 16), 3_938_000_000 * 16);
    }

    #[test]
    fn test_raw_pcie_bandwidth_gen1() {
        assert_eq!(raw_pcie_bandwidth_bps(1, 1), 250_000_000);
        assert_eq!(raw_pcie_bandwidth_bps(2, 4), 2_000_000_000);
    }

    #[test]
    fn test_discover_topology_returns_graph() {
        let graph = discover_topology();
        // On CI without GPUs: empty graph is valid
        // On strandgate with 2 GPUs: should have pairs
        if graph.gpus.len() >= 2 {
            assert!(!graph.pairs.is_empty());
            for pair in &graph.pairs {
                assert!(pair.contention_factor > 0.0);
                assert!(pair.contention_factor <= 1.0);
            }
        }
    }

    #[test]
    fn test_switch_neighbors_empty() {
        let graph = PcieTopologyGraph {
            gpus: Vec::new(),
            bridge_chains: HashMap::new(),
            pairs: Vec::new(),
            bridge_fanout: HashMap::new(),
        };
        assert!(graph.switch_neighbors(0).is_empty());
    }

    #[test]
    fn test_pair_lookup_none() {
        let graph = PcieTopologyGraph {
            gpus: Vec::new(),
            bridge_chains: HashMap::new(),
            pairs: Vec::new(),
            bridge_fanout: HashMap::new(),
        };
        assert!(graph.pair(0, 1).is_none());
    }

    #[test]
    fn test_effective_bandwidth_no_pair() {
        let graph = PcieTopologyGraph {
            gpus: Vec::new(),
            bridge_chains: HashMap::new(),
            pairs: Vec::new(),
            bridge_fanout: HashMap::new(),
        };
        assert_eq!(graph.effective_bandwidth_bps(0, 1), 0);
    }

    #[test]
    fn test_find_common_bridge_none() {
        let (bridge, hops) = find_common_bridge(None, None);
        assert!(bridge.is_none());
        assert_eq!(hops, u32::MAX);
    }

    #[test]
    fn test_find_common_bridge_shared() {
        let chain_a = vec![PciBridge {
            pci_slot: "0000:00:01.0".to_string(),
            sysfs_path: PathBuf::from("/sys/bus/pci/devices/0000:00:01.0"),
            class_code: 0x0604,
            depth: 0,
        }];
        let chain_b = vec![PciBridge {
            pci_slot: "0000:00:01.0".to_string(),
            sysfs_path: PathBuf::from("/sys/bus/pci/devices/0000:00:01.0"),
            class_code: 0x0604,
            depth: 0,
        }];
        let (bridge, hops) = find_common_bridge(Some(&chain_a), Some(&chain_b));
        assert!(bridge.is_some());
        assert_eq!(hops, 0);
        assert_eq!(bridge.unwrap().pci_slot, "0000:00:01.0");
    }

    #[test]
    fn test_find_common_bridge_different_depths() {
        let chain_a = vec![
            PciBridge {
                pci_slot: "0000:03:00.0".to_string(),
                sysfs_path: PathBuf::new(),
                class_code: 0x0604,
                depth: 0,
            },
            PciBridge {
                pci_slot: "0000:00:01.0".to_string(),
                sysfs_path: PathBuf::new(),
                class_code: 0x0604,
                depth: 1,
            },
        ];
        let chain_b = vec![PciBridge {
            pci_slot: "0000:00:01.0".to_string(),
            sysfs_path: PathBuf::new(),
            class_code: 0x0604,
            depth: 0,
        }];
        let (bridge, hops) = find_common_bridge(Some(&chain_a), Some(&chain_b));
        assert!(bridge.is_some());
        // chain_a index 1 + chain_b index 0 = 1 hop
        assert_eq!(hops, 1);
    }

    #[test]
    fn test_contention_factor_synthetic() {
        let pair = GpuPairTopology {
            gpu_a: 0,
            gpu_b: 1,
            common_bridge: Some(PciBridge {
                pci_slot: "0000:00:01.0".to_string(),
                sysfs_path: PathBuf::new(),
                class_code: 0x0604,
                depth: 0,
            }),
            hops: 0,
            same_iommu_group: true,
            same_numa: true,
            contention_factor: 0.25, // 4 GPUs sharing one x16
        };
        assert!((pair.contention_factor - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    #[ignore = "requires GPU hardware"]
    fn test_discover_topology_on_hardware() {
        let graph = discover_topology();
        assert!(!graph.gpus.is_empty(), "Expected GPUs on hardware");

        println!("=== PCIe Topology Graph ===");
        println!("GPUs: {}", graph.gpus.len());

        for gpu in &graph.gpus {
            let chain = graph.bridge_chains.get(&gpu.card_index);
            let bridge_count = chain.map_or(0, Vec::len);
            println!(
                "  card{}: {} {:04x} pci={} bridges={}",
                gpu.card_index, gpu.vendor, gpu.device_id, gpu.pci_slot, bridge_count
            );
            if let Some(chain) = chain {
                for (i, bridge) in chain.iter().enumerate() {
                    println!(
                        "    bridge[{}]: {} depth={}",
                        i, bridge.pci_slot, bridge.depth
                    );
                }
            }
        }

        for pair in &graph.pairs {
            let bw = graph.effective_bandwidth_bps(pair.gpu_a, pair.gpu_b);
            println!(
                "  card{} <-> card{}: hops={}, contention={:.2}, same_numa={}, same_iommu={}, bw={:.1} Gbps",
                pair.gpu_a, pair.gpu_b, pair.hops, pair.contention_factor,
                pair.same_numa, pair.same_iommu_group,
                bw as f64 / 1e9
            );
        }
    }
}
