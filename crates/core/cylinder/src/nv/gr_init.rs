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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_domains() -> Vec<(&'static str, usize, usize)> {
        vec![
            ("PMC", 0x0000_0000, 0x0000_1000),
            ("PFIFO", 0x0000_2000, 0x0000_4000),
            ("PGRAPH", 0x0040_0000, 0x0042_0000),
        ]
    }

    #[test]
    fn from_bar0_diff_no_changes() {
        let cold = vec![(0, 0x1234u32), (4, 0x5678)];
        let warm = vec![(0, 0x1234u32), (4, 0x5678)];
        let seq = GrInitSequence::from_bar0_diff(
            ChipFamily::Kepler,
            &cold,
            &warm,
            &sample_domains(),
            InitSource::Manual { experiment: 196 },
        );
        assert!(seq.is_empty());
    }

    #[test]
    fn from_bar0_diff_captures_changes() {
        let cold = vec![(0x200, 0x0000_0000u32), (0x204, 0x1111_1111)];
        let warm = vec![(0x200, 0x5fec_dff1u32), (0x204, 0x1111_1111)];
        let seq = GrInitSequence::from_bar0_diff(
            ChipFamily::Volta,
            &cold,
            &warm,
            &sample_domains(),
            InitSource::NouveauDiff {
                version: "1.4.0".into(),
            },
        );
        assert_eq!(seq.len(), 1);
        assert_eq!(seq.writes[0].offset, 0x200);
        assert_eq!(seq.writes[0].value, 0x5fec_dff1);
        assert_eq!(seq.writes[0].domain, "PMC");
    }

    #[test]
    fn from_bar0_diff_skips_error_patterns() {
        let cold = vec![(0x400700, 0x0000_0000u32)];
        let warm = vec![(0x400700, 0xbadf_5040u32)];
        let seq = GrInitSequence::from_bar0_diff(
            ChipFamily::Volta,
            &cold,
            &warm,
            &sample_domains(),
            InitSource::Manual { experiment: 196 },
        );
        assert!(seq.is_empty());
    }

    #[test]
    fn filter_domain() {
        let cold = vec![(0x200, 0u32), (0x2200, 0u32), (0x400700, 0u32)];
        let warm = vec![(0x200, 1u32), (0x2200, 1u32), (0x400700, 1u32)];
        let seq = GrInitSequence::from_bar0_diff(
            ChipFamily::Kepler,
            &cold,
            &warm,
            &sample_domains(),
            InitSource::Manual { experiment: 1 },
        );
        let pmc_writes = seq.filter_domain("PMC");
        assert_eq!(pmc_writes.len(), 1);
        assert_eq!(pmc_writes[0].offset, 0x200);

        let pfifo_writes = seq.filter_domain("PFIFO");
        assert_eq!(pfifo_writes.len(), 1);
        assert_eq!(pfifo_writes[0].offset, 0x2200);
    }

    #[test]
    fn domain_summary() {
        let cold = vec![(0x100, 0u32), (0x200, 0u32), (0x2200, 0u32)];
        let warm = vec![(0x100, 1u32), (0x200, 2u32), (0x2200, 3u32)];
        let seq = GrInitSequence::from_bar0_diff(
            ChipFamily::Kepler,
            &cold,
            &warm,
            &sample_domains(),
            InitSource::Manual { experiment: 1 },
        );
        let summary = seq.domain_summary();
        assert!(!summary.is_empty());
    }

    #[test]
    fn merge_sequences() {
        let a = GrInitSequence {
            chip: ChipFamily::Kepler,
            writes: vec![
                RegWrite { offset: 0x200, value: 0xAAAA, domain: "PMC".into(), mask: None },
                RegWrite { offset: 0x300, value: 0xBBBB, domain: "PMC".into(), mask: None },
            ],
            source: InitSource::Manual { experiment: 1 },
            description: "seq A".into(),
        };
        let b = GrInitSequence {
            chip: ChipFamily::Kepler,
            writes: vec![
                RegWrite { offset: 0x200, value: 0xCCCC, domain: "PMC".into(), mask: None },
                RegWrite { offset: 0x400, value: 0xDDDD, domain: "PMC".into(), mask: None },
            ],
            source: InitSource::Manual { experiment: 2 },
            description: "seq B".into(),
        };
        let merged = a.merge(&b);
        assert_eq!(merged.len(), 3);
        let w200 = merged.writes.iter().find(|w| w.offset == 0x200).unwrap();
        assert_eq!(w200.value, 0xCCCC);
    }

    #[test]
    fn serde_roundtrip() {
        let seq = GrInitSequence {
            chip: ChipFamily::Volta,
            writes: vec![RegWrite {
                offset: 0x200,
                value: 0x5fec_dff1,
                domain: "PMC".into(),
                mask: None,
            }],
            source: InitSource::NouveauDiff { version: "1.4.0".into() },
            description: "test".into(),
        };
        let json = seq.to_json().unwrap();
        let back = GrInitSequence::from_json(&json).unwrap();
        assert_eq!(back.chip, ChipFamily::Volta);
        assert_eq!(back.writes.len(), 1);
        assert_eq!(back.writes[0].value, 0x5fec_dff1);
    }

    #[test]
    fn display_format() {
        let seq = GrInitSequence {
            chip: ChipFamily::Kepler,
            writes: vec![
                RegWrite { offset: 0x200, value: 1, domain: "PMC".into(), mask: None },
                RegWrite { offset: 0x2200, value: 1, domain: "PFIFO".into(), mask: None },
            ],
            source: InitSource::Manual { experiment: 1 },
            description: "test".into(),
        };
        let s = format!("{seq}");
        assert!(s.contains("Kepler"));
        assert!(s.contains("2 writes"));
        assert!(s.contains("2 domains"));
    }

    #[test]
    fn chip_family_from_sm() {
        assert_eq!(ChipFamily::from_sm(37), ChipFamily::Kepler);
        assert_eq!(ChipFamily::from_sm(70), ChipFamily::Volta);
        assert_eq!(ChipFamily::from_sm(120), ChipFamily::Blackwell);
    }

    #[test]
    fn chip_family_unsigned_falcon() {
        assert!(ChipFamily::Kepler.allows_unsigned_falcon());
        assert!(ChipFamily::Maxwell.allows_unsigned_falcon());
        assert!(!ChipFamily::Volta.allows_unsigned_falcon());
        assert!(!ChipFamily::Blackwell.allows_unsigned_falcon());
    }

    #[test]
    fn unknown_domain_offset() {
        let d = domain_for_offset(0xFFFF_FF00, &sample_domains());
        assert_eq!(d, "UNKNOWN");
    }

    #[test]
    fn catalyst_source_serde_roundtrip() {
        let seq = GrInitSequence {
            chip: ChipFamily::Volta,
            writes: vec![RegWrite {
                offset: 0x504000,
                value: 0x0000_8042,
                domain: "GPC".into(),
                mask: None,
            }],
            source: InitSource::Catalyst {
                driver_version: "470.256.02".into(),
                bdf: "0000:49:00.0".into(),
            },
            description: "catalyst test".into(),
        };
        let json = seq.to_json().unwrap();
        assert!(json.contains("Catalyst"));
        assert!(json.contains("470.256.02"));
        let back = GrInitSequence::from_json(&json).unwrap();
        assert_eq!(back.writes.len(), 1);
        assert_eq!(back.writes[0].offset, 0x504000);
        if let InitSource::Catalyst { driver_version, bdf } = &back.source {
            assert_eq!(driver_version, "470.256.02");
            assert_eq!(bdf, "0000:49:00.0");
        } else {
            panic!("expected Catalyst source");
        }
    }
}
