// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::DevinitError;

use super::super::VbiosInterpreter;

/// BAR0 register read/write opcodes.
pub(super) fn dispatch_register(vm: &mut VbiosInterpreter<'_>, op: u8) -> Result<(), DevinitError> {
    match op {
        0x7A => {
            // INIT_ZM_REG: addr(u32) + data(u32)
            let reg = vm.rd32(vm.offset + 1);
            let val = vm.rd32(vm.offset + 5);
            if reg < 0x0100_0000 {
                vm.bar0_wr32(reg, val);
            }
            vm.offset += 9;
        }
        0x6E => {
            // INIT_NV_REG: addr(u32) + mask(u32) + value(u32) — read-modify-write
            let reg = vm.rd32(vm.offset + 1);
            let mask = vm.rd32(vm.offset + 5);
            let val = vm.rd32(vm.offset + 9);
            if reg < 0x0100_0000 {
                vm.bar0_mask(reg, mask, val);
            }
            vm.offset += 13;
        }
        0x58 => {
            // INIT_ZM_REG_SEQUENCE: base(u32) + count(u8) + count×data(u32)
            let base = vm.rd32(vm.offset + 1);
            let count = vm.rd08(vm.offset + 5) as usize;
            vm.offset += 6;
            for i in 0..count {
                if vm.offset + 4 > vm.rom.len() {
                    break;
                }
                let val = vm.rd32(vm.offset);
                let reg = base + (i as u32) * 4;
                if reg < 0x0100_0000 {
                    vm.bar0_wr32(reg, val);
                }
                vm.offset += 4;
            }
        }
        0x77 => {
            // INIT_ZM_REG16: addr(u32) + data(u16) — write low 16 bits
            let reg = vm.rd32(vm.offset + 1);
            let val = vm.rd16(vm.offset + 5) as u32;
            if reg < 0x0100_0000 {
                vm.bar0_wr32(reg, val);
            }
            vm.offset += 7;
        }
        0x47 => {
            // INIT_ANDN_REG: addr(u32) + mask(u32) — clear bits
            let reg = vm.rd32(vm.offset + 1);
            let mask = vm.rd32(vm.offset + 5);
            if reg < 0x0100_0000 {
                vm.bar0_mask(reg, !mask, 0);
            }
            vm.offset += 9;
        }
        0x48 => {
            // INIT_OR_REG: addr(u32) + value(u32) — set bits
            let reg = vm.rd32(vm.offset + 1);
            let val = vm.rd32(vm.offset + 5);
            if reg < 0x0100_0000 {
                vm.bar0_mask(reg, 0xFFFF_FFFF, val);
            }
            vm.offset += 9;
        }
        0x90 => {
            // INIT_COPY_ZM_REG: src_reg(u32) + dst_reg(u32)
            let src = vm.rd32(vm.offset + 1);
            let dst = vm.rd32(vm.offset + 5);
            if src < 0x0100_0000 && dst < 0x0100_0000 {
                let val = vm.bar0_rd32(src);
                vm.bar0_wr32(dst, val);
            }
            vm.offset += 9;
        }
        0x91 => {
            // INIT_ZM_REG_GROUP: addr(u32) + count(u8) + count×data(u32)
            let base = vm.rd32(vm.offset + 1);
            let count = vm.rd08(vm.offset + 5) as usize;
            vm.offset += 6;
            for i in 0..count {
                if vm.offset + 4 > vm.rom.len() {
                    break;
                }
                let val = vm.rd32(vm.offset);
                let reg = base + (i as u32) * 4;
                if reg < 0x0100_0000 {
                    vm.bar0_wr32(reg, val);
                }
                vm.offset += 4;
            }
        }
        0x97 => {
            // INIT_ZM_MASK_ADD: addr(u32) + mask(u32) + add(u8)
            let reg = vm.rd32(vm.offset + 1);
            let mask = vm.rd32(vm.offset + 5);
            let add = vm.rd08(vm.offset + 9) as u32;
            if reg < 0x0100_0000 {
                let cur = vm.bar0_rd32(reg);
                vm.bar0_wr32(reg, (cur & mask) + add);
            }
            vm.offset += 11;
        }
        0x5A => {
            // INIT_ZM_REG_INDIRECT: reg(u32) + bios_addr(u32)
            // Nouveau: reads u32 from BIOS ROM at bios_addr, writes to BAR0 reg.
            let reg = vm.rd32(vm.offset + 1);
            let bios_addr = vm.rd32(vm.offset + 5) as usize;
            if reg < 0x0100_0000 {
                let val = vm.rd32(bios_addr);
                vm.bar0_wr32(reg, val);
            }
            vm.offset += 9;
        }
        0x5F => {
            // INIT_COPY_NV_REG: src(u32) + smask(u32) + sshift(u8)
            //                   + dst(u32) + dmask(u32) + dshift(u8)
            // Reads src, masks+shifts, then RMW into dst.
            let src = vm.rd32(vm.offset + 1);
            let smask = vm.rd32(vm.offset + 5);
            let sshift = vm.rd08(vm.offset + 9);
            let dst = vm.rd32(vm.offset + 10);
            let dmask = vm.rd32(vm.offset + 14);
            let dshift = vm.rd08(vm.offset + 18);
            if src < 0x0100_0000 && dst < 0x0100_0000 {
                let sval = (vm.bar0_rd32(src) & smask) >> sshift;
                let dval = vm.bar0_rd32(dst) & dmask;
                vm.bar0_wr32(dst, dval | (sval << dshift));
            }
            vm.offset += 22;
        }
        _ => unreachable!("dispatch_register called with non-register opcode {op:#04x}"),
    }
    Ok(())
}
