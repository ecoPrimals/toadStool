// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emulator front-ends wrapping pure CPU cores (`cpu6502`, `cpuz80`).

use std::collections::HashSet;

use crate::embedded::cpu6502::Cpu6502;
use crate::embedded::cpuz80::Z80Cpu;
use crate::embedded::types::EmulationStatus;

/// 6502 emulator: [`Cpu6502`] core, breakpoints, and lifecycle flags.
#[derive(Debug)]
#[cfg_attr(
    not(feature = "embedded-placeholder-impls"),
    allow(dead_code, reason = "fields used only when adapter impls are compiled")
)]
pub struct Emulator6502 {
    pub(crate) cpu: Cpu6502,
    pub(crate) breakpoints: HashSet<u32>,
    pub(crate) initialized: bool,
    pub(crate) image_loaded: bool,
    pub(crate) running: bool,
    pub(crate) status: EmulationStatus,
}

/// Z80 emulator.
#[derive(Debug)]
#[cfg_attr(
    not(feature = "embedded-placeholder-impls"),
    allow(dead_code, reason = "fields used only when adapter impls are compiled")
)]
pub struct EmulatorZ80 {
    pub(crate) cpu: Z80Cpu,
    pub(crate) breakpoints: HashSet<u32>,
    pub(crate) initialized: bool,
    pub(crate) image_loaded: bool,
    pub(crate) running: bool,
    pub(crate) status: EmulationStatus,
}

impl Emulator6502 {
    /// Create an emulator with cleared memory and reset CPU state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Emulator6502 {
    fn default() -> Self {
        Self {
            cpu: Cpu6502::new(),
            breakpoints: HashSet::new(),
            initialized: false,
            image_loaded: false,
            running: false,
            status: EmulationStatus::Stopped,
        }
    }
}

impl EmulatorZ80 {
    /// Create an emulator with cleared memory.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for EmulatorZ80 {
    fn default() -> Self {
        Self {
            cpu: Z80Cpu::new(),
            breakpoints: HashSet::new(),
            initialized: false,
            image_loaded: false,
            running: false,
            status: EmulationStatus::Stopped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emulator_6502_new_and_default_agree() {
        let a = Emulator6502::new();
        let b = Emulator6502::default();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert_eq!(a.cpu.mem.len(), 65536);
    }

    #[test]
    fn emulator_z80_new_and_default_agree() {
        let a = EmulatorZ80::new();
        let b = EmulatorZ80::default();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert_eq!(a.cpu.mem.len(), 65536);
    }

    #[test]
    fn emulator_types_debug_contains_struct_names() {
        let s6502 = format!("{:?}", Emulator6502::new());
        let sz80 = format!("{:?}", EmulatorZ80::new());
        assert!(s6502.contains("Emulator6502"), "{s6502}");
        assert!(sz80.contains("EmulatorZ80"), "{sz80}");
    }
}
