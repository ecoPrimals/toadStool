// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::DevinitError;

use super::{VbiosInterpreter, ram_restrict_group_count};

/// Execute a single VBIOS init opcode.
pub(super) fn dispatch_opcode(
    vm: &mut VbiosInterpreter<'_>,
    op: u8,
    cond_table: usize,
) -> Result<(), DevinitError> {
    match op {
        // ── Termination ─────────────────────────────────────
        0x71 => {
            // INIT_DONE — end of script
            vm.offset = 0;
        }

        // ── Control flow ────────────────────────────────────
        0x72 => {
            // INIT_RESUME — re-enable execution
            vm.execute = true;
            vm.offset += 1;
        }
        0x38 => {
            // INIT_NOT — invert execution flag
            vm.execute = !vm.execute;
            vm.offset += 1;
        }
        0x33 => {
            // INIT_REPEAT: count(u8) — repeat next block count times
            vm.repeat_count = vm.rd08(vm.offset + 1);
            vm.repeat_offset = vm.offset + 2;
            vm.offset += 2;
        }
        0x36 => {
            // INIT_END_REPEAT
            if vm.repeat_count > 1 {
                vm.repeat_count -= 1;
                vm.offset = vm.repeat_offset;
            } else {
                vm.repeat_count = 0;
                vm.offset += 1;
            }
        }
        0x5C => {
            // INIT_JUMP: offset(u16) — jump to offset in ROM
            let target = vm.rd16(vm.offset + 1) as usize;
            if target == 0 || target >= vm.rom.len() {
                vm.offset = 0;
            } else {
                vm.offset = target;
            }
        }
        0x5B => {
            // INIT_SUB_DIRECT: addr(u16) — call sub-script
            let sub_addr = vm.rd16(vm.offset + 1) as usize;
            vm.offset += 3;
            if sub_addr != 0 && sub_addr < vm.rom.len() {
                let saved = vm.offset;
                vm.offset = sub_addr;
                vm.run()?;
                vm.offset = saved;
            }
        }
        0x6B => {
            // INIT_SUB: index(u8) — call indexed sub-script via init_script table
            let index = vm.rd08(vm.offset + 1) as usize;
            vm.offset += 2;

            let init_tables_base = vm.find_init_tables_base();
            if init_tables_base != 0 && init_tables_base + 2 <= vm.rom.len() {
                let script_table_ptr = vm.rd16(init_tables_base) as usize;
                if script_table_ptr != 0 {
                    let entry_off = script_table_ptr + index * 2;
                    if entry_off + 2 <= vm.rom.len() {
                        let sub_addr = vm.rd16(entry_off) as usize;
                        if sub_addr != 0 && sub_addr < vm.rom.len() {
                            let saved = vm.offset;
                            vm.offset = sub_addr;
                            vm.run()?;
                            vm.offset = saved;
                        }
                    }
                }
            }
        }

        // ── Conditions ──────────────────────────────────────
        0x75 => {
            // INIT_CONDITION: cond(u8)
            let cond = vm.rd08(vm.offset + 1);
            vm.stats.conditions_evaluated += 1;
            if cond_table != 0 && !vm.condition_met(cond_table, cond) {
                vm.execute = false;
            }
            vm.offset += 2;
        }
        0x73 => {
            // INIT_STRAP_CONDITION: mask(u32), value(u32)
            // Nouveau: reads NV_PEXTDEV_BOOT_0 (0x101000), checks (strap & mask) == value
            let mask = vm.rd32(vm.offset + 1);
            let value = vm.rd32(vm.offset + 5);
            vm.stats.conditions_evaluated += 1;
            let strap = vm.bar0_rd32(0x0010_1000);
            if (strap & mask) != value {
                vm.execute = false;
            }
            vm.offset += 9;
        }
        0x6D => {
            // INIT_RAM_CONDITION: mask(u8), value(u8)
            // Nouveau reads NV_PFB_CFG0 (0x100000) for RAM strap
            let mask = vm.rd08(vm.offset + 1);
            let value = vm.rd08(vm.offset + 2);
            vm.stats.conditions_evaluated += 1;
            let strap = vm.bar0_rd32(0x0010_0000);
            if (strap as u8 & mask) != value {
                vm.execute = false;
            }
            vm.offset += 3;
        }
        0x56 => {
            // INIT_CONDITION_TIME: cond(u8), retries(u8)
            // Stride 3 for ALL generations (nouveau: init->offset += 3).
            // Retries × ~50ms delay (capped at 100 iterations).
            let cond = vm.rd08(vm.offset + 1);
            let retries = vm.rd08(vm.offset + 2) as u32;
            let wait = retries.min(100);
            vm.stats.conditions_evaluated += 1;
            if cond_table != 0 {
                let mut met = false;
                for _ in 0..wait.max(1) {
                    if vm.condition_met(cond_table, cond) {
                        met = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    vm.stats.delays_total_us += 50_000;
                }
                if !met {
                    vm.execute = false;
                }
            }
            vm.offset += 3;
        }
        0x76 => {
            // INIT_IO_CONDITION: cond(u8)
            vm.stats.conditions_evaluated += 1;
            vm.offset += 2;
        }
        0x39 => {
            // INIT_IO_FLAG_CONDITION: cond(u8)
            vm.stats.conditions_evaluated += 1;
            vm.offset += 2;
        }
        0x3A => {
            // INIT_GENERIC_CONDITION: cond(u8), size(u8)
            // For known conditions (DP/eDP checks): stride 3.
            // For unknown conditions: stride 3 + size (skip data block).
            // In sovereign mode we have no display connector info, so all
            // conditions are "unknown" → skip the data block.
            let _cond = vm.rd08(vm.offset + 1);
            let size = vm.rd08(vm.offset + 2) as usize;
            vm.stats.conditions_evaluated += 1;
            vm.execute = false;
            vm.offset += 3 + size;
        }

        // ── Delays ──────────────────────────────────────────
        0x74 => {
            // INIT_TIME: usec(u16)
            let usec = vm.rd16(vm.offset + 1) as u64;
            if vm.execute && usec > 0 {
                if usec <= 20_000 {
                    std::thread::sleep(std::time::Duration::from_micros(usec));
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(usec / 1000));
                }
                vm.stats.delays_total_us += usec;
            }
            vm.offset += 3;
        }
        0x57 => {
            // INIT_LTIME: usec(u16) — same as TIME
            let usec = vm.rd16(vm.offset + 1) as u64;
            if vm.execute && usec > 0 {
                std::thread::sleep(std::time::Duration::from_micros(usec));
                vm.stats.delays_total_us += usec;
            }
            vm.offset += 3;
        }

        // ── Register writes ─────────────────────────────────
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
            if vm.bios_gen == super::BiosGeneration::Kepler {
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
            // Same layout as 0x88.
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

        // ── I/O and GPIO opcodes (no-op for VFIO) ───────────
        0x69 => {
            vm.offset += 5;
            vm.stats.ops_skipped += 1;
        }
        0x32 => {
            let count = vm.rd08(vm.offset + 7) as usize;
            vm.offset += 8 + count * 4;
            vm.stats.ops_skipped += 1;
        }
        0x37 => {
            vm.offset += 11;
            vm.stats.ops_skipped += 1;
        }
        0x3B | 0x3C => {
            vm.offset += 5;
            vm.stats.ops_skipped += 1;
        }
        0x49 => {
            let count = vm.rd08(vm.offset + 7) as usize;
            vm.offset += 8 + count * 2;
            vm.stats.ops_skipped += 1;
        }
        0x4C => {
            vm.offset += 7;
            vm.stats.ops_skipped += 1;
        }
        0x4D => {
            vm.offset += 6;
            vm.stats.ops_skipped += 1;
        }
        0x4E => {
            let count = vm.rd08(vm.offset + 4) as usize;
            vm.offset += 5 + count;
            vm.stats.ops_skipped += 1;
        }
        0x4F => {
            // INIT_TMDS: tmds(u8) + addr(u8) + mask(u8) + data(u8) = stride 5
            vm.offset += 5;
            vm.stats.ops_skipped += 1;
        }
        0x50 => {
            // INIT_IO_RESTRICT_PROG: port(u16) + index(u8) + mask(u8) + shift(u8)
            // + count(u8) + reg(u32) + count × u32
            let count = vm.rd08(vm.offset + 6) as usize;
            vm.offset += 11 + count * 4;
            vm.stats.ops_skipped += 1;
        }
        0x51 => {
            vm.offset += 7;
            vm.stats.ops_skipped += 1;
        }
        0x52 => {
            vm.offset += 4;
            vm.stats.ops_skipped += 1;
        }
        0x53 => {
            vm.offset += 3;
            vm.stats.ops_skipped += 1;
        }
        0x54 => {
            let count = vm.rd08(vm.offset + 1) as usize;
            vm.offset += 2 + count * 2;
            vm.stats.ops_skipped += 1;
        }
        0x5E => {
            vm.offset += 6;
            vm.stats.ops_skipped += 1;
        }
        0x62 => {
            vm.offset += 5;
            vm.stats.ops_skipped += 1;
        }
        0x78 => {
            vm.offset += 6;
            vm.stats.ops_skipped += 1;
        }
        0x96 => {
            vm.offset += 11;
            vm.stats.ops_skipped += 1;
        }
        0x98 => {
            vm.offset += 8;
            vm.stats.ops_skipped += 1;
        }
        0x99 => {
            let count = vm.rd08(vm.offset + 5) as usize;
            vm.offset += 6 + count;
            vm.stats.ops_skipped += 1;
        }
        0x9A => {
            vm.offset += 9;
            vm.stats.ops_skipped += 1;
        }
        0xA9 => {
            let count = vm.rd08(vm.offset + 1) as usize;
            vm.offset += 2 + count * 2;
            vm.stats.ops_skipped += 1;
        }

        // ── Volta+ extended opcodes ──────────────────────────
        //
        // GV100 VBIOS uses opcodes not present in upstream nouveau.
        // Strides determined empirically from ROM analysis by finding
        // known opcodes at the expected successor offset.
        0x9E => {
            // Undocumented Volta opcode. Appears to be a prefix/modifier
            // for the following opcode. Stride 1 — let the next byte be
            // parsed as its own opcode.
            vm.offset += 1;
            vm.stats.ops_skipped += 1;
        }
        0xAC => {
            // Stride 13: followed by INIT_NV_REG (0x6e) at +13.
            // Layout: opcode + reg:u32 + val:u32 + unk:u32
            let reg = vm.rd32(vm.offset + 1);
            let val = vm.rd32(vm.offset + 5);
            if reg < 0x0100_0000 {
                vm.bar0_wr32(reg, val);
            }
            vm.offset += 13;
        }
        0xB0 => {
            // Stride 10: followed by INIT_ZM_REG_GROUP (0x91) at +10.
            // Layout: opcode + reg:u32 + 5 bytes data
            let reg = vm.rd32(vm.offset + 1);
            let val = vm.rd32(vm.offset + 5);
            if reg < 0x0100_0000 {
                vm.bar0_wr32(reg, val);
            }
            vm.offset += 10;
        }
        0xB1 => {
            // Stride 3: followed by INIT_RESUME (0x72) at +3.
            // Layout: opcode + index:u8 + flag:u8
            vm.offset += 3;
            vm.stats.ops_skipped += 1;
        }

        // ── Hardware-specific no-ops ─────────────────────────
        0x70 => {
            // INIT_EON — end of nested condition (complement of INIT_NOT/0x38)
            vm.execute = true;
            vm.offset += 1;
        }
        0x63 => vm.offset += 1,
        0x65 => {
            // INIT_RESET: reg(u32), value1(u32), value2(u32) — engine reset
            let reg = vm.rd32(vm.offset + 1);
            let val1 = vm.rd32(vm.offset + 5);
            let val2 = vm.rd32(vm.offset + 9);
            if vm.execute && reg < 0x0100_0000 {
                vm.bar0_wr32(reg, val1);
                std::thread::sleep(std::time::Duration::from_micros(10));
                vm.bar0_wr32(reg, val2);
                std::thread::sleep(std::time::Duration::from_micros(10));
            }
            vm.offset += 13;
        }
        0x66..=0x68 => vm.offset += 1,
        0x6F => vm.offset += 2,
        0x8C | 0x8D | 0x8E | 0x92 | 0xAA => vm.offset += 1,

        // ── Unknown opcodes ─────────────────────────────────
        _ => {
            // 0xFF in erased ROM regions means we've run past the real
            // script boundary. Treat consecutive 0xFF as end-of-script.
            if op == 0xFF {
                let next = vm.rd08(vm.offset + 1);
                if next == 0xFF {
                    tracing::debug!(
                        offset = vm.offset,
                        "VBIOS: consecutive 0xFF — treating as end-of-script"
                    );
                    vm.offset = 0;
                    return Ok(());
                }
            }
            if vm.stats.unknown_opcodes.len() < 10 {
                tracing::warn!(
                    offset = format!("{:#06x}", vm.offset),
                    opcode = format!("{op:#04x}"),
                    "VBIOS: unknown opcode"
                );
            }
            vm.stats.unknown_opcodes.push((vm.offset, op));
            vm.stats.ops_skipped += 1;
            vm.offset += 1;
            if vm.stats.unknown_opcodes.len() == 100 {
                tracing::warn!(
                    offset = format!("{:#06x}", vm.offset - 1),
                    opcode = format!("{op:#04x}"),
                    "VBIOS: stream desynced (100 unknown opcodes) — terminating script"
                );
                vm.offset = 0;
                return Ok(());
            }
        }
    }
    Ok(())
}
