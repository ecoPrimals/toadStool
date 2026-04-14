// SPDX-License-Identifier: AGPL-3.0-or-later

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
    let graph = PcieTopologyGraph::empty();
    assert!(graph.switch_neighbors(0).is_empty());
}

#[test]
fn test_pair_lookup_none() {
    let graph = PcieTopologyGraph::empty();
    assert!(graph.pair(0, 1).is_none());
}

#[test]
fn test_effective_bandwidth_no_pair() {
    let graph = PcieTopologyGraph::empty();
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
        #[expect(clippy::cast_precision_loss, reason = "display-only bandwidth value")]
        let bw_gbps = bw as f64 / 1e9;
        println!(
            "  card{} <-> card{}: hops={}, contention={:.2}, same_numa={}, same_iommu={}, bw={bw_gbps:.1} Gbps",
            pair.gpu_a,
            pair.gpu_b,
            pair.hops,
            pair.contention_factor,
            pair.same_numa,
            pair.same_iommu_group,
        );
    }
}
