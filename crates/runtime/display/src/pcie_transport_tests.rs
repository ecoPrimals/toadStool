// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_raw_pcie_bandwidth_gen3_x16() {
    let bps = raw_pcie_bandwidth_bps(Some(3), Some(16));
    assert_eq!(bps, 128_000_000_000);
}

#[test]
fn test_raw_pcie_bandwidth_gen4_x16() {
    let bps = raw_pcie_bandwidth_bps(Some(4), Some(16));
    assert_eq!(bps, 256_000_000_000);
}

#[test]
fn test_raw_pcie_bandwidth_gen5_x16() {
    let bps = raw_pcie_bandwidth_bps(Some(5), Some(16));
    assert_eq!(bps, 512_000_000_000);
}

#[test]
fn test_raw_pcie_bandwidth_unknown_gen() {
    assert_eq!(raw_pcie_bandwidth_bps(None, Some(16)), 0);
    assert_eq!(raw_pcie_bandwidth_bps(Some(99), Some(16)), 0);
}

#[test]
fn test_estimate_link_bandwidth_uses_minimum() {
    let fast = PcieEndpoint {
        card_index: 0,
        pci_slot: "0000:25:00.0".into(),
        vendor: GpuVendor::Amd,
        render_node: PathBuf::from("/dev/dri/renderD128"),
        numa_node: Some(0),
        pcie_gen: Some(4),
        pcie_width: Some(16),
    };
    let slow = PcieEndpoint {
        card_index: 1,
        pci_slot: "0000:41:00.0".into(),
        vendor: GpuVendor::Nvidia,
        render_node: PathBuf::from("/dev/dri/renderD129"),
        numa_node: Some(0),
        pcie_gen: Some(3),
        pcie_width: Some(16),
    };
    let bw = estimate_link_bandwidth(&fast, &slow);
    let expected_raw = 128_000_000_000u64; // gen3 x16
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "test assertion"
    )]
    let expected = (expected_raw as f64 * 0.78) as u64;
    assert_eq!(bw, expected);
}

#[test]
fn test_discover_pcie_links_returns_vec() {
    let links = discover_pcie_links();
    for link in &links {
        assert!(!link.source.pci_slot.is_empty());
        assert!(!link.target.pci_slot.is_empty());
        assert_ne!(link.source.card_index, link.target.card_index);
    }
}

#[test]
fn test_discover_pcie_transports_returns_transport_info() {
    let transports = discover_pcie_transports();
    for t in &transports {
        assert_eq!(t.medium, TransportMedium::Pcie);
        assert_eq!(t.direction, TransportDirection::Bidirectional);
        assert!(t.id.starts_with("pcie:"));
    }
}

#[test]
fn test_gpus_by_numa() {
    let by_numa = gpus_by_numa();
    let total: usize = by_numa.values().map(Vec::len).sum();
    assert_eq!(total, discover_gpus().len());
}

#[test]
fn test_pcie_endpoint_fields() {
    let ep = PcieEndpoint {
        card_index: 0,
        pci_slot: "0000:25:00.0".into(),
        vendor: GpuVendor::Amd,
        render_node: PathBuf::from("/dev/dri/renderD128"),
        numa_node: Some(0),
        pcie_gen: Some(4),
        pcie_width: Some(16),
    };
    assert_eq!(ep.card_index, 0);
    assert_eq!(ep.vendor, GpuVendor::Amd);
}

#[test]
fn test_pcie_link_same_numa() {
    let link = PcieLink {
        source: PcieEndpoint {
            card_index: 0,
            pci_slot: "0000:25:00.0".into(),
            vendor: GpuVendor::Amd,
            render_node: PathBuf::from("/dev/dri/renderD128"),
            numa_node: Some(0),
            pcie_gen: Some(4),
            pcie_width: Some(16),
        },
        target: PcieEndpoint {
            card_index: 1,
            pci_slot: "0000:41:00.0".into(),
            vendor: GpuVendor::Nvidia,
            render_node: PathBuf::from("/dev/dri/renderD129"),
            numa_node: Some(0),
            pcie_gen: Some(4),
            pcie_width: Some(16),
        },
        bandwidth_bps: 200_000_000_000,
        same_numa: true,
        via_switch: None,
        hops: 0,
        contention_factor: 1.0,
    };
    assert!(link.same_numa);
    assert!((link.contention_factor - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_pcie_transport_medium() {
    assert_eq!(format!("{}", TransportMedium::Pcie), "PCIe");
}

#[test]
#[ignore = "requires 2+ GPU devices"]
#[expect(
    clippy::cast_precision_loss,
    reason = "display formatting for bandwidth"
)]
fn test_discover_pcie_links_on_hardware() {
    let links = discover_pcie_links();
    assert!(!links.is_empty(), "Expected PCIe links between GPUs");
    for link in &links {
        let gb_per_sec = link.bandwidth_bps as f64 / 1e9;
        println!(
            "{}→{}: {gb_per_sec:.1} GB/s (NUMA same={})",
            link.source.pci_slot, link.target.pci_slot, link.same_numa
        );
    }
}

#[test]
#[ignore = "requires 2+ GPU devices"]
fn test_open_pcie_transport_on_hardware() {
    let links = discover_pcie_links();
    assert!(!links.is_empty());
    let link = &links[0];
    let transport = PcieTransport::open(link.source.clone(), link.target.clone());
    assert!(
        transport.is_ok(),
        "Failed to open transport: {:?}",
        transport.err()
    );
    let t = transport.unwrap();
    assert!(t.is_available());
    assert!(t.bandwidth_bps() > 0);
    println!("Opened: {} ({} bps)", t.info().id, t.bandwidth_bps());
}
