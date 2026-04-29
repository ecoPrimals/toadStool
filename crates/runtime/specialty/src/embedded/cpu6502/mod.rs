// SPDX-License-Identifier: AGPL-3.0-or-later
//! NMOS 6502 subset emulator (instruction stepping + cycle accounting).
//!
//! Decimal mode (`D` flag) is not implemented for `ADC`/`SBC` (binary mode only).

#![allow(
    missing_docs,
    reason = "6502 emulator internals mirror hardware register naming"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::many_single_char_names,
    reason = "8-bit CPU emulation: truncation is the hardware semantics, single-char names match register conventions (A, X, Y, S, P)"
)]

const N: u8 = 0x80;
const V: u8 = 0x40;
const B: u8 = 0x10;
const I: u8 = 0x04;
const Z: u8 = 0x02;
const C: u8 = 0x01;

/// 6502 CPU with 64 KiB linear RAM.
#[derive(Debug, Clone)]
pub struct Cpu6502 {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub p: u8,
    pub mem: Vec<u8>,
    pub cycles: u64,
    pub halted: bool,
}

impl Default for Cpu6502 {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu6502 {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            p: 0x34,
            mem: vec![0u8; 65536],
            cycles: 0,
            halted: false,
        }
    }

    fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, v: u8) {
        self.mem[addr as usize] = v;
    }

    fn read16(&self, addr: u16) -> u16 {
        let lo = u16::from(self.read(addr));
        let hi = u16::from(self.read(addr.wrapping_add(1)));
        lo | (hi << 8)
    }

    fn push(&mut self, v: u8) {
        self.write(0x0100 | u16::from(self.sp), v);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.read(0x0100 | u16::from(self.sp))
    }

    /// Load bytes into memory starting at `addr` (wraps at 64K).
    pub fn load(&mut self, addr: u16, data: &[u8]) {
        for (i, b) in data.iter().enumerate() {
            self.mem[(usize::from(addr) + i) & 0xFFFF] = *b;
        }
    }

    /// Reset vector fetch from `0xFFFC`.
    pub fn reset(&mut self) {
        self.pc = self.read16(0xFFFC);
        self.sp = 0xFD;
        self.p |= I;
        self.halted = false;
    }
}

mod alu;
mod decode;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
