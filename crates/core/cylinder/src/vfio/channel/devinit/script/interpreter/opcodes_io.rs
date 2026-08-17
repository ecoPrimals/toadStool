// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::DevinitError;

use super::super::VbiosInterpreter;

/// I/O and GPIO opcodes — no-op for VFIO (stride past operand bytes).
pub(super) fn dispatch_io(vm: &mut VbiosInterpreter<'_>, op: u8) -> Result<(), DevinitError> {
    match op {
        0x69 => vm.offset += 5,
        0x32 => {
            let count = vm.rd08(vm.offset + 7) as usize;
            vm.offset += 8 + count * 4;
        }
        0x37 => vm.offset += 11,
        0x3B | 0x3C => vm.offset += 5,
        0x49 => {
            let count = vm.rd08(vm.offset + 7) as usize;
            vm.offset += 8 + count * 2;
        }
        0x4C => vm.offset += 7,
        0x4D => {
            // INIT_ZM_I2C_BYTE: index(u8) + addr(u8) + count(u8), then count
            // pairs of (reg, val). Variable length, not the fixed 6 this used
            // to advance — on a measured GK210 image a count of 2 needs 8
            // bytes, and stopping at 6 landed mid-payload and desynced the
            // remainder of the script.
            // nouveau: init_zm_i2c_byte, offset += 4 then += 2 per entry.
            let count = vm.rd08(vm.offset + 3) as usize;
            vm.offset += 4 + count * 2;
        }
        0x4E => {
            let count = vm.rd08(vm.offset + 4) as usize;
            vm.offset += 5 + count;
        }
        0x4F => {
            // INIT_TMDS: tmds(u8) + addr(u8) + mask(u8) + data(u8) = stride 5
            vm.offset += 5;
        }
        0x50 => {
            // INIT_IO_RESTRICT_PROG: port(u16) + index(u8) + mask(u8) + shift(u8)
            // + count(u8) + reg(u32) + count × u32
            let count = vm.rd08(vm.offset + 6) as usize;
            vm.offset += 11 + count * 4;
        }
        0x51 => vm.offset += 7,
        0x52 => vm.offset += 4,
        0x53 => vm.offset += 3,
        0x54 => {
            let count = vm.rd08(vm.offset + 1) as usize;
            vm.offset += 2 + count * 2;
        }
        0x5E => vm.offset += 6,
        0x62 => vm.offset += 5,
        0x78 => vm.offset += 6,
        // INIT_I2C_LONG_IF. nouveau's init_i2c_long_if advances 7, and 7 was
        // tried here: on the measured GK210 image it takes unknown opcodes from
        // 1 to 5, so this image's encoding is 11 bytes. Left at 11 on the
        // evidence. If a Volta image ever decodes worse at 11, this needs to
        // become capability-selected rather than flipped.
        0x96 => vm.offset += 11,
        0x98 => vm.offset += 8,
        0x99 => {
            let count = vm.rd08(vm.offset + 5) as usize;
            vm.offset += 6 + count;
        }
        0x9A => vm.offset += 9,
        0xA9 => {
            let count = vm.rd08(vm.offset + 1) as usize;
            vm.offset += 2 + count * 2;
        }
        _ => unreachable!("dispatch_io called with non-I/O opcode {op:#04x}"),
    }
    vm.stats.ops_skipped += 1;
    Ok(())
}
