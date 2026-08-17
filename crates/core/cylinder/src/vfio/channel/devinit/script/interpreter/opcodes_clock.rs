// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::DevinitError;

use super::super::{VbiosInterpreter, ram_restrict_group_count};

/// PLL programming and RAM-restrict memory opcodes.
pub(super) fn dispatch_clock(vm: &mut VbiosInterpreter<'_>, op: u8) -> Result<(), DevinitError> {
    match op {
        // ── PLL programming ─────────────────────────────────
        //
        // PLL opcodes write pre-computed coefficient words from the VBIOS ROM
        // directly to PLL control registers. The VBIOS stores the actual
        // hardware register value (N/M/P dividers encoded), not abstract
        // frequencies — so a direct BAR0 write is correct.
        0x79 => {
            // INIT_PLL: reg(u32) + freq(u16|u32)
            // Kepler: 7 bytes (reg:4 + freq:2 + unk:1)
            // Maxwell+: 9 bytes (reg:4 + freq:4 + unk:1)
            let reg = vm.rd32(vm.offset + 1);
            if vm.bios_gen == super::super::BiosGeneration::Kepler {
                let val = vm.rd16(vm.offset + 5) as u32;
                if reg < 0x0100_0000 && val != 0 {
                    vm.bar0_wr32(reg, val);
                }
                vm.offset += 7;
            } else {
                let val = vm.rd32(vm.offset + 5);
                if reg < 0x0100_0000 && val != 0 {
                    vm.bar0_wr32(reg, val);
                }
                vm.offset += 9;
            }
        }
        0x4B => {
            // INIT_PLL_INDIRECT: reg(u32) + freq_data(u16) + pll_off(u16)
            // Writes freq_data to the PLL register. pll_off is a ROM lookup
            // hint the host interpreter does not need.
            let reg = vm.rd32(vm.offset + 1);
            let val = vm.rd16(vm.offset + 5) as u32;
            if reg < 0x0100_0000 && val != 0 {
                vm.bar0_wr32(reg, val);
            }
            vm.offset += 9;
        }
        0x34 => {
            // INIT_RAM_RESTRICT_PLL: reg(u32) + mask(u32) + unk(u8)
            // + count(u8) + count × freq(u32)
            // Selects one of count values based on RAM strap index.
            let reg = vm.rd32(vm.offset + 1);
            let count = vm.rd08(vm.offset + 9) as usize;
            let strap = vm.bar0_rd32(0x0010_0000) as usize;
            let n = ram_restrict_group_count(vm.rom);
            let idx = strap % n;
            if count > 0 && idx < count {
                let val = vm.rd32(vm.offset + 10 + idx * 4);
                if reg < 0x0100_0000 && val != 0 {
                    vm.bar0_wr32(reg, val);
                }
            }
            vm.offset += 10 + count * 4;
        }
        0x4A => {
            // Same layout as 0x34 — RAM-restrict PLL variant.
            let reg = vm.rd32(vm.offset + 1);
            let count = vm.rd08(vm.offset + 9) as usize;
            let strap = vm.bar0_rd32(0x0010_0000) as usize;
            let n = ram_restrict_group_count(vm.rom);
            let idx = strap % n;
            if count > 0 && idx < count {
                let val = vm.rd32(vm.offset + 10 + idx * 4);
                if reg < 0x0100_0000 && val != 0 {
                    vm.bar0_wr32(reg, val);
                }
            }
            vm.offset += 10 + count * 4;
        }
        0x59 => {
            // INIT_PLL2: reg(u32) + freq(u32) + unk(u32) — extended PLL
            let reg = vm.rd32(vm.offset + 1);
            let val = vm.rd32(vm.offset + 5);
            if reg < 0x0100_0000 && val != 0 {
                vm.bar0_wr32(reg, val);
            }
            vm.offset += 13;
        }
        0x87 => {
            // INIT_RAM_RESTRICT_ZM_REG: reg(u32) + N × val(u32)
            // Writes reg with val[idx] where idx = RAM strap % N.
            let reg = vm.rd32(vm.offset + 1);
            let n = ram_restrict_group_count(vm.rom);
            let strap = vm.bar0_rd32(0x0010_0000) as usize;
            let idx = strap % n;
            if idx < n {
                let val = vm.rd32(vm.offset + 5 + idx * 4);
                if reg < 0x0100_0000 {
                    vm.bar0_wr32(reg, val);
                }
            }
            vm.offset += 5 + n * 4;
        }

        // ── RAM-restrict groups ─────────────────────────────
        0x88 => {
            // INIT_RAM_RESTRICT_ZM_REG_GROUP: base(u32) + count(u8)
            // + count × N × val(u32)
            // For each of `count` sequential registers starting at `base`,
            // selects the value at RAM strap index from N alternatives.
            let base = vm.rd32(vm.offset + 1);
            let count = vm.rd08(vm.offset + 5) as usize;
            let n = ram_restrict_group_count(vm.rom);
            let strap = vm.bar0_rd32(0x0010_0000) as usize;
            let idx = strap % n;
            let data_start = vm.offset + 6;
            for i in 0..count {
                let reg = base + (i as u32) * 4;
                let val = vm.rd32(data_start + (i * n + idx) * 4);
                if reg < 0x0100_0000 {
                    vm.bar0_wr32(reg, val);
                }
            }
            vm.offset += 6 + count * n * 4;
        }
        0x8F => {
            // INIT_RAM_RESTRICT_ZM_REG_GROUP
            //   addr(u32) @+1, incr(u8) @+5, num(u8) @+6, then num × N × u32.
            //
            // Header is **7** bytes, and +5 is the address stride, not a
            // count. Reading +5 as the count over a 6-byte header consumed
            // `incr × N × 4` payload bytes instead of `num × N × 4` — on the
            // K80's first group that is 134 bytes where the opcode occupies
            // 39, so the walk resumed inside the payload and the remainder of
            // the script decoded as noise.
            let mut addr = vm.rd32(vm.offset + 1);
            let incr = u32::from(vm.rd08(vm.offset + 5));
            let num = vm.rd08(vm.offset + 6) as usize;
            let n = ram_restrict_group_count(vm.rom);
            let strap = vm.bar0_rd32(0x0010_0000) as usize;
            let idx = strap % n;

            let data_start = vm.offset + 7;
            for i in 0..num {
                let val = vm.rd32(data_start + (i * n + idx) * 4);
                if addr < 0x0100_0000 {
                    vm.bar0_wr32(addr, val);
                }
                addr = addr.wrapping_add(incr);
            }
            vm.offset += 7 + num * n * 4;
        }
        _ => unreachable!("dispatch_clock called with non-clock opcode {op:#04x}"),
    }
    Ok(())
}
