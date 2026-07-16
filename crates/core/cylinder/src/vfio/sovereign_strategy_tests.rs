// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from sovereign_strategy.rs (S335).

use std::sync::Arc;

use super::sovereign_strategy::*;
use crate::nv::generation;

#[test]
fn kepler_strategy_from_profile() {
    let profile = generation::profile_for_sm(35);
    let bridge = Arc::new(crate::nv::gsp_bridge::NoopGspBridge::default());
    let strat = strategy_for_profile(profile, bridge, 35);

    assert_eq!(strat.family_name(), "Kepler");
    assert!(!strat.needs_cg_sweep());
    assert!(!strat.needs_pgob_before_memory());
    assert!(!strat.needs_gr_init_after_falcon());
    assert_eq!(strat.falcon_boot_style(), FalconBootStyle::DirectPio);
    assert!(strat.engine_ungate_sequences().is_none());
    assert!(strat.power_profile().rollback_on_devinit_failure);
}

#[test]
fn volta_strategy_from_profile() {
    let profile = generation::profile_for_sm(70);
    let bridge = Arc::new(crate::nv::gsp_bridge::NoopGspBridge::default());
    let strat = strategy_for_profile(profile, bridge, 70);

    assert_eq!(strat.family_name(), "Volta");
    assert!(strat.needs_cg_sweep());
    assert!(strat.needs_pgob_before_memory());
    assert!(strat.needs_gr_init_after_falcon());
    assert_eq!(strat.falcon_boot_style(), FalconBootStyle::AcrDmaHs);
    assert!(!strat.power_profile().rollback_on_devinit_failure);
}

#[test]
fn kepler_golden_sequences_wired() {
    use crate::nv::gr_init::{ChipFamily, GrInitSequence, InitSource};
    let profile = generation::profile_for_sm(35);
    let bridge = Arc::new(crate::nv::gsp_bridge::NoopGspBridge::default());
    let seq = GrInitSequence::from_bar0_diff(
        ChipFamily::Kepler,
        &[(0x400700, 0)],
        &[(0x400700, 0x42)],
        &[("PGRAPH", 0x400000, 0x420000)],
        InitSource::Manual { experiment: 212 },
    );
    let strat = NvKeplerStrategy::new(profile.clone(), bridge, 35).with_golden_sequences(vec![(
        "PGRAPH".into(),
        seq,
        Some(0x400700),
    )]);
    let seqs = strat.engine_ungate_sequences();
    assert!(seqs.is_some());
    assert_eq!(seqs.unwrap().len(), 1);
    assert_eq!(seqs.unwrap()[0].0, "PGRAPH");
}

#[test]
fn volta_golden_sequences_wired() {
    use crate::nv::gr_init::{ChipFamily, GrInitSequence, InitSource};
    let profile = generation::profile_for_sm(70);
    let bridge = Arc::new(crate::nv::gsp_bridge::NoopGspBridge::default());
    let seq = GrInitSequence::from_bar0_diff(
        ChipFamily::Volta,
        &[(0x41A004, 0)],
        &[(0x41A004, 0x1)],
        &[("PGRAPH", 0x400000, 0x420000)],
        InitSource::Manual { experiment: 212 },
    );
    let strat = NvAcrStrategy::new(profile.clone(), bridge, 70).with_golden_sequences(vec![(
        "GR_INIT".into(),
        seq,
        None,
    )]);
    let seqs = strat.engine_ungate_sequences();
    assert!(seqs.is_some());
    assert_eq!(seqs.unwrap().len(), 1);
    assert_eq!(seqs.unwrap()[0].0, "GR_INIT");
}

#[test]
fn empty_golden_sequences_returns_none() {
    let profile = generation::profile_for_sm(70);
    let bridge = Arc::new(crate::nv::gsp_bridge::NoopGspBridge::default());
    let strat = NvAcrStrategy::new(profile.clone(), bridge, 70);
    assert!(strat.engine_ungate_sequences().is_none());
}
