// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from hardware_guard.rs (S335).

use super::hardware_guard::*;

use crate::vfio::device::MappedBar;

fn make_test_bar(boot0: u32) -> MappedBar {
    let mut data = vec![0u8; 0x100_0000]; // 16 MiB mock BAR
    data[0..4].copy_from_slice(&boot0.to_le_bytes());
    MappedBar::from_test_heap(data.into_boxed_slice())
}

#[test]
fn dead_gpu_refuses_construction() {
    let bar = make_test_bar(0xFFFF_FFFF);
    assert!(GuardedBar::new(&bar, 16).is_err());
}

#[test]
fn zero_boot0_refuses_construction() {
    let bar = make_test_bar(0);
    assert!(GuardedBar::new(&bar, 16).is_err());
}

#[test]
fn valid_boot0_constructs() {
    let bar = make_test_bar(0x0f22_d0a1);
    assert!(GuardedBar::new(&bar, 16).is_ok());
}

#[test]
fn blocked_register_refused() {
    let bar = make_test_bar(0x0f22_d0a1);
    let guard = GuardedBar::new(&bar, 0).unwrap();
    let result = guard.write_u32(0x13_8020, 0x42);
    assert!(result.is_err());
    match result.unwrap_err() {
        GuardRefusal::BlockedRegister { offset, .. } => {
            assert_eq!(offset, 0x13_8020);
        }
        other => panic!("expected BlockedRegister, got {other}"),
    }
}

#[test]
fn safe_register_allowed() {
    let bar = make_test_bar(0x0f22_d0a1);
    let guard = GuardedBar::new(&bar, 0).unwrap();
    assert!(guard.write_u32(PMC_ENABLE, 0x42).is_ok());
}

#[test]
fn fecs_pio_blocked_when_pgraph_off() {
    let bar = make_test_bar(0x0f22_d0a1);
    // PMC_ENABLE is zero → PGRAPH disabled → PIO blocked
    let guard = GuardedBar::new(&bar, 0).unwrap();
    assert!(guard.write_u32(0x40_9180, 0).is_err()); // FECS IMEMC
    assert!(guard.write_u32(0x40_91A0, 0).is_err()); // FECS IMEMD
    assert!(guard.write_u32(0x41_A184, 0).is_err()); // GPCCS PIO
}

#[test]
fn fecs_pio_allowed_when_pgraph_on() {
    let bar = make_test_bar(0x0f22_d0a1);
    // Set PMC_ENABLE PGRAPH bit → PIO should be allowed
    let _ = bar.write_u32(PMC_ENABLE as usize, PGRAPH_BIT);
    let guard = GuardedBar::new(&bar, 0).unwrap();
    assert!(guard.write_u32(0x40_9180, 0).is_ok()); // FECS IMEMC
    assert!(guard.write_u32(0x41_A184, 0).is_ok()); // GPCCS PIO
}

#[test]
fn write_count_increments() {
    let bar = make_test_bar(0x0f22_d0a1);
    let guard = GuardedBar::new(&bar, 0).unwrap();
    let _ = guard.write_u32(0x400, 1);
    let _ = guard.write_u32(0x404, 2);
    assert_eq!(guard.write_count(), 2);
}

#[test]
fn canary_detects_corruption() {
    let bar = make_test_bar(0x0f22_d0a1);
    let guard = GuardedBar::new(&bar, 0).unwrap();
    // Corrupt BOOT0 via raw bar0 write
    let _ = bar.write_u32(0, 0xDEAD_BEEF);
    let result = guard.check_canary();
    assert!(result.is_err());
    assert!(!guard.is_alive());
}
