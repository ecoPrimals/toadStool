// SPDX-License-Identifier: AGPL-3.0-or-later

//! Automated warm state capture pipeline — cold/warm snapshot and diff.
//!
//! Orchestrates the "cold snapshot → driver bind → warm snapshot → diff →
//! GrInitSequence" pipeline. This is the tooling that lets us systematically
//! learn what a driver initializes, then encode it as a Rust-side replay.
//!
//! # Pipeline
//!
//! ```text
//! 1. Capture cold BAR0 snapshot (device bound to vfio-pci, pre-driver)
//! 2. Swap to kernel driver (nouveau/nvidia) via glowplug
//! 3. Driver initializes GPU
//! 4. Swap back to vfio-pci (FLR disabled)
//! 5. Capture warm BAR0 snapshot
//! 6. Diff cold vs warm
//! 7. Build GrInitSequence from diff
//! ```
//!
//! Steps 2-4 are orchestrated externally (by glowplug). This module owns
//! the snapshot, diff, and sequence extraction logic.

use serde::{Deserialize, Serialize};

use crate::nv::gr_init::{ChipFamily, GrInitSequence, InitSource};
use crate::nv::pri::is_error_or_zero;
use crate::vfio::bar_cartography::{BarMap, diff_bar_maps, diff_snapshots, snapshot_registers};
use crate::vfio::device::MappedBar;

/// A point-in-time capture of BAR0 register state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar0Snapshot {
    /// BDF of the device (e.g. "0000:41:00.0").
    pub bdf: String,
    /// Label for this capture (e.g. "cold", "nouveau-warm", "nvidia-warm").
    pub label: String,
    /// Per-register (offset, value) pairs.
    pub registers: Vec<(usize, u32)>,
    /// Timestamp of capture (epoch millis).
    pub timestamp_ms: u64,
}

impl Bar0Snapshot {
    /// Capture a snapshot of specific BAR0 offsets.
    pub fn capture(bar0: &MappedBar, bdf: &str, label: &str, offsets: &[usize]) -> Self {
        let registers = snapshot_registers(bar0, offsets);
        Self {
            bdf: bdf.to_string(),
            label: label.to_string(),
            registers,
            timestamp_ms: epoch_millis(),
        }
    }

    /// Capture a full-range snapshot (every 4 bytes up to `scan_size`).
    pub fn capture_full(bar0: &MappedBar, bdf: &str, label: &str, scan_size: usize) -> Self {
        let offsets: Vec<usize> = (0..scan_size).step_by(4).collect();
        Self::capture(bar0, bdf, label, &offsets)
    }

    /// Capture only known register domains (skipping dead inter-domain gaps).
    ///
    /// Reading unmapped MMIO regions causes PCIe completion timeouts (~100μs
    /// each). A full 16MB scan hits 4M reads, ~7 minutes on GV100. Scanning
    /// only known domains (641K regs for Volta) finishes in seconds.
    pub fn capture_domains(
        bar0: &MappedBar,
        bdf: &str,
        label: &str,
        domains: &[(&str, usize, usize)],
    ) -> Self {
        let mut offsets = Vec::new();
        for &(_, start, end) in domains {
            offsets.extend((start..end).step_by(4));
        }
        Self::capture(bar0, bdf, label, &offsets)
    }

    /// Number of registers captured.
    pub fn len(&self) -> usize {
        self.registers.len()
    }

    /// Whether the snapshot is empty.
    pub fn is_empty(&self) -> bool {
        self.registers.is_empty()
    }

    /// Count of non-error, non-zero registers (alive registers).
    pub fn alive_count(&self) -> usize {
        self.registers
            .iter()
            .filter(|(_, v)| !is_error_or_zero(*v))
            .count()
    }

    /// Convert a catalyst snapshot into a replay sequence.
    ///
    /// Filters to non-zero, non-PRI-fault registers — the catalyst's
    /// product. The resulting `GrInitSequence` can be persisted as JSON
    /// and replayed on future boots without the catalyst driver.
    pub fn to_catalyst_replay(
        &self,
        chip: ChipFamily,
        driver_version: &str,
        domains: &[(&str, usize, usize)],
    ) -> GrInitSequence {
        use crate::nv::gr_init::{InitSource, RegWrite};
        use crate::nv::pri::{domain_for_offset, is_pri_fault};

        let writes: Vec<RegWrite> = self
            .registers
            .iter()
            .filter(|(_, v)| *v != 0 && !is_pri_fault(*v))
            .map(|(off, val)| RegWrite {
                offset: *off as u32,
                value: *val,
                domain: domain_for_offset(*off, domains),
                mask: None,
            })
            .collect();

        let description = format!(
            "{chip:?} catalyst replay: {} writes from {} capture of {}",
            writes.len(),
            driver_version,
            self.bdf,
        );

        GrInitSequence {
            chip,
            writes,
            source: InitSource::Catalyst {
                driver_version: driver_version.to_string(),
                bdf: self.bdf.clone(),
            },
            description,
        }
    }

    /// Serialize to JSON for archival.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Diff between two BAR0 snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar0Diff {
    /// Cold snapshot label.
    pub cold_label: String,
    /// Warm snapshot label.
    pub warm_label: String,
    /// Registers that changed: (offset, cold_value, warm_value).
    pub changed: Vec<(usize, u32, u32)>,
    /// Registers that stayed the same.
    pub unchanged_count: usize,
    /// Total registers compared.
    pub total_compared: usize,
}

impl Bar0Diff {
    /// Compute the diff between two snapshots.
    pub fn from_snapshots(cold: &Bar0Snapshot, warm: &Bar0Snapshot) -> Self {
        let changed = diff_snapshots(&cold.registers, &warm.registers);
        let total_compared = cold.registers.len().min(warm.registers.len());

        Self {
            cold_label: cold.label.clone(),
            warm_label: warm.label.clone(),
            unchanged_count: total_compared.saturating_sub(changed.len()),
            changed,
            total_compared,
        }
    }

    /// Number of registers that changed.
    pub fn changed_count(&self) -> usize {
        self.changed.len()
    }

    /// Filter changed registers to a BAR0 offset range.
    pub fn in_range(&self, start: usize, end: usize) -> Vec<&(usize, u32, u32)> {
        self.changed
            .iter()
            .filter(|(off, _, _)| *off >= start && *off < end)
            .collect()
    }

    /// Convert a cold-vs-catalyst diff into a minimal replay sequence.
    ///
    /// Only includes registers that changed to non-zero, non-PRI-fault
    /// values — the catalyst's product. Uses the warm (catalyst) value.
    pub fn to_replay_sequence(
        &self,
        chip: ChipFamily,
        source: InitSource,
        domains: &[(&str, usize, usize)],
    ) -> GrInitSequence {
        use crate::nv::gr_init::RegWrite;
        use crate::nv::pri::{domain_for_offset, is_pri_fault};

        let writes: Vec<RegWrite> = self
            .changed
            .iter()
            .filter(|(_, _, warm_val)| *warm_val != 0 && !is_pri_fault(*warm_val))
            .map(|(off, _, warm_val)| RegWrite {
                offset: *off as u32,
                value: *warm_val,
                domain: domain_for_offset(*off, domains),
                mask: None,
            })
            .collect();

        let description = format!(
            "{chip:?} catalyst delta: {} writes from {} changed registers ({} → {})",
            writes.len(),
            self.changed.len(),
            self.cold_label,
            self.warm_label,
        );

        GrInitSequence {
            chip,
            writes,
            source,
            description,
        }
    }

    /// Serialize to JSON for archival.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Complete warm state capture: cold snapshot + warm snapshot + diff +
/// derived `GrInitSequence`.
#[derive(Debug, Clone)]
pub struct WarmStateCapture {
    /// BDF of the captured device.
    pub bdf: String,
    /// Cold (pre-driver) BAR0 snapshot.
    pub cold_snapshot: Bar0Snapshot,
    /// Warm (post-driver) BAR0 snapshot.
    pub warm_snapshot: Bar0Snapshot,
    /// Computed diff between cold and warm.
    pub diff: Bar0Diff,
    /// Derived GR init sequence from the diff.
    pub gr_init: GrInitSequence,
}

impl WarmStateCapture {
    /// Build a complete capture from two snapshots.
    ///
    /// `domains` maps BAR0 offset ranges to domain names for the
    /// `GrInitSequence` labeling.
    pub fn from_snapshots(
        cold: Bar0Snapshot,
        warm: Bar0Snapshot,
        chip: ChipFamily,
        source: InitSource,
        domains: &[(&str, usize, usize)],
    ) -> Self {
        let diff = Bar0Diff::from_snapshots(&cold, &warm);
        let gr_init =
            GrInitSequence::from_bar0_diff(chip, &cold.registers, &warm.registers, domains, source);

        Self {
            bdf: cold.bdf.clone(),
            cold_snapshot: cold,
            warm_snapshot: warm,
            diff,
            gr_init,
        }
    }

    /// Build from a `BarMapDiff` (full cartography-level diff).
    ///
    /// Converts the cartography diff into the snapshot-based format,
    /// producing a `GrInitSequence` from the woke_up + value_changed
    /// entries.
    pub fn from_bar_map_diff(
        bdf: &str,
        cold_map: &BarMap,
        warm_map: &BarMap,
        chip: ChipFamily,
        source: InitSource,
        domains: &[(&str, usize, usize)],
    ) -> Self {
        let bar_diff = diff_bar_maps(cold_map, warm_map);

        let cold_regs: Vec<(usize, u32)> = cold_map
            .register_map
            .iter()
            .map(|(&off, probe)| (off, probe.read1))
            .collect();
        let warm_regs: Vec<(usize, u32)> = warm_map
            .register_map
            .iter()
            .map(|(&off, probe)| (off, probe.read1))
            .collect();

        let cold_snapshot = Bar0Snapshot {
            bdf: bdf.to_string(),
            label: "cold (BarMap)".into(),
            registers: cold_regs,
            timestamp_ms: epoch_millis(),
        };

        let warm_snapshot = Bar0Snapshot {
            bdf: bdf.to_string(),
            label: "warm (BarMap)".into(),
            registers: warm_regs,
            timestamp_ms: epoch_millis(),
        };

        let changed: Vec<(usize, u32, u32)> = bar_diff
            .value_changed
            .iter()
            .copied()
            .chain(bar_diff.woke_up.iter().map(|&(off, val)| (off, 0, val)))
            .collect();

        let diff = Bar0Diff {
            cold_label: cold_snapshot.label.clone(),
            warm_label: warm_snapshot.label.clone(),
            total_compared: cold_snapshot.registers.len(),
            unchanged_count: bar_diff.unchanged,
            changed,
        };

        let gr_init = GrInitSequence::from_bar0_diff(
            chip,
            &cold_snapshot.registers,
            &warm_snapshot.registers,
            domains,
            source,
        );

        Self {
            bdf: bdf.to_string(),
            cold_snapshot,
            warm_snapshot,
            diff,
            gr_init,
        }
    }

    /// Summary string for logging.
    pub fn summary(&self) -> String {
        format!(
            "WarmStateCapture({bdf}): {cold_alive} alive (cold) → {warm_alive} alive (warm), \
             {changed} changed, {writes} GR writes across {domains} domains",
            bdf = self.bdf,
            cold_alive = self.cold_snapshot.alive_count(),
            warm_alive = self.warm_snapshot.alive_count(),
            changed = self.diff.changed_count(),
            writes = self.gr_init.len(),
            domains = self.gr_init.domains().len(),
        )
    }
}

impl std::fmt::Display for WarmStateCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

fn epoch_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
