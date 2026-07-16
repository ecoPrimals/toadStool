// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from generation/mod.rs (S334).

use super::*;

#[test]
fn kepler_k80_profile() {
    let p = profile_for_sm(37);
    assert_eq!(p.name, "Kepler");
    assert_eq!(p.compute_class, 0xA1C0);
    assert_eq!(p.qmd_version, QmdVersion::V21);
    assert_eq!(p.boot_strategy, BootStrategy::NoAcr);
    assert_eq!(p.memory_type, MemoryType::Gddr5);
    assert!(p.has_hardware_f64_rcp);
    assert!(p.userd_gp_get);
}

#[test]
fn volta_titanv_profile() {
    let p = profile_for_sm(70);
    assert_eq!(p.name, "Volta");
    assert_eq!(p.compute_class, 0xC3C0);
    assert_eq!(p.qmd_version, QmdVersion::V22);
    assert_eq!(p.boot_strategy, BootStrategy::AcrSec2);
    assert_eq!(p.local_mem_window, 0xFF00_0000_0000_0000);
    assert!(p.has_hardware_f64_rcp);
}

#[test]
fn blackwell_5060_profile() {
    let p = profile_for_sm(120);
    assert_eq!(p.name, "Blackwell B");
    assert_eq!(p.compute_class, 0xCEC0);
    assert_eq!(p.channel_class, 0xC96F); // CUDA R580 trace: Blackwell uses 0xC96F
    assert_eq!(p.qmd_version, QmdVersion::V50);
    assert_eq!(p.qmd_word_count, 96);
    assert_eq!(p.completion, CompletionStrategy::SemaphoreFence);
    assert_eq!(p.nctaid_source, NctaidSource::DriverCbuf7);
    assert!(!p.has_hardware_f64_rcp);
    assert!(!p.userd_gp_get);
}

#[test]
fn turing_profile() {
    let p = profile_for_sm(75);
    assert_eq!(p.name, "Turing");
    assert_eq!(p.compute_class, 0xC5C0);
    assert_eq!(p.launch_method, LaunchMethod::Pcas);
}

#[test]
fn ampere_split() {
    let a = profile_for_sm(80);
    assert_eq!(a.name, "Ampere A");
    assert_eq!(a.compute_class, 0xC6C0);

    let b = profile_for_sm(86);
    assert_eq!(b.name, "Ampere B");
    assert_eq!(b.compute_class, 0xC7C0);

    assert_eq!(a.launch_method, LaunchMethod::Pcas2);
    assert_eq!(b.launch_method, LaunchMethod::Pcas2);
}

#[test]
fn ada_profile() {
    let p = profile_for_sm(89);
    assert_eq!(p.name, "Ada");
    assert_eq!(p.compute_class, 0xC9C0);
}

#[test]
fn hopper_profile() {
    let p = profile_for_sm(90);
    assert_eq!(p.name, "Hopper");
    assert_eq!(p.compute_class, 0xCBC0);
}

#[test]
fn blackwell_datacenter_profile() {
    let p = profile_for_sm(100);
    assert_eq!(p.name, "Blackwell A");
    assert_eq!(p.compute_class, 0xCDC0);
    assert_eq!(p.channel_class, 0xC96F);
}

#[test]
fn unknown_sm_falls_back_to_volta() {
    let p = profile_for_sm(999);
    assert_eq!(p.name, "Blackwell B");
}

#[test]
fn all_profiles_cover_known_generations() {
    let known_sms = [35, 37, 50, 60, 70, 75, 80, 86, 89, 90, 100, 120];
    for sm in known_sms {
        let p = profile_for_sm(sm);
        assert!(
            p.sm_range.contains(&sm),
            "SM {sm} should be in range {:?} ({})",
            p.sm_range,
            p.name
        );
    }
}

/// Profile compute classes are the authoritative source; the legacy
/// identity table (`sm_to_compute_class`) is coarser-grained and
/// incorrect for SM 80+ (Ada/Hopper/Blackwell have wrong class IDs).
/// Once the identity table delegates through `profile_for_sm`, all
/// SM values will match. For now, only verify Kepler–Turing.
#[test]
fn compute_class_matches_identity_table_where_aligned() {
    use crate::nv::identity::sm_to_compute_class;
    let aligned_sms = [35, 50, 60, 70, 75, 80];
    for sm in aligned_sms {
        let profile_class = profile_for_sm(sm).compute_class;
        let identity_class = sm_to_compute_class(sm);
        assert_eq!(
            profile_class, identity_class,
            "SM {sm}: profile={profile_class:#06X} vs identity={identity_class:#06X}"
        );
    }
}

#[test]
fn firmware_chip_matches_identity() {
    use crate::nv::identity::chip_name;
    let sms = [35, 50, 60, 70, 75, 80, 86, 89, 90, 100, 120];
    for sm in sms {
        let profile_chip = profile_for_sm(sm).firmware_chip;
        let identity_chip = chip_name(sm);
        assert_eq!(
            profile_chip, identity_chip,
            "SM {sm}: profile={profile_chip} vs identity={identity_chip}"
        );
    }
}

#[test]
fn kepler_uses_v1_two_level_pt() {
    let p = profile_for_sm(37);
    assert_eq!(p.page_table_format, PageTableFormat::V1TwoLevel);
    assert_eq!(p.instance_block_format, InstanceBlockFormat::Simple);
    assert_eq!(p.runlist_format, RunlistFormat::Gk104Global);
}

#[test]
fn volta_uses_v2_five_level_pt() {
    let p = profile_for_sm(70);
    assert_eq!(p.page_table_format, PageTableFormat::V2FiveLevel);
    assert_eq!(p.instance_block_format, InstanceBlockFormat::Subcontexted);
    assert_eq!(p.runlist_format, RunlistFormat::Gv100PerRunlist);
}

#[test]
fn pascal_uses_v2_pt_simple_instance() {
    let p = profile_for_sm(60);
    assert_eq!(p.page_table_format, PageTableFormat::V2FiveLevel);
    assert_eq!(p.instance_block_format, InstanceBlockFormat::Simple);
    assert_eq!(p.runlist_format, RunlistFormat::Gk104Global);
}

#[test]
fn tier_offsets_present_on_all_profiles() {
    let sms = [35, 50, 60, 70, 75, 80, 86, 89, 90, 100, 120];
    for sm in sms {
        let p = profile_for_sm(sm);
        assert_eq!(p.fecs_pc_offset, 0x0040_9624, "SM {sm}: FECS PC offset");
        assert_eq!(
            p.gpc_broadcast_offset, 0x0041_A004,
            "SM {sm}: GPC broadcast offset"
        );
        assert_eq!(p.ce0_base_offset, 0x0010_4000, "SM {sm}: CE0 base offset");
        assert_eq!(
            p.pgraph_status_offset, 0x0040_0700,
            "SM {sm}: PGRAPH status offset"
        );
        assert!(p.ce_class != 0, "SM {sm}: CE class should be non-zero");
        assert_eq!(p.ptop_device_info_base, 0x0002_2700, "SM {sm}: PTOP base");
        assert_eq!(
            p.runlist_pbdma_map_base, 0x0000_2390,
            "SM {sm}: PBDMA map base"
        );
    }
}

#[test]
fn ce_class_varies_by_generation() {
    assert_eq!(profile_for_sm(35).ce_class, 0xA0B5);
    assert_eq!(profile_for_sm(70).ce_class, 0xC3B5);
    assert_eq!(profile_for_sm(75).ce_class, 0xC5B5);
}

#[test]
fn is_kepler_helper() {
    assert!(is_kepler(profile_for_sm(35)));
    assert!(is_kepler(profile_for_sm(37)));
    assert!(!is_kepler(profile_for_sm(70)));
    assert!(!is_kepler(profile_for_sm(120)));
}

#[test]
fn uses_semaphore_fence_helper() {
    assert!(!uses_semaphore_fence(profile_for_sm(70)));
    assert!(!uses_semaphore_fence(profile_for_sm(89)));
    assert!(uses_semaphore_fence(profile_for_sm(100)));
    assert!(uses_semaphore_fence(profile_for_sm(120)));
}
