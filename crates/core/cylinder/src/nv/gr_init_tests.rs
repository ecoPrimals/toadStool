// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from gr_init.rs (S334).

use super::gr_init::*;
use crate::nv::pri::domain_for_offset;

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
            RegWrite {
                offset: 0x200,
                value: 0xAAAA,
                domain: "PMC".into(),
                mask: None,
            },
            RegWrite {
                offset: 0x300,
                value: 0xBBBB,
                domain: "PMC".into(),
                mask: None,
            },
        ],
        source: InitSource::Manual { experiment: 1 },
        description: "seq A".into(),
    };
    let b = GrInitSequence {
        chip: ChipFamily::Kepler,
        writes: vec![
            RegWrite {
                offset: 0x200,
                value: 0xCCCC,
                domain: "PMC".into(),
                mask: None,
            },
            RegWrite {
                offset: 0x400,
                value: 0xDDDD,
                domain: "PMC".into(),
                mask: None,
            },
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
        source: InitSource::NouveauDiff {
            version: "1.4.0".into(),
        },
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
            RegWrite {
                offset: 0x200,
                value: 1,
                domain: "PMC".into(),
                mask: None,
            },
            RegWrite {
                offset: 0x2200,
                value: 1,
                domain: "PFIFO".into(),
                mask: None,
            },
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
    if let InitSource::Catalyst {
        driver_version,
        bdf,
    } = &back.source
    {
        assert_eq!(driver_version, "470.256.02");
        assert_eq!(bdf, "0000:49:00.0");
    } else {
        panic!("expected Catalyst source");
    }
}
