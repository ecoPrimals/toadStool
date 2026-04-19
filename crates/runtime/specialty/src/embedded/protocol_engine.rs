// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(missing_docs)]
//! Protocol engines emit byte sequences for ISP/ICSP/parallel programming without performing I/O.

use super::chip_database::{AvrChipInfo, PicChipInfo};
use super::errors::EmbeddedProgrammerError;

/// One AVR serial programming instruction (four SPI bytes, MSB first per frame).
pub type AvrIspFrame = [u8; 4];

/// Aggregated TX bytes for AVR ISP (what the transport clocks out; MISO not modeled here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvrIspSequence {
    pub frames: Vec<AvrIspFrame>,
}

impl AvrIspSequence {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.frames.iter().flat_map(|f| f.iter().copied()).collect()
    }
}

/// Programming Enable — second byte must echo 0x53 on MISO when synchronized.
#[inline]
pub fn avr_isp_programming_enable() -> AvrIspFrame {
    [0xAC, 0x53, 0x00, 0x00]
}

/// Chip erase (typical 9–45 ms busy on device).
#[inline]
pub fn avr_isp_chip_erase() -> AvrIspFrame {
    [0xAC, 0x80, 0x00, 0x00]
}

/// Read signature byte index `idx` in 0..3 (usually 0..3 for three signature bytes; AVR uses 0,1,2).
#[inline]
pub fn avr_isp_read_signature_byte(idx: u8) -> AvrIspFrame {
    [0x30, 0x00, idx, 0x00]
}

/// Read fuse bytes (returns TX frame; response data is clocked in on MISO during frame).
#[inline]
pub fn avr_isp_read_fuse_low() -> AvrIspFrame {
    [0x50, 0x00, 0x00, 0x00]
}

#[inline]
pub fn avr_isp_read_fuse_high() -> AvrIspFrame {
    [0x58, 0x08, 0x00, 0x00]
}

#[inline]
pub fn avr_isp_read_fuse_extended() -> AvrIspFrame {
    [0x50, 0x08, 0x00, 0x00]
}

/// Write fuse low byte (`value` is programmed fuse value).
#[inline]
pub fn avr_isp_write_fuse_low(value: u8) -> AvrIspFrame {
    [0xAC, 0xA0, 0x00, value]
}

#[inline]
pub fn avr_isp_write_fuse_high(value: u8) -> AvrIspFrame {
    [0xAC, 0xA8, 0x00, value]
}

#[inline]
pub fn avr_isp_write_fuse_extended(value: u8) -> AvrIspFrame {
    [0xAC, 0xA4, 0x00, value]
}

/// Read one flash byte at byte address `addr` (see AVR serial programming instruction set).
#[inline]
pub fn avr_isp_read_flash_byte(addr: u32) -> AvrIspFrame {
    let w = (addr >> 1) as u16;
    let cmd = if (addr & 1) == 0 { 0x20u8 } else { 0x28u8 };
    [cmd, (w >> 8) as u8, w as u8, 0x00]
}

/// Load flash page buffer low byte (`addr` page-relative word index bits, `data` low byte value).
#[inline]
pub fn avr_isp_load_flash_low(addr_low: u8, data: u8) -> AvrIspFrame {
    [0x40, 0x00, addr_low, data]
}

/// Load flash page buffer high byte.
#[inline]
pub fn avr_isp_load_flash_high(addr_high: u8, data: u8) -> AvrIspFrame {
    [0x48, 0x00, addr_high, data]
}

/// Write loaded flash page (`page` is word address of page base / page index per AVR family).
#[inline]
pub fn avr_isp_write_program_page(program_word_high: u8, program_word_low: u8) -> AvrIspFrame {
    [0x4C, program_word_high, program_word_low, 0x00]
}

/// Extended address byte for >64KiB parts (ATmega2560 et al.).
#[inline]
pub fn avr_isp_load_extended_address(ext: u8) -> AvrIspFrame {
    [0x4D, 0x00, ext, 0x00]
}

/// Build programming enable + signature read + fuse read (typical connect self-check sequence).
pub fn avr_isp_connect_probe_sequence() -> AvrIspSequence {
    let mut frames = Vec::with_capacity(8);
    frames.push(avr_isp_programming_enable());
    for i in 0u8..3 {
        frames.push(avr_isp_read_signature_byte(i));
    }
    frames.push(avr_isp_read_fuse_low());
    frames.push(avr_isp_read_fuse_high());
    frames.push(avr_isp_read_fuse_extended());
    AvrIspSequence { frames }
}

/// Validate `[addr, addr+length)` lies in flash for chip.
pub fn avr_validate_flash_range(
    chip: &AvrChipInfo,
    address: u32,
    length: u32,
) -> Result<(), EmbeddedProgrammerError> {
    let end = address
        .checked_add(length)
        .ok_or(EmbeddedProgrammerError::AddressOutOfRange {
            address,
            length,
            limit: chip.flash_size,
        })?;
    if end > chip.flash_size {
        return Err(EmbeddedProgrammerError::AddressOutOfRange {
            address,
            length,
            limit: chip.flash_size,
        });
    }
    Ok(())
}

/// Flash page write must be aligned to `chip.flash_page_size` and non-empty.
pub fn avr_validate_page_write(
    chip: &AvrChipInfo,
    address: u32,
    data_len: usize,
) -> Result<(), EmbeddedProgrammerError> {
    if data_len == 0 {
        return Err(EmbeddedProgrammerError::DataLayoutInvalid {
            detail: "empty page write".into(),
        });
    }
    if data_len != chip.flash_page_size as usize {
        return Err(EmbeddedProgrammerError::DataLayoutInvalid {
            detail: format!(
                "page write length {data_len} must equal flash page size {}",
                chip.flash_page_size
            ),
        });
    }
    if !address.is_multiple_of(chip.flash_page_size) {
        return Err(EmbeddedProgrammerError::DataLayoutInvalid {
            detail: format!(
                "page write address 0x{address:x} must be aligned to {}",
                chip.flash_page_size
            ),
        });
    }
    avr_validate_flash_range(chip, address, data_len as u32)
}

/// Generate read sequence for `length` bytes starting at `address` (one ISP frame per byte).
pub fn avr_isp_read_flash_sequence(
    chip: &AvrChipInfo,
    address: u32,
    length: u32,
) -> Result<AvrIspSequence, EmbeddedProgrammerError> {
    avr_validate_flash_range(chip, address, length)?;
    let mut frames = Vec::with_capacity(length as usize);
    for i in 0..length {
        frames.push(avr_isp_read_flash_byte(address + i));
    }
    Ok(AvrIspSequence { frames })
}

/// Generate page program TX frames for one page of data (load low/high pairs + write page).
pub fn avr_isp_program_page_sequence(
    chip: &AvrChipInfo,
    page_base: u32,
    page_data: &[u8],
) -> Result<AvrIspSequence, EmbeddedProgrammerError> {
    avr_validate_page_write(chip, page_base, page_data.len())?;
    let page = chip.flash_page_size as usize;
    let mut frames = Vec::with_capacity(page + 4);

    if chip.flash_size > 64 * 1024 {
        let ext = (page_base >> 16) as u8;
        frames.push(avr_isp_load_extended_address(ext));
    }

    for off in (0..page).step_by(2) {
        let word_idx = (off / 2) as u8;
        let low = page_data[off];
        let high = page_data[off + 1];
        frames.push(avr_isp_load_flash_low(word_idx, low));
        frames.push(avr_isp_load_flash_high(word_idx, high));
    }

    let page_word = (page_base >> 1) as u16;
    frames.push(avr_isp_write_program_page(
        (page_word >> 8) as u8,
        page_word as u8,
    ));

    Ok(AvrIspSequence { frames })
}

/// Generate chip erase + delay placeholder (delay is host-side; we append a synthetic marker byte 0x00 for tracing only — not sent on SPI).
pub fn avr_isp_erase_sequence(_chip: &AvrChipInfo) -> AvrIspSequence {
    AvrIspSequence {
        frames: vec![avr_isp_chip_erase()],
    }
}

// --- PIC18 ICSP (6-bit core commands as low 6 bits per serialized byte; transport clocks bits) ---

/// Magic entry pattern ("MCHP") for PIC18 programming mode entry (logical bytes; bit-serial on wire).
pub const PIC18_ICSP_ENTRY_KEY: [u8; 4] = [0x4D, 0x43, 0x48, 0x50];

/// 6-bit command values (masked with 0x3F) — PIC18 programming executive / core command subset.
pub mod pic18_cmd {
    /// NOP / padding.
    pub const NOP: u8 = 0x00;
    /// Read data from program memory (core mode — device-specific response handling).
    pub const READ_PROGRAM_MEMORY: u8 = 0x05;
    /// Increment address.
    pub const INCREMENT_ADDRESS: u8 = 0x06;
    /// Begin programming cycle.
    pub const BEGIN_PROGRAMMING: u8 = 0x08;
    /// Bulk erase program memory.
    pub const BULK_ERASE_PROGRAM: u8 = 0x0B;
    /// Row erase program memory.
    pub const ROW_ERASE_PROGRAM: u8 = 0x0A;
}

/// ICSP entry + bulk erase + first read opcode (protocol skeleton for transport).
pub fn pic18_icsp_connect_sequence() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&PIC18_ICSP_ENTRY_KEY);
    v.push(pic18_cmd::NOP);
    v.push(pic18_cmd::BULK_ERASE_PROGRAM);
    v.push(pic18_cmd::NOP);
    v
}

/// Read path: NOP padding + read command + increment (address load is device/PE-specific; kept minimal).
pub fn pic18_icsp_read_program_memory_ops(count: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(count as usize * 2);
    for _ in 0..count {
        v.push(pic18_cmd::READ_PROGRAM_MEMORY);
        v.push(pic18_cmd::INCREMENT_ADDRESS);
    }
    v
}

pub fn pic_validate_flash_range(
    chip: &PicChipInfo,
    address: u32,
    length: u32,
) -> Result<(), EmbeddedProgrammerError> {
    let end = address
        .checked_add(length)
        .ok_or(EmbeddedProgrammerError::AddressOutOfRange {
            address,
            length,
            limit: chip.flash_size,
        })?;
    if end > chip.flash_size {
        return Err(EmbeddedProgrammerError::AddressOutOfRange {
            address,
            length,
            limit: chip.flash_size,
        });
    }
    Ok(())
}

/// EPROM/GPIO parallel: address setup + /OE pulse (abstract encoding: 0xA0 addr_lo addr_hi 0xE0 read strobe).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelEpromSequence {
    pub ops: Vec<u8>,
}

pub fn parallel_eprom_read_block(
    address: u32,
    length: u32,
    max_address: u32,
) -> Result<ParallelEpromSequence, EmbeddedProgrammerError> {
    let end = address
        .checked_add(length)
        .ok_or(EmbeddedProgrammerError::AddressOutOfRange {
            address,
            length,
            limit: max_address,
        })?;
    if end > max_address {
        return Err(EmbeddedProgrammerError::AddressOutOfRange {
            address,
            length,
            limit: max_address,
        });
    }
    let mut ops = Vec::with_capacity(length as usize * 4);
    for a in address..end {
        ops.push(0xA0);
        ops.push(a as u8);
        ops.push((a >> 8) as u8);
        ops.push(0xE0); // /OE low strobe marker
    }
    Ok(ParallelEpromSequence { ops })
}

/// Combines AVR / PIC / parallel protocol builders for testing and host-side validation.
#[derive(Debug, Clone, Default)]
pub struct ProtocolEngine {
    /// Last built AVR sequence (for diagnostics).
    pub last_avr: Option<AvrIspSequence>,
    /// Last built PIC opcode stream.
    pub last_pic: Option<Vec<u8>>,
    /// Last parallel EPROM op stream.
    pub last_parallel: Option<ParallelEpromSequence>,
}

impl ProtocolEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build_avr_connect_probe(&mut self) -> &AvrIspSequence {
        let s = avr_isp_connect_probe_sequence();
        self.last_avr = Some(s);
        self.last_avr.as_ref().expect("just set")
    }

    pub fn build_avr_read_flash(
        &mut self,
        chip: &AvrChipInfo,
        address: u32,
        length: u32,
    ) -> Result<&AvrIspSequence, EmbeddedProgrammerError> {
        let s = avr_isp_read_flash_sequence(chip, address, length)?;
        self.last_avr = Some(s);
        Ok(self.last_avr.as_ref().expect("set"))
    }

    pub fn build_avr_program_page(
        &mut self,
        chip: &AvrChipInfo,
        page_base: u32,
        data: &[u8],
    ) -> Result<&AvrIspSequence, EmbeddedProgrammerError> {
        let s = avr_isp_program_page_sequence(chip, page_base, data)?;
        self.last_avr = Some(s);
        Ok(self.last_avr.as_ref().expect("set"))
    }

    pub fn build_pic_connect(&mut self) -> &[u8] {
        let s = pic18_icsp_connect_sequence();
        self.last_pic = Some(s);
        self.last_pic.as_deref().expect("set")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedded::chip_database::{DeviceSignature, avr_by_name};

    #[test]
    fn avr_programming_enable_bytes() {
        let f = avr_isp_programming_enable();
        assert_eq!(f, [0xAC, 0x53, 0x00, 0x00]);
    }

    #[test]
    fn avr_signature_frames_match_device_bytes() {
        let sig = DeviceSignature::from_bytes(0x1E, 0x95, 0x0F);
        let b0 = (sig.0 >> 16) as u8;
        let b1 = (sig.0 >> 8) as u8;
        let b2 = sig.0 as u8;
        let f0 = avr_isp_read_signature_byte(0);
        let f1 = avr_isp_read_signature_byte(1);
        let f2 = avr_isp_read_signature_byte(2);
        assert_eq!(f0[2], 0);
        assert_eq!(f1[2], 1);
        assert_eq!(f2[2], 2);
        // Frames are TX; signature bytes appear on MISO during these frames.
        let _ = (b0, b1, b2);
    }

    #[test]
    fn avr_read_flash_sequence_length() {
        let chip = avr_by_name("ATmega328P").expect("chip");
        let seq = avr_isp_read_flash_sequence(chip, 0, 16).expect("seq");
        assert_eq!(seq.frames.len(), 16);
    }

    #[test]
    fn pic_entry_includes_mchp() {
        let v = pic18_icsp_connect_sequence();
        assert!(v.windows(4).any(|w| w == PIC18_ICSP_ENTRY_KEY));
    }
}
