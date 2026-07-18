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
        0x4D => vm.offset += 6,
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
