// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::DevinitError;

use super::super::VbiosInterpreter;

/// Volta+ extended opcodes, hardware-specific handlers, and unknown-op fallback.
pub(super) fn dispatch_extended(vm: &mut VbiosInterpreter<'_>, op: u8) -> Result<(), DevinitError> {
    match op {
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

        _ => unreachable!("dispatch_extended called with non-extended opcode {op:#04x}"),
    }
    Ok(())
}

/// Handle unknown opcodes and end-of-ROM 0xFF sentinel.
pub(super) fn dispatch_unknown(vm: &mut VbiosInterpreter<'_>, op: u8) -> Result<(), DevinitError> {
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
    }
    Ok(())
}
