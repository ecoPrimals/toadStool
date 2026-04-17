// SPDX-License-Identifier: AGPL-3.0-or-later
//! NMOS 6502 subset emulator (instruction stepping + cycle accounting).
//!
//! Decimal mode (`D` flag) is not implemented for `ADC`/`SBC` (binary mode only).

#![allow(missing_docs)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::many_single_char_names,
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

    fn set_nz(&mut self, v: u8) {
        self.p &= !(N | Z);
        if v == 0 {
            self.p |= Z;
        }
        if v & 0x80 != 0 {
            self.p |= N;
        }
    }

    fn cmp_val(&mut self, r: u8, m: u8) {
        let d = u16::from(r).wrapping_sub(u16::from(m));
        self.set_nz(d as u8);
        self.p &= !C;
        if r >= m {
            self.p |= C;
        }
    }

    fn adc(&mut self, m: u8) {
        let c = u16::from(self.p & C);
        let a = u16::from(self.a);
        let mv = u16::from(m);
        let s = a + mv + c;
        self.p &= !(C | V | Z | N);
        if s > 0xFF {
            self.p |= C;
        }
        let r = s as u8;
        if (a ^ mv) & 0x80 == 0 && (a ^ u16::from(r)) & 0x80 != 0 {
            self.p |= V;
        }
        self.a = r;
        self.set_nz(self.a);
    }

    fn sbc(&mut self, m: u8) {
        self.adc(!m);
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

    /// Execute one instruction; returns cycles consumed for that instruction.
    pub fn step(&mut self) -> u32 {
        if self.halted {
            return 0;
        }
        let opc = self.read(self.pc);
        let ip = self.pc;
        self.pc = self.pc.wrapping_add(1);

        macro_rules! addr_zp {
            () => {{
                let z = self.read(ip.wrapping_add(1)) as u16;
                self.pc = self.pc.wrapping_add(1);
                z
            }};
        }
        macro_rules! addr_zpx {
            () => {{
                let z = self.read(ip.wrapping_add(1)).wrapping_add(self.x) as u16;
                self.pc = self.pc.wrapping_add(1);
                z & 0xFF
            }};
        }
        macro_rules! addr_abs {
            () => {{
                let a = self.read16(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(2);
                a
            }};
        }
        macro_rules! addr_absx {
            () => {{
                let base = self.read16(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(2);
                let addr = base.wrapping_add(self.x.into());
                (addr, base & 0xFF00 != addr & 0xFF00)
            }};
        }
        macro_rules! addr_absy {
            () => {{
                let base = self.read16(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(2);
                let addr = base.wrapping_add(self.y.into());
                (addr, base & 0xFF00 != addr & 0xFF00)
            }};
        }
        macro_rules! addr_ind_x {
            () => {{
                let z = self.read(ip.wrapping_add(1)).wrapping_add(self.x) as u16;
                self.pc = self.pc.wrapping_add(1);
                let p = z & 0xFF;
                let lo = self.read(p);
                let hi = self.read((p + 1) & 0xFF);
                u16::from(lo) | (u16::from(hi) << 8)
            }};
        }
        macro_rules! addr_ind_y {
            () => {{
                let z = self.read(ip.wrapping_add(1)) as u16;
                self.pc = self.pc.wrapping_add(1);
                let base = self.read16(z & 0xFF | (z & 0xFF00));
                let addr = base.wrapping_add(self.y.into());
                (addr, base & 0xFF00 != addr & 0xFF00)
            }};
        }

        let mut cyc: u32 = 2;

        match opc {
            0xEA => {} // NOP
            0x00 => {
                // BRK
                self.pc = ip.wrapping_add(2);
                self.push((self.pc >> 8) as u8);
                self.push(self.pc as u8);
                self.push(self.p | B);
                self.p |= I;
                self.pc = self.read16(0xFFFE);
                cyc = 7;
            }
            0x18 => {
                self.p &= !C;
                cyc = 2;
            }
            0x38 => {
                self.p |= C;
                cyc = 2;
            }
            // LDA
            0xA9 => {
                let v = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.a = v;
                self.set_nz(self.a);
                cyc = 2;
            }
            0xA5 => {
                let a = addr_zp!();
                self.a = self.read(a);
                self.set_nz(self.a);
                cyc = 3;
            }
            0xAD => {
                let a = addr_abs!();
                self.a = self.read(a);
                self.set_nz(self.a);
                cyc = 4;
            }
            0xB5 => {
                let a = addr_zpx!();
                self.a = self.read(a);
                self.set_nz(self.a);
                cyc = 4;
            }
            0xBD => {
                let (a, pg) = addr_absx!();
                self.a = self.read(a);
                self.set_nz(self.a);
                cyc = 4 + pg as u32;
            }
            0xB9 => {
                let (a, pg) = addr_absy!();
                self.a = self.read(a);
                self.set_nz(self.a);
                cyc = 4 + pg as u32;
            }
            0xA1 => {
                let a = addr_ind_x!();
                self.a = self.read(a);
                self.set_nz(self.a);
                cyc = 6;
            }
            0xB1 => {
                let (a, pg) = addr_ind_y!();
                self.a = self.read(a);
                self.set_nz(self.a);
                cyc = 5 + pg as u32;
            }
            // STA
            0x85 => {
                let a = addr_zp!();
                self.write(a, self.a);
                cyc = 3;
            }
            0x8D => {
                let a = addr_abs!();
                self.write(a, self.a);
                cyc = 4;
            }
            0x95 => {
                let a = addr_zpx!();
                self.write(a, self.a);
                cyc = 4;
            }
            0x9D => {
                let (a, _) = addr_absx!();
                self.write(a, self.a);
                cyc = 5;
            }
            0x99 => {
                let (a, _) = addr_absy!();
                self.write(a, self.a);
                cyc = 5;
            }
            0x81 => {
                let a = addr_ind_x!();
                self.write(a, self.a);
                cyc = 6;
            }
            0x91 => {
                let (a, _) = addr_ind_y!();
                self.write(a, self.a);
                cyc = 6;
            }
            // LDX
            0xA2 => {
                let v = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.x = v;
                self.set_nz(self.x);
                cyc = 2;
            }
            0xA6 => {
                let a = addr_zp!();
                self.x = self.read(a);
                self.set_nz(self.x);
                cyc = 3;
            }
            0xAE => {
                let a = addr_abs!();
                self.x = self.read(a);
                self.set_nz(self.x);
                cyc = 4;
            }
            0xB6 => {
                let z = self.read(ip.wrapping_add(1)).wrapping_add(self.y) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.x = self.read(z & 0xFF);
                self.set_nz(self.x);
                cyc = 4;
            }
            0xBE => {
                let base = self.read16(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(2);
                let addr = base.wrapping_add(self.y.into());
                let pg = base & 0xFF00 != addr & 0xFF00;
                self.x = self.read(addr);
                self.set_nz(self.x);
                cyc = 4 + pg as u32;
            }
            // STX
            0x86 => {
                let a = addr_zp!();
                self.write(a, self.x);
                cyc = 3;
            }
            0x8E => {
                let a = addr_abs!();
                self.write(a, self.x);
                cyc = 4;
            }
            0x96 => {
                let z = self.read(ip.wrapping_add(1)).wrapping_add(self.y) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.write(z & 0xFF, self.x);
                cyc = 4;
            }
            // LDY
            0xA0 => {
                let v = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.y = v;
                self.set_nz(self.y);
                cyc = 2;
            }
            0xA4 => {
                let a = addr_zp!();
                self.y = self.read(a);
                self.set_nz(self.y);
                cyc = 3;
            }
            0xAC => {
                let a = addr_abs!();
                self.y = self.read(a);
                self.set_nz(self.y);
                cyc = 4;
            }
            0xB4 => {
                let a = addr_zpx!();
                self.y = self.read(a);
                self.set_nz(self.y);
                cyc = 4;
            }
            0xBC => {
                let (a, pg) = addr_absx!();
                self.y = self.read(a);
                self.set_nz(self.y);
                cyc = 4 + pg as u32;
            }
            // STY
            0x84 => {
                let a = addr_zp!();
                self.write(a, self.y);
                cyc = 3;
            }
            0x8C => {
                let a = addr_abs!();
                self.write(a, self.y);
                cyc = 4;
            }
            0x94 => {
                let a = addr_zpx!();
                self.write(a, self.y);
                cyc = 4;
            }
            // ADC / SBC
            0x69 => {
                let m = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.adc(m);
                cyc = 2;
            }
            0x65 => {
                let a = addr_zp!();
                let m = self.read(a);
                self.adc(m);
                cyc = 3;
            }
            0x6D => {
                let a = addr_abs!();
                let m = self.read(a);
                self.adc(m);
                cyc = 4;
            }
            0x61 => {
                let a = addr_ind_x!();
                let m = self.read(a);
                self.adc(m);
                cyc = 6;
            }
            0x71 => {
                let (a, pg) = addr_ind_y!();
                let m = self.read(a);
                self.adc(m);
                cyc = 5 + pg as u32;
            }
            0x75 => {
                let a = addr_zpx!();
                let m = self.read(a);
                self.adc(m);
                cyc = 4;
            }
            0x7D => {
                let (a, pg) = addr_absx!();
                let m = self.read(a);
                self.adc(m);
                cyc = 4 + pg as u32;
            }
            0x79 => {
                let (a, pg) = addr_absy!();
                let m = self.read(a);
                self.adc(m);
                cyc = 4 + pg as u32;
            }
            0xE9 | 0xEB => {
                let m = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.sbc(m);
                cyc = 2;
            }
            0xE5 => {
                let a = addr_zp!();
                let m = self.read(a);
                self.sbc(m);
                cyc = 3;
            }
            0xED => {
                let a = addr_abs!();
                let m = self.read(a);
                self.sbc(m);
                cyc = 4;
            }
            0xE1 => {
                let a = addr_ind_x!();
                let m = self.read(a);
                self.sbc(m);
                cyc = 6;
            }
            0xF1 => {
                let (a, pg) = addr_ind_y!();
                let m = self.read(a);
                self.sbc(m);
                cyc = 5 + pg as u32;
            }
            0xF5 => {
                let a = addr_zpx!();
                let m = self.read(a);
                self.sbc(m);
                cyc = 4;
            }
            0xFD => {
                let (a, pg) = addr_absx!();
                let m = self.read(a);
                self.sbc(m);
                cyc = 4 + pg as u32;
            }
            0xF9 => {
                let (a, pg) = addr_absy!();
                let m = self.read(a);
                self.sbc(m);
                cyc = 4 + pg as u32;
            }
            // AND ORA EOR
            0x29 => {
                let m = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.a &= m;
                self.set_nz(self.a);
                cyc = 2;
            }
            0x25 => {
                let a = addr_zp!();
                self.a &= self.read(a);
                self.set_nz(self.a);
                cyc = 3;
            }
            0x2D => {
                let a = addr_abs!();
                self.a &= self.read(a);
                self.set_nz(self.a);
                cyc = 4;
            }
            0x09 => {
                let m = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.a |= m;
                self.set_nz(self.a);
                cyc = 2;
            }
            0x05 => {
                let a = addr_zp!();
                self.a |= self.read(a);
                self.set_nz(self.a);
                cyc = 3;
            }
            0x0D => {
                let a = addr_abs!();
                self.a |= self.read(a);
                self.set_nz(self.a);
                cyc = 4;
            }
            0x49 => {
                let m = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.a ^= m;
                self.set_nz(self.a);
                cyc = 2;
            }
            0x45 => {
                let a = addr_zp!();
                self.a ^= self.read(a);
                self.set_nz(self.a);
                cyc = 3;
            }
            0x4D => {
                let a = addr_abs!();
                self.a ^= self.read(a);
                self.set_nz(self.a);
                cyc = 4;
            }
            // CMP
            0xC9 => {
                let m = self.read(ip.wrapping_add(1));
                self.pc = self.pc.wrapping_add(1);
                self.cmp_val(self.a, m);
                cyc = 2;
            }
            0xC5 => {
                let a = addr_zp!();
                self.cmp_val(self.a, self.read(a));
                cyc = 3;
            }
            0xCD => {
                let a = addr_abs!();
                self.cmp_val(self.a, self.read(a));
                cyc = 4;
            }
            0xC1 => {
                let a = addr_ind_x!();
                self.cmp_val(self.a, self.read(a));
                cyc = 6;
            }
            0xD1 => {
                let (a, pg) = addr_ind_y!();
                self.cmp_val(self.a, self.read(a));
                cyc = 5 + pg as u32;
            }
            0xD5 => {
                let a = addr_zpx!();
                self.cmp_val(self.a, self.read(a));
                cyc = 4;
            }
            0xDD => {
                let (a, pg) = addr_absx!();
                self.cmp_val(self.a, self.read(a));
                cyc = 4 + pg as u32;
            }
            0xD9 => {
                let (a, pg) = addr_absy!();
                self.cmp_val(self.a, self.read(a));
                cyc = 4 + pg as u32;
            }
            // JMP
            0x4C => {
                self.pc = self.read16(ip.wrapping_add(1));
                cyc = 3;
            }
            0x6C => {
                let ptr = self.read16(ip.wrapping_add(1));
                let lo = self.read(ptr);
                let hi = if ptr & 0xFF == 0xFF {
                    self.read(ptr & 0xFF00)
                } else {
                    self.read(ptr.wrapping_add(1))
                };
                self.pc = u16::from(lo) | (u16::from(hi) << 8);
                cyc = 5;
            }
            // JSR / RTI / RTS
            0x20 => {
                let target = self.read16(ip.wrapping_add(1));
                // Stack gets address of last byte of JSR (`ip + 2`); RTS adds 1 → return to `ip + 3`.
                let ret = ip.wrapping_add(2);
                self.push((ret >> 8) as u8);
                self.push(ret as u8);
                self.pc = target;
                cyc = 6;
            }
            0x40 => {
                // RTI
                self.p = self.pop() & !B;
                let lo = u16::from(self.pop());
                let hi = u16::from(self.pop());
                self.pc = hi << 8 | lo;
                cyc = 6;
            }
            0x60 => {
                // RTS
                let lo = u16::from(self.pop());
                let hi = u16::from(self.pop());
                self.pc = (hi << 8 | lo).wrapping_add(1);
                cyc = 6;
            }
            // Branches (rel)
            0xF0 => {
                let off = self.read(ip.wrapping_add(1)) as i8;
                self.pc = self.pc.wrapping_add(1);
                if self.p & Z != 0 {
                    let old = self.pc;
                    self.pc = self.pc.wrapping_add(off as u16);
                    cyc = 3 + (old & 0xFF00 != self.pc & 0xFF00) as u32;
                } else {
                    cyc = 2;
                }
            }
            0xD0 => {
                let off = self.read(ip.wrapping_add(1)) as i8;
                self.pc = self.pc.wrapping_add(1);
                if self.p & Z == 0 {
                    let old = self.pc;
                    self.pc = self.pc.wrapping_add(off as u16);
                    cyc = 3 + (old & 0xFF00 != self.pc & 0xFF00) as u32;
                } else {
                    cyc = 2;
                }
            }
            0x90 => {
                let off = self.read(ip.wrapping_add(1)) as i8;
                self.pc = self.pc.wrapping_add(1);
                if self.p & C == 0 {
                    let old = self.pc;
                    self.pc = self.pc.wrapping_add(off as u16);
                    cyc = 3 + (old & 0xFF00 != self.pc & 0xFF00) as u32;
                } else {
                    cyc = 2;
                }
            }
            0xB0 => {
                let off = self.read(ip.wrapping_add(1)) as i8;
                self.pc = self.pc.wrapping_add(1);
                if self.p & C != 0 {
                    let old = self.pc;
                    self.pc = self.pc.wrapping_add(off as u16);
                    cyc = 3 + (old & 0xFF00 != self.pc & 0xFF00) as u32;
                } else {
                    cyc = 2;
                }
            }
            // PHA PLA
            0x48 => {
                self.push(self.a);
                cyc = 3;
            }
            0x68 => {
                self.a = self.pop();
                self.set_nz(self.a);
                cyc = 4;
            }
            // TAX TXA TAY TYA
            0xAA => {
                self.x = self.a;
                self.set_nz(self.x);
                cyc = 2;
            }
            0x8A => {
                self.a = self.x;
                self.set_nz(self.a);
                cyc = 2;
            }
            0xA8 => {
                self.y = self.a;
                self.set_nz(self.y);
                cyc = 2;
            }
            0x98 => {
                self.a = self.y;
                self.set_nz(self.a);
                cyc = 2;
            }
            // INX INY DEX DEY
            0xE8 => {
                self.x = self.x.wrapping_add(1);
                self.set_nz(self.x);
                cyc = 2;
            }
            0xC8 => {
                self.y = self.y.wrapping_add(1);
                self.set_nz(self.y);
                cyc = 2;
            }
            0xCA => {
                self.x = self.x.wrapping_sub(1);
                self.set_nz(self.x);
                cyc = 2;
            }
            0x88 => {
                self.y = self.y.wrapping_sub(1);
                self.set_nz(self.y);
                cyc = 2;
            }
            _ => {
                // Unknown opcode: treat as NOP for stability in tests
                self.pc = ip.wrapping_add(1);
                cyc = 2;
            }
        }

        self.cycles += u64::from(cyc);
        cyc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_snippet(mem: &[u8], at: u16) -> Cpu6502 {
        let mut c = Cpu6502::new();
        c.load(at, mem);
        c.pc = at;
        for _ in 0..64 {
            c.step();
        }
        c
    }

    #[test]
    fn lda_imm_and_sta_zp() {
        let c = run_snippet(&[0xA9, 0x42, 0x85, 0x10], 0x0200);
        assert_eq!(c.a, 0x42);
        assert_eq!(c.read(0x0010), 0x42);
    }

    #[test]
    fn adc_immediate_carry() {
        let mut c = Cpu6502::new();
        c.load(0x300, &[0x38, 0x69, 0xFF]);
        c.pc = 0x300;
        c.step();
        c.step();
        assert_eq!(c.a, 0x00);
        assert!(c.p & C != 0);
    }

    #[test]
    fn beq_takes_branch() {
        let mut c = Cpu6502::new();
        c.load(0x200, &[0xA9, 0x00, 0xF0, 0x02, 0xA9, 0x01, 0xA9, 0x02]);
        c.pc = 0x200;
        c.step();
        c.step();
        c.step();
        assert_eq!(c.a, 0x02);
    }

    #[test]
    fn jsr_rts() {
        let mut c = Cpu6502::new();
        c.mem[0xFFFC] = 0x00;
        c.mem[0xFFFD] = 0x04;
        c.load(0x0400, &[0x20, 0x10, 0x04, 0xA9, 0x99, 0xEA]);
        c.load(0x0410, &[0xA9, 0x55, 0x60]);
        c.reset();
        assert_eq!(c.pc, 0x0400);
        c.step();
        assert_eq!(c.pc, 0x0410);
        c.step();
        assert_eq!(c.a, 0x55);
        c.step();
        assert_eq!(c.pc, 0x0403);
        c.step();
        assert_eq!(c.a, 0x99);
    }

    #[test]
    fn jmp_indirect_page_bug() {
        let mut c = Cpu6502::new();
        c.mem[0x50FF] = 0x50;
        c.mem[0x5000] = 0xEE;
        c.load(0x200, &[0x6C, 0xFF, 0x50]);
        c.pc = 0x200;
        c.step();
        assert_eq!(c.pc, 0xEE50);
    }
}
