// SPDX-License-Identifier: AGPL-3.0-or-later

//! GR initialization sequence — capture and replay of PGRAPH register state.
//!
//! A `GrInitSequence` is an ordered list of BAR0 register writes that
//! reproduces a driver's GPU initialization. Built from BAR0 snapshot
//! diffs (cold vs warm), it encodes what a driver does so we can replay
//! it in pure Rust — eliminating the driver dependency.
//!
//! # Strategy
//!
//! ```text
//! 1. Cold state (VFIO)  → BAR0 snapshot (baseline)
//! 2. Driver binds       → BAR0 snapshot (warm)
//! 3. Diff               → changed registers
//! 4. GrInitSequence     → ordered write list
//! 5. Apply from cold    → reproduce warm state in Rust
//! ```

use serde::{Deserialize, Serialize};

use crate::nv::generation::BootStrategy;
use crate::nv::pri::{domain_for_offset, is_pri_fault};
use crate::vfio::device::MappedBar;

/// GPU architecture family for init sequence selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChipFamily {
    /// GK110/GK210 — Tesla K80, K40. No ACR/HS, direct PIO falcon.
    Kepler,
    /// GM200/GM204 — GTX 980 Ti, Titan X (Maxwell).
    Maxwell,
    /// GP100/GP10x — GTX 1080, Tesla P100. ACR via SEC2.
    Pascal,
    /// GV100 — Titan V, Tesla V100. ACR via SEC2, HS fuse.
    Volta,
    /// TU102/TU104 — RTX 2080, Tesla T4.
    Turing,
    /// GA100/GA102 — A100, RTX 3090.
    Ampere,
    /// AD102/AD103 — RTX 4090.
    Ada,
    /// GH100 — H100.
    Hopper,
    /// GB100/GB202 — B100, RTX 5090, RTX 5060.
    Blackwell,
}

impl ChipFamily {
    /// Derive chip family from a `GenerationProfile`.
    ///
    /// This is the preferred constructor — it keeps the SM→family mapping
    /// consistent with the single source of truth in `generation.rs`.
    pub fn from_profile(profile: &crate::nv::generation::GenerationProfile) -> Self {
        match profile.name {
            "Kepler" => Self::Kepler,
            "Maxwell" => Self::Maxwell,
            "Pascal" => Self::Pascal,
            "Volta" => Self::Volta,
            "Turing" => Self::Turing,
            "Ampere A" | "Ampere B" => Self::Ampere,
            "Ada" => Self::Ada,
            "Hopper" => Self::Hopper,
            _ => Self::Blackwell,
        }
    }

    /// Derive chip family from SM version.
    ///
    /// Delegates to `profile_for_sm` to stay consistent with
    /// `GenerationProfile` SM ranges.
    pub fn from_sm(sm: u32) -> Self {
        let profile = crate::nv::generation::profile_for_sm(sm);
        Self::from_profile(profile)
    }

    /// Whether this family allows unsigned falcon firmware (no ACR/HS).
    pub fn allows_unsigned_falcon(&self) -> bool {
        matches!(self, Self::Kepler | Self::Maxwell)
    }

    /// The boot strategy for this family.
    pub fn boot_strategy(&self) -> BootStrategy {
        match self {
            Self::Kepler => BootStrategy::NoAcr,
            Self::Maxwell => BootStrategy::Untested,
            Self::Blackwell => BootStrategy::KmodPromote,
            _ => BootStrategy::AcrSec2,
        }
    }

    /// Default engine name for ungating sequences derived from this family.
    pub fn engine_label(&self) -> String {
        match self {
            Self::Kepler => "PGRAPH".into(),
            _ => "GR_INIT".into(),
        }
    }
}

/// A single register write in an init sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegWrite {
    /// BAR0 offset.
    pub offset: u32,
    /// Value to write.
    pub value: u32,
    /// BAR0 domain name (from `NV_BAR0_DOMAINS`).
    pub domain: String,
    /// Optional mask for read-modify-write. If `Some(mask)`, the write
    /// is `(current & !mask) | (value & mask)` rather than a direct write.
    pub mask: Option<u32>,
}

/// Where an init sequence was derived from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InitSource {
    /// Derived from a nouveau driver's warm state diff.
    NouveauDiff {
        /// Kernel module version string.
        version: String,
    },
    /// Derived from an nvidia vendor driver's warm state diff.
    NvidiaDiff {
        /// Driver version (e.g. "470.256.02").
        version: String,
    },
    /// Manually constructed from experiment observations.
    Manual {
        /// Experiment number from hotSpring.
        experiment: u16,
    },
    /// Merged from multiple sources.
    Merged {
        /// Source descriptions.
        sources: Vec<String>,
    },
    /// Captured from a catalyst driver session (proprietary driver used
    /// to initialize GPU, then removed). The catalyst's product — the
    /// register state — is preserved and replayed without the driver.
    Catalyst {
        /// Driver version used as catalyst (e.g. "470.256.02").
        driver_version: String,
        /// BDF of the GPU that was catalyzed.
        bdf: String,
    },
}

/// An ordered sequence of BAR0 register writes that initializes the
/// GR engine (and related subsystems) on an NVIDIA GPU.
///
/// Built from BAR0 diffs between cold and warm states, this is the
/// Rust-side replacement for what a Linux kernel driver does during
/// `probe()` / `init()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrInitSequence {
    /// Target GPU family.
    pub chip: ChipFamily,
    /// Ordered register writes.
    pub writes: Vec<RegWrite>,
    /// Provenance of this sequence.
    pub source: InitSource,
    /// Human-readable description.
    pub description: String,
}

impl GrInitSequence {
    /// Construct from a BAR0 diff (cold snapshot vs warm snapshot).
    ///
    /// `domains` maps BAR0 offset ranges to domain names for labeling.
    /// Each changed register becomes a write entry, ordered by offset.
    pub fn from_bar0_diff(
        chip: ChipFamily,
        cold: &[(usize, u32)],
        warm: &[(usize, u32)],
        domains: &[(&str, usize, usize)],
        source: InitSource,
    ) -> Self {
        let mut writes = Vec::new();

        for (c, w) in cold.iter().zip(warm.iter()) {
            debug_assert_eq!(c.0, w.0, "snapshot offset mismatch");
            let offset = c.0;

            if c.1 == w.1 {
                continue;
            }

            if is_pri_fault(w.1) {
                continue;
            }

            let domain = domain_for_offset(offset, domains);

            writes.push(RegWrite {
                offset: offset as u32,
                value: w.1,
                domain,
                mask: None,
            });
        }

        let description = format!(
            "{chip:?} GR init: {} register writes from {:?}",
            writes.len(),
            source,
        );

        Self {
            chip,
            writes,
            source,
            description,
        }
    }

    /// Number of register writes in this sequence.
    pub fn len(&self) -> usize {
        self.writes.len()
    }

    /// Whether this sequence has no writes.
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }

    /// Extract writes for a single BAR0 domain.
    pub fn filter_domain(&self, name: &str) -> Vec<&RegWrite> {
        self.writes.iter().filter(|w| w.domain == name).collect()
    }

    /// List all domains that have writes in this sequence.
    pub fn domains(&self) -> Vec<String> {
        let mut seen = Vec::new();
        for w in &self.writes {
            if !seen.contains(&w.domain) {
                seen.push(w.domain.clone());
            }
        }
        seen
    }

    /// Summary: count of writes per domain.
    pub fn domain_summary(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for w in &self.writes {
            if let Some(entry) = counts.iter_mut().find(|(d, _)| d == &w.domain) {
                entry.1 += 1;
            } else {
                counts.push((w.domain.clone(), 1));
            }
        }
        counts
    }

    /// Merge two init sequences, preferring `other` for conflicting offsets.
    pub fn merge(&self, other: &Self) -> Self {
        let mut writes = self.writes.clone();

        for ow in &other.writes {
            if let Some(existing) = writes.iter_mut().find(|w| w.offset == ow.offset) {
                existing.value = ow.value;
                existing.mask = ow.mask;
            } else {
                writes.push(ow.clone());
            }
        }

        writes.sort_by_key(|w| w.offset);

        Self {
            chip: self.chip,
            writes,
            source: InitSource::Merged {
                sources: vec![self.description.clone(), other.description.clone()],
            },
            description: format!(
                "Merged {chip:?} init: {desc_a} + {desc_b}",
                chip = self.chip,
                desc_a = self.description,
                desc_b = other.description,
            ),
        }
    }

    /// Replay this init sequence onto hardware via BAR0 MMIO writes.
    ///
    /// Each `RegWrite` is applied in order. If `mask` is set, the write
    /// is a read-modify-write: `(current & !mask) | (value & mask)`.
    /// Returns the count of writes applied, or an error if any write fails.
    pub fn apply(&self, bar0: &MappedBar) -> Result<usize, String> {
        for (i, w) in self.writes.iter().enumerate() {
            let val = match w.mask {
                Some(mask) => {
                    let current = bar0
                        .read_u32(w.offset as usize)
                        .map_err(|e| format!("read failed at offset {:#x}: {e}", w.offset))?;
                    (current & !mask) | (w.value & mask)
                }
                None => w.value,
            };

            bar0.write_u32(w.offset as usize, val)
                .map_err(|e| format!("write #{i} failed at offset {:#x}: {e}", w.offset))?;
        }
        Ok(self.writes.len())
    }

    /// Read back registers and compare against expected values.
    ///
    /// Returns a list of mismatches: `(offset, expected, actual)`.
    /// An empty result means all registers match.
    pub fn validate(&self, bar0: &MappedBar) -> Vec<(u32, u32, u32)> {
        let mut mismatches = Vec::new();
        for w in &self.writes {
            let actual = bar0.read_u32(w.offset as usize).unwrap_or(0xDEAD_DEAD);
            let expected = match w.mask {
                Some(mask) => w.value & mask,
                None => w.value,
            };
            let actual_masked = match w.mask {
                Some(mask) => actual & mask,
                None => actual,
            };
            if expected != actual_masked {
                mismatches.push((w.offset, expected, actual));
            }
        }
        mismatches
    }

    /// Serialize to JSON for storage/handoff.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl std::fmt::Display for GrInitSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GrInitSequence({:?}, {} writes, {} domains)",
            self.chip,
            self.writes.len(),
            self.domains().len(),
        )
    }
}

// PRI fault detection and domain lookup delegated to crate::nv::pri
