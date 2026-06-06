// SPDX-License-Identifier: AGPL-3.0-or-later
//! PMU FALCON I/O primitives: reset, code/data upload, args read, exec.

use crate::vfio::device::MappedBar;

use super::types::pmu_reg;

/// Reset the PMU FALCON microcontroller.
pub fn pmu_falcon_reset(bar0: &MappedBar) {
    let r = |reg| bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
    let w = |reg, val| {
        let _ = bar0.write_u32(reg, val);
    };

    w(pmu_reg::FALCON_CTRL, 0x02);
    std::thread::sleep(std::time::Duration::from_millis(5));

    let ctrl = r(pmu_reg::FALCON_CTRL);
    tracing::debug!(ctrl = format!("{ctrl:#010x}"), "PMU FALCON CTRL after halt");
}

/// Upload code to PMU FALCON IMEM.
pub fn pmu_upload_code(
    bar0: &MappedBar,
    rom: &[u8],
    pmu_addr: u32,
    rom_offset: u32,
    size: u32,
    secure: bool,
) {
    let w = |reg, val: u32| {
        let _ = bar0.write_u32(reg, val);
    };

    let sec_flag: u32 = if secure { 0x1000_0000 } else { 0 };
    w(pmu_reg::IMEM_PORT, 0x0100_0000 | sec_flag | pmu_addr);

    let data = &rom[rom_offset as usize..(rom_offset + size) as usize];
    for (i, chunk) in data.chunks(4).enumerate() {
        let byte_offset = (i * 4) as u32;
        if byte_offset & 0xFF == 0 {
            w(pmu_reg::IMEM_TAG, (pmu_addr + byte_offset) >> 8);
        }

        let word = match chunk.len() {
            4 => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
            3 => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], 0]),
            2 => u32::from_le_bytes([chunk[0], chunk[1], 0, 0]),
            1 => u32::from_le_bytes([chunk[0], 0, 0, 0]),
            _ => 0,
        };
        w(pmu_reg::IMEM_DATA, word);
    }

    let total_words = (size as usize).div_ceil(4);
    let remainder = (total_words * 4) & 0xFF;
    if remainder != 0 {
        let padding_words = (256 - remainder) / 4;
        for _ in 0..padding_words {
            w(pmu_reg::IMEM_DATA, 0);
        }
    }
}

/// Upload data to PMU FALCON DMEM.
pub fn pmu_upload_data(bar0: &MappedBar, rom: &[u8], pmu_addr: u32, rom_offset: u32, size: u32) {
    let w = |reg, val: u32| {
        let _ = bar0.write_u32(reg, val);
    };

    w(pmu_reg::DMEM_PORT, 0x0100_0000 | pmu_addr);

    let data = &rom[rom_offset as usize..(rom_offset + size) as usize];
    for chunk in data.chunks(4) {
        let word = match chunk.len() {
            4 => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
            3 => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], 0]),
            2 => u32::from_le_bytes([chunk[0], chunk[1], 0, 0]),
            1 => u32::from_le_bytes([chunk[0], 0, 0, 0]),
            _ => 0,
        };
        w(pmu_reg::DMEM_DATA, word);
    }
}

/// Read a DMEM argument pointer (indirect read: DMEM[DMEM[argp] + argi]).
///
/// Uses BIT(25) for DMEMC read mode, consistent with the GM200+ PIO protocol
/// used by all other falcon PIO paths in this codebase.
pub fn pmu_read_args(bar0: &MappedBar, argp: u32, argi: u32) -> u32 {
    let r = |reg| bar0.read_u32(reg).unwrap_or(0);
    let w = |reg, val: u32| {
        let _ = bar0.write_u32(reg, val);
    };

    w(pmu_reg::DMEM_PORT, 0x0200_0000 | argp);
    let indirect = r(pmu_reg::DMEM_DATA);
    w(pmu_reg::DMEM_PORT, 0x0200_0000 | (indirect + argi));
    r(pmu_reg::DMEM_DATA)
}

/// Start PMU FALCON execution at the given address.
pub fn pmu_exec(bar0: &MappedBar, init_addr: u32) {
    let w = |reg, val: u32| {
        let _ = bar0.write_u32(reg, val);
    };
    w(pmu_reg::FALCON_PC, init_addr);
    w(pmu_reg::FALCON_TRIG, 0);
    w(pmu_reg::FALCON_CTRL, 0x02);
}
