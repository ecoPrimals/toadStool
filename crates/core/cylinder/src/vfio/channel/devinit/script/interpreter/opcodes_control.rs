// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::DevinitError;

use super::super::VbiosInterpreter;

/// Termination, control flow, conditions, and delay opcodes.
pub(super) fn dispatch_control(
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
                    vm.bar0.delay_us(50_000);
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
                    vm.bar0.delay_us(usec);
                } else {
                    vm.bar0.delay_us(usec);
                }
                vm.stats.delays_total_us += usec;
            }
            vm.offset += 3;
        }
        0x57 => {
            // INIT_LTIME: usec(u16) — same as TIME
            let usec = vm.rd16(vm.offset + 1) as u64;
            if vm.execute && usec > 0 {
                vm.bar0.delay_us(usec);
                vm.stats.delays_total_us += usec;
            }
            vm.offset += 3;
        }

        _ => unreachable!("dispatch_control called with non-control opcode {op:#04x}"),
    }
    Ok(())
}
