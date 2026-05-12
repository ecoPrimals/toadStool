// SPDX-License-Identifier: AGPL-3.0-or-later
//! Low-level falcon PIO upload helpers — IMEM/DMEM register port protocol.
//!
//! These are standalone BAR0 register-write sequences that upload firmware
//! blobs into a falcon's instruction and data memory. They have zero
//! dependencies on GSP, ACR, or vfio_compute — only `MappedBar` and
//! falcon register offsets.

use crate::vfio::channel::registers::falcon;
use crate::vfio::device::MappedBar;

/// Upload firmware to a falcon's IMEM via the IMEMC/IMEMD/IMEMT port registers.
///
/// The upload protocol matches nouveau's `falcon_load_firmware()`:
/// 1. Write IMEMC with auto-increment and target address
/// 2. Set IMEMT tag for each 256-byte block
/// 3. Write IMEMD with 32-bit words of firmware data
pub fn falcon_upload_imem(bar0: &MappedBar, base: usize, addr: u32, data: &[u8], secure: bool) {
    let w = |off: usize, val: u32| {
        let _ = bar0.write_u32(base + off, val);
    };

    let sec_flag: u32 = if secure { 0x1000_0000 } else { 0 };
    w(falcon::IMEMC, 0x0100_0000 | sec_flag | addr);

    for (i, chunk) in data.chunks(4).enumerate() {
        let byte_offset = (i * 4) as u32;
        if byte_offset & 0xFF == 0 {
            w(falcon::IMEMT, (addr + byte_offset) >> 8);
        }
        let word = le_word(chunk);
        w(falcon::IMEMD, word);
    }

    let total_bytes = (data.len().div_ceil(4)) * 4;
    let remainder = total_bytes & 0xFF;
    if remainder != 0 {
        let padding_words = (256 - remainder) / 4;
        for _ in 0..padding_words {
            w(falcon::IMEMD, 0);
        }
    }
}

/// Upload data to a falcon's DMEM via the DMEMC/DMEMD port registers.
pub fn falcon_upload_dmem(bar0: &MappedBar, base: usize, addr: u32, data: &[u8]) {
    let w = |off: usize, val: u32| {
        let _ = bar0.write_u32(base + off, val);
    };

    w(falcon::DMEMC, 0x0100_0000 | addr);

    for chunk in data.chunks(4) {
        w(falcon::DMEMD, le_word(chunk));
    }
}

fn le_word(chunk: &[u8]) -> u32 {
    match chunk.len() {
        4 => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
        3 => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], 0]),
        2 => u32::from_le_bytes([chunk[0], chunk[1], 0, 0]),
        1 => u32::from_le_bytes([chunk[0], 0, 0, 0]),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_word_full() {
        assert_eq!(le_word(&[0x78, 0x56, 0x34, 0x12]), 0x1234_5678);
    }

    #[test]
    fn le_word_partial_3() {
        assert_eq!(le_word(&[0xAA, 0xBB, 0xCC]), 0x00CC_BBAA);
    }

    #[test]
    fn le_word_partial_2() {
        assert_eq!(le_word(&[0x01, 0x02]), 0x0000_0201);
    }

    #[test]
    fn le_word_partial_1() {
        assert_eq!(le_word(&[0xFF]), 0x0000_00FF);
    }

    #[test]
    fn le_word_empty() {
        assert_eq!(le_word(&[]), 0);
    }
}
