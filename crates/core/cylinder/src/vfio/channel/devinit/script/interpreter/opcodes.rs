// SPDX-License-Identifier: AGPL-3.0-or-later

#[path = "opcodes_clock.rs"]
mod clock;
#[path = "opcodes_control.rs"]
mod control;
#[path = "opcodes_extended.rs"]
mod extended;
#[path = "opcodes_io.rs"]
mod io;
#[path = "opcodes_register.rs"]
mod register;

use crate::error::DevinitError;

use super::VbiosInterpreter;

/// Execute a single VBIOS init opcode.
pub(super) fn dispatch_opcode(
    vm: &mut VbiosInterpreter<'_>,
    op: u8,
    cond_table: usize,
) -> Result<(), DevinitError> {
    match op {
        // ── Control flow, conditions, delays ────────────────
        0x33 | 0x36 | 0x38 | 0x39 | 0x3A | 0x56 | 0x57 | 0x5B | 0x5C | 0x6B | 0x6D | 0x71
        | 0x72 | 0x73 | 0x74 | 0x75 | 0x76 => control::dispatch_control(vm, op, cond_table),

        // ── Register writes ─────────────────────────────────
        0x47 | 0x48 | 0x58 | 0x5A | 0x5F | 0x6E | 0x77 | 0x7A | 0x90 | 0x91 | 0x97 => {
            register::dispatch_register(vm, op)
        }

        // ── PLL and RAM-restrict memory opcodes ─────────────
        0x34 | 0x4A | 0x4B | 0x59 | 0x79 | 0x87 | 0x88 | 0x8F => clock::dispatch_clock(vm, op),

        // ── I/O and GPIO (no-op for VFIO) ───────────────────
        0x32 | 0x37 | 0x3B | 0x3C | 0x49 | 0x4C | 0x4D | 0x4E | 0x4F | 0x50 | 0x51 | 0x52
        | 0x53 | 0x54 | 0x5E | 0x62 | 0x69 | 0x78 | 0x96 | 0x98 | 0x99 | 0x9A | 0xA9 => {
            io::dispatch_io(vm, op)
        }

        // ── Volta+ and hardware-specific ────────────────────
        0x63
        | 0x65
        | 0x66..=0x68
        | 0x6F
        | 0x70
        | 0x8C
        | 0x8D
        | 0x8E
        | 0x92
        | 0x9E
        | 0xAA
        | 0xAC
        | 0xB0
        | 0xB1 => extended::dispatch_extended(vm, op),

        // ── Unknown opcodes ─────────────────────────────────
        _ => extended::dispatch_unknown(vm, op),
    }
}
