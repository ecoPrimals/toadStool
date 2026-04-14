// SPDX-License-Identifier: AGPL-3.0-or-later
//! `PCIe` peer-to-peer transport — GPU-to-GPU data movement.
//!
//! Implements [`HardwareTransport`] for `PCIe` paths between GPU render nodes.
//! Each `PcieTransport` represents a unidirectional or bidirectional data path
//! between two DRM render nodes on the same `PCIe` fabric.
//!
//! ## Mechanism
//!
//! Current implementation uses CPU-staged transfer through DRM render nodes:
//! source writes → CPU memory → target reads. This is already faster than
//! application-level copy for DMA-capable buffers.
//!
//! Future evolution: true P2P via `dma-buf` export/import
//! (`DRM_IOCTL_PRIME_HANDLE_TO_FD` / `DRM_IOCTL_PRIME_FD_TO_HANDLE`).
//!
//! ## Bandwidth
//!
//! Bandwidth is reported from `PCIe` link characteristics discovered via sysfs:
//! - `PCIe` 3.0 x16: ~12 `GB/s` practical
//! - `PCIe` 4.0 x16: ~25 `GB/s` practical
//! - `PCIe` 5.0 x16: ~50 `GB/s` practical

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;

use toadstool_core::{
    HardwareTransport, TransportDirection, TransportError, TransportInfo, TransportMedium,
};
use toadstool_sysmon::gpu::{GpuDevice, GpuVendor, discover_gpus};
use toadstool_sysmon::pcie_topology::{PciBridge, discover_topology};

/// A `PCIe` link between two GPU render nodes.
///
/// The transport reads from the source render node and writes to the target,
/// using CPU staging. Topology information enables the `TransportRouter` to
/// make intelligent placement decisions.
pub struct PcieTransport {
    info: TransportInfo,
    source: PcieEndpoint,
    target: PcieEndpoint,
    bandwidth: u64,
}

/// One end of a `PCIe` transport link.
#[derive(Debug, Clone)]
pub struct PcieEndpoint {
    /// DRM card index
    pub card_index: u32,
    /// `PCIe` slot (e.g. "0000:25:00.0")
    pub pci_slot: String,
    /// Vendor
    pub vendor: GpuVendor,
    /// Render node path
    pub render_node: PathBuf,
    /// NUMA node (-1 or actual)
    pub numa_node: Option<i32>,
    /// `PCIe` generation
    pub pcie_gen: Option<u32>,
    /// `PCIe` link width
    pub pcie_width: Option<u32>,
}

/// A discovered `PCIe` link between two GPUs with bandwidth estimate.
#[derive(Debug, Clone)]
pub struct PcieLink {
    /// Source endpoint info.
    pub source: PcieEndpoint,
    /// Target endpoint info.
    pub target: PcieEndpoint,
    /// Estimated bandwidth in bits per second.
    pub bandwidth_bps: u64,
    /// Whether both GPUs share the same NUMA node.
    pub same_numa: bool,
    /// The `PCIe` switch/bridge shared between source and target (if any).
    pub via_switch: Option<PciBridge>,
    /// Number of switch hops between source and target (0 = direct neighbor).
    pub hops: u32,
    /// Contention factor from switch fan-out (1.0 = uncontested, 0.25 = 4-way shared).
    pub contention_factor: f64,
}

impl PcieTransport {
    /// Open a `PCIe` transport between two GPU render nodes.
    ///
    /// The transport ID format is `pcie:{source_slot}→{target_slot}`.
    ///
    /// # Errors
    ///
    /// Returns an error if either render node cannot be accessed.
    pub fn open(source: PcieEndpoint, target: PcieEndpoint) -> Result<Self, TransportError> {
        if !source.render_node.exists() {
            return Err(TransportError::OpenFailed(format!(
                "source render node not found: {}",
                source.render_node.display()
            )));
        }
        if !target.render_node.exists() {
            return Err(TransportError::OpenFailed(format!(
                "target render node not found: {}",
                target.render_node.display()
            )));
        }

        let bandwidth = estimate_link_bandwidth(&source, &target);
        let id = format!("pcie:{}→{}", source.pci_slot, target.pci_slot);
        let label = format!(
            "{} card{} → {} card{}",
            source.vendor, source.card_index, target.vendor, target.card_index
        );

        Ok(Self {
            info: TransportInfo {
                id,
                label,
                medium: TransportMedium::Pcie,
                direction: TransportDirection::Bidirectional,
            },
            source,
            target,
            bandwidth,
        })
    }

    /// Source endpoint.
    #[must_use]
    pub const fn source(&self) -> &PcieEndpoint {
        &self.source
    }

    /// Target endpoint.
    #[must_use]
    pub const fn target(&self) -> &PcieEndpoint {
        &self.target
    }
}

impl HardwareTransport for PcieTransport {
    fn info(&self) -> &TransportInfo {
        &self.info
    }

    fn bandwidth_bps(&self) -> u64 {
        self.bandwidth
    }

    fn is_available(&self) -> bool {
        self.source.render_node.exists() && self.target.render_node.exists()
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&self.target.render_node)
            .map_err(TransportError::Io)?;

        file.write_all(data).map_err(TransportError::Io)?;

        Ok(data.len())
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .open(&self.source.render_node)
            .map_err(TransportError::Io)?;

        let n = file.read(buf).map_err(TransportError::Io)?;

        Ok(n)
    }
}

/// Discover all `PCIe` GPU-to-GPU links on the system.
///
/// Uses the full `PCIe` topology graph to include switch hierarchy, hop count,
/// and contention factors. Returns one [`PcieLink`] for each ordered pair of
/// GPUs. A system with 2 GPUs yields 2 links (A→B and B→A).
#[must_use]
pub fn discover_pcie_links() -> Vec<PcieLink> {
    let topo = discover_topology();
    if topo.gpus.len() < 2 {
        return Vec::new();
    }

    let mut links = Vec::new();

    for source in &topo.gpus {
        for target in &topo.gpus {
            if source.card_index == target.card_index {
                continue;
            }

            let src_ep = endpoint_from_device(source);
            let tgt_ep = endpoint_from_device(target);

            let pair_topo = topo.pair(source.card_index, target.card_index);

            let bandwidth = pair_topo.map_or_else(
                || estimate_link_bandwidth(&src_ep, &tgt_ep),
                |_| topo.effective_bandwidth_bps(source.card_index, target.card_index),
            );

            let same_numa = pair_topo.map_or_else(
                || {
                    matches!(
                        (src_ep.numa_node, tgt_ep.numa_node),
                        (Some(a), Some(b)) if a >= 0 && b >= 0 && a == b
                    )
                },
                |p| p.same_numa,
            );

            let via_switch = pair_topo.and_then(|p| p.common_bridge.clone());
            let hops = pair_topo.map_or(u32::MAX, |p| p.hops);
            let contention_factor = pair_topo.map_or(1.0, |p| p.contention_factor);

            links.push(PcieLink {
                source: src_ep,
                target: tgt_ep,
                bandwidth_bps: bandwidth,
                same_numa,
                via_switch,
                hops,
                contention_factor,
            });
        }
    }

    links
}

/// Discover `PCIe` transport info for all GPU pairs (for `transport.discover`).
#[must_use]
pub fn discover_pcie_transports() -> Vec<TransportInfo> {
    discover_pcie_links()
        .into_iter()
        .map(|link| TransportInfo {
            id: format!("pcie:{}→{}", link.source.pci_slot, link.target.pci_slot),
            label: format!(
                "{} card{} → {} card{}",
                link.source.vendor,
                link.source.card_index,
                link.target.vendor,
                link.target.card_index
            ),
            medium: TransportMedium::Pcie,
            direction: TransportDirection::Bidirectional,
        })
        .collect()
}

/// Build a `PcieEndpoint` from a discovered [`GpuDevice`].
fn endpoint_from_device(gpu: &GpuDevice) -> PcieEndpoint {
    let topo = gpu.pcie_topology();
    PcieEndpoint {
        card_index: gpu.card_index,
        pci_slot: gpu.pci_slot.clone(),
        vendor: gpu.vendor,
        render_node: gpu.render_node(),
        numa_node: topo.numa_node,
        pcie_gen: topo.generation,
        pcie_width: topo.width,
    }
}

/// Estimate practical bandwidth between two endpoints based on the slower link.
///
/// Uses the minimum `PCIe` gen/width of the two endpoints, with a 0.78
/// efficiency factor (encoding overhead + protocol overhead).
fn estimate_link_bandwidth(source: &PcieEndpoint, target: &PcieEndpoint) -> u64 {
    let src_raw = raw_pcie_bandwidth_bps(source.pcie_gen, source.pcie_width);
    let tgt_raw = raw_pcie_bandwidth_bps(target.pcie_gen, target.pcie_width);

    let min_raw = src_raw.min(tgt_raw);
    if min_raw == 0 {
        return 0;
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "bandwidth * 0.78 is always positive and within u64; precision loss is acceptable for bandwidth estimates"
    )]
    {
        (min_raw as f64 * 0.78) as u64
    }
}

/// Raw (theoretical) `PCIe` bandwidth for a given generation and lane width.
fn raw_pcie_bandwidth_bps(generation: Option<u32>, width: Option<u32>) -> u64 {
    let transfer_rate_gbps: f64 = match generation {
        Some(5) => 32.0,
        Some(4) => 16.0,
        Some(3) => 8.0,
        Some(2) => 5.0,
        Some(1) => 2.5,
        _ => return 0,
    };
    let lanes = u64::from(width.unwrap_or(1));
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "transfer rate is always positive and fits u64"
    )]
    let bits_per_second = (transfer_rate_gbps * 1e9) as u64 * lanes;
    bits_per_second
}

/// Group GPUs by NUMA node for locality-aware routing.
#[must_use]
pub fn gpus_by_numa() -> HashMap<i32, Vec<PcieEndpoint>> {
    let gpus = discover_gpus();
    let mut by_numa: HashMap<i32, Vec<PcieEndpoint>> = HashMap::new();
    for gpu in &gpus {
        let ep = endpoint_from_device(gpu);
        let node = ep.numa_node.unwrap_or(-1);
        by_numa.entry(node).or_default().push(ep);
    }
    by_numa
}

#[cfg(test)]
#[path = "pcie_transport_tests.rs"]
mod tests;
