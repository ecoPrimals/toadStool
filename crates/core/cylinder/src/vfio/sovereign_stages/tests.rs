// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{detect_chip, AMD_GRBM_STATUS, ChipDetection};
use crate::vfio::device::MappedBar;

fn test_bar_with(boot0: u32, grbm: u32) -> MappedBar {
    let mut data = vec![0u8; AMD_GRBM_STATUS as usize + 4];
    data[0..4].copy_from_slice(&boot0.to_le_bytes());
    data[AMD_GRBM_STATUS as usize..AMD_GRBM_STATUS as usize + 4]
        .copy_from_slice(&grbm.to_le_bytes());
    MappedBar::from_test_heap(data.into_boxed_slice())
}

#[test]
fn detect_chip_nvidia_from_boot0() {
    let bar = test_bar_with(0x1400_0000, 0);
    match detect_chip(&bar) {
        ChipDetection::Nvidia { chip, sm } => {
            assert_eq!(chip, "gv100");
            assert_eq!(sm, 70);
        }
        other => panic!("expected Nvidia, got {other:?}"),
    }
}

#[test]
fn detect_chip_amd_from_grbm() {
    let bar = test_bar_with(0xFFFF_FFFF, 0x0000_3000);
    match detect_chip(&bar) {
        ChipDetection::AmdPresent { family, grbm_status } => {
            assert_eq!(family, "Vega 20");
            assert_eq!(grbm_status, 0x0000_3000);
            assert!(detect_chip(&bar).diagnostic().contains("cold boot not implemented"));
        }
        other => panic!("expected AmdPresent, got {other:?}"),
    }
}

#[test]
fn detect_chip_not_found() {
    let bar = test_bar_with(0xFFFF_FFFF, 0xFFFF_FFFF);
    match detect_chip(&bar) {
        ChipDetection::NotFound { .. } => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
    assert!(detect_chip(&bar).diagnostic().contains("no GPU found"));
}
