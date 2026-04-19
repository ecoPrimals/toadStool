// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait implementations for embedded emulators (MOS 6502 and Zilog Z80 cores).
//!
//! See DEBT.md `D-EMBEDDED-EMULATOR` for GDB / remote-debug transport tracking.

use crate::SpecialtyRuntimeError;
use toadstool::ToadStoolError;

use super::errors::EmbeddedEmulatorError;

mod mos6502;
mod z80;

#[cfg(test)]
mod tests;

fn emulator_err(e: EmbeddedEmulatorError) -> ToadStoolError {
    SpecialtyRuntimeError::from(e).into()
}

fn copy_mem_range(mem: &[u8], addr: u32, len: u32) -> Result<Vec<u8>, EmbeddedEmulatorError> {
    let end = u64::from(addr).checked_add(u64::from(len)).ok_or_else(|| {
        EmbeddedEmulatorError::NotReady {
            detail: "memory read overflow".into(),
        }
    })?;
    if end > 65536 {
        return Err(EmbeddedEmulatorError::NotReady {
            detail: "memory read past 64K".into(),
        });
    }
    let start = addr as usize;
    let end = end as usize;
    Ok(mem[start..end].to_vec())
}

fn write_mem_range(mem: &mut [u8], addr: u32, data: &[u8]) -> Result<(), EmbeddedEmulatorError> {
    let end = u64::from(addr)
        .checked_add(data.len() as u64)
        .ok_or_else(|| EmbeddedEmulatorError::NotReady {
            detail: "memory write overflow".into(),
        })?;
    if end > 65536 {
        return Err(EmbeddedEmulatorError::NotReady {
            detail: "memory write past 64K".into(),
        });
    }
    let start = addr as usize;
    mem[start..start + data.len()].copy_from_slice(data);
    Ok(())
}
