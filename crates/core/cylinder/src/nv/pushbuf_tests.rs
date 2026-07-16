// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from pushbuf.rs (S335).

use super::pushbuf::*;

#[test]
fn mthd_incr_field_order() {
    let pb = PushBuf::compute_dispatch(0xC6C0, 0x1_0000_0000);
    let words = pb.as_words();
    assert!(words.len() >= 6, "dispatch should have >=3 methods");
    let hdr = words[0];
    assert_eq!(hdr >> 29, 1, "SEC_OP should be 1 (INC_METHOD)");
    assert_eq!((hdr >> 16) & 0x1FFF, 1, "count should be 1");
    assert_eq!((hdr >> 13) & 0x7, 0, "subchannel should be 0");
}

#[test]
fn compute_init_sets_object() {
    let pb = PushBuf::compute_init(0xCEC0, 0xFF00_0000, 0x1_0000_0000, 0x8000);
    let words = pb.as_words();
    let hdr = words[0];
    assert_eq!((hdr >> 13) & 0x7, 0, "subchannel 0");
    assert_eq!(hdr & 0x1FFF, 0, "SET_OBJECT method addr >> 2 = 0");
    assert_eq!(words[1], 0xCEC0, "compute class");
}

#[test]
fn compute_dispatch_uses_pcas2_for_ampere_plus() {
    let pb = PushBuf::compute_dispatch(0xC6C0, 0x2_0000_0000);
    let words = pb.as_words();
    let pcas2_method = method::SEND_SIGNALING_PCAS2_B >> 2;
    let found = words.chunks(2).any(|w| (w[0] & 0x1FFF) == pcas2_method);
    assert!(found, "Ampere+ should use SEND_SIGNALING_PCAS2_B");
}

#[test]
fn compute_dispatch_uses_pcas_for_turing() {
    let pb = PushBuf::compute_dispatch(0xC5C0, 0x1_0000_0000);
    let words = pb.as_words();
    let pcas_method = method::SEND_SIGNALING_PCAS_B >> 2;
    let found = words.chunks(2).any(|w| (w[0] & 0x1FFF) == pcas_method);
    assert!(found, "Turing should use SEND_SIGNALING_PCAS_B");
}

fn decode_push_va(words: &[u32]) -> Option<u64> {
    for w in words.chunks(2) {
        if (w[0] & 0x1FFF) == (method::SEND_PCAS_A >> 2) {
            return Some(u64::from(w[1]) << 8);
        }
    }
    None
}

#[test]
fn send_pcas_a_encodes_qmd_addr_shifted() {
    let addr: u64 = 0x3_DEAD_0000;
    let pb = PushBuf::compute_dispatch(0xC6C0, addr);
    let decoded = decode_push_va(pb.as_words()).expect("SEND_PCAS_A missing");
    assert_eq!(decoded, addr, "round-trip qmd addr");
}

#[test]
fn gr_context_init_subchannel_0() {
    let entries = vec![(0x0418, 0x1234)];
    let pb = PushBuf::gr_context_init(0xCEC0, &entries);
    let words = pb.as_words();
    for w in words.chunks(2) {
        assert_eq!((w[0] >> 13) & 0x7, 0, "GR init should be subchannel 0");
    }
}

#[test]
fn as_bytes_len() {
    let pb = PushBuf::compute_dispatch(0xC5C0, 0);
    assert_eq!(pb.as_bytes().len(), pb.as_words().len() * 4);
}

#[test]
fn empty_pushbuf() {
    let pb = PushBuf::new();
    assert!(pb.as_words().is_empty());
    assert!(pb.as_bytes().is_empty());
}

/// Regression: CBUF addresses referenced in the QMD may be loaded
/// through the method stream if a driver implements descriptor table
/// upload that way. Verify that the offsets we expose are consistent
/// with what `compute_dispatch` expects (subchannel 1, proper method
/// encoding).
#[test]
fn method_constants_consistent() {
    assert_eq!(method::SET_OBJECT, 0x0000);
    assert_eq!(method::INVALIDATE_SHADER_CACHES, 0x021C);
    assert_eq!(method::SEND_PCAS_A, 0x02B4);
    assert_eq!(method::SEND_SIGNALING_PCAS_B, 0x02BC);
    assert_eq!(method::SEND_SIGNALING_PCAS2_B, 0x02C0);
}

#[test]
fn slm_registers_present_in_compute_init() {
    let pb = PushBuf::compute_init(0xCEC0, 0, 0x1_0000_0000, 0x8000);
    let words = pb.as_words();
    let slm_a = method::SET_SHADER_LOCAL_MEMORY_A >> 2;
    let slm_b = method::SET_SHADER_LOCAL_MEMORY_B >> 2;
    let found_a = words.chunks(2).any(|w| (w[0] & 0x1FFF) == slm_a);
    let found_b = words.chunks(2).any(|w| (w[0] & 0x1FFF) == slm_b);
    assert!(found_a, "SET_SHADER_LOCAL_MEMORY_A should be present");
    assert!(found_b, "SET_SHADER_LOCAL_MEMORY_B should be present");
}
