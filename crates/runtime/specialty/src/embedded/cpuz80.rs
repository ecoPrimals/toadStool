// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal Zilog Z80 emulator (single-byte opcodes + selected `0xED` block).

#![allow(
    missing_docs,
    reason = "Z80 emulator internals mirror hardware register naming"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    reason = "8-bit CPU emulation: truncation is the hardware semantics"
)]

/// Z80 CPU with 64 KiB RAM.
#[derive(Debug, Clone)]
pub struct Z80Cpu {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub ix: u16,
    pub iy: u16,
    pub sp: u16,
    pub pc: u16,
    pub mem: Vec<u8>,
    pub cycles: u64,
    pub halted: bool,
}

impl Default for Z80Cpu {
    fn default() -> Self {
        Self::new()
    }
}

const S: u8 = 0x80;
const Z: u8 = 0x40;
const H: u8 = 0x10;
const PV: u8 = 0x04;
const N: u8 = 0x02;
const C: u8 = 0x01;

impl Z80Cpu {
    pub fn new() -> Self {
        Self {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            ix: 0,
            iy: 0,
            sp: 0xFFFF,
            pc: 0,
            mem: vec![0u8; 65536],
            cycles: 0,
            halted: false,
        }
    }

    fn read(&self, a: u16) -> u8 {
        self.mem[a as usize]
    }

    fn write(&mut self, a: u16, v: u8) {
        self.mem[a as usize] = v;
    }

    fn read16_imm(&mut self) -> u16 {
        let lo = u16::from(self.read(self.pc));
        self.pc = self.pc.wrapping_add(1);
        let hi = u16::from(self.read(self.pc));
        self.pc = self.pc.wrapping_add(1);
        lo | (hi << 8)
    }

    fn read8_imm(&mut self) -> u8 {
        let v = self.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }

    fn hl(&self) -> u16 {
        u16::from(self.l) | (u16::from(self.h) << 8)
    }

    fn set_hl(&mut self, v: u16) {
        self.l = v as u8;
        self.h = (v >> 8) as u8;
    }

    fn bc(&self) -> u16 {
        u16::from(self.c) | (u16::from(self.b) << 8)
    }

    fn de(&self) -> u16 {
        u16::from(self.e) | (u16::from(self.d) << 8)
    }

    fn set_bc(&mut self, v: u16) {
        self.c = v as u8;
        self.b = (v >> 8) as u8;
    }

    fn set_de(&mut self, v: u16) {
        self.e = v as u8;
        self.d = (v >> 8) as u8;
    }

    fn set_nz_pv_c(&mut self, val: u8, carry: bool, half: bool, subtract: bool) {
        self.f = 0;
        if val == 0 {
            self.f |= Z;
        }
        if val & 0x80 != 0 {
            self.f |= S;
        }
        if carry {
            self.f |= C;
        }
        if half {
            self.f |= H;
        }
        if subtract {
            self.f |= N;
        }
        if val.count_ones().is_multiple_of(2) {
            self.f |= PV;
        }
    }

    fn alu_add(&mut self, v: u8, with_c: bool) {
        let c0 = u16::from(with_c && (self.f & C != 0));
        let r = u16::from(self.a) + u16::from(v) + c0;
        let h = ((self.a & 0xF) + (v & 0xF) + c0 as u8) > 0xF;
        let out = r as u8;
        self.set_nz_pv_c(out, r > 0xFF, h, false);
        self.a = out;
    }

    fn alu_sub(&mut self, v: u8, with_c: bool) {
        let c0 = i16::from(with_c && (self.f & C != 0));
        let r = i16::from(self.a) - i16::from(v) - c0;
        let out = r as u8;
        let h = (self.a as i16 & 0xF) - (v as i16 & 0xF) - c0 < 0;
        self.set_nz_pv_c(out, r < 0, h, true);
        self.a = out;
    }

    fn alu_and(&mut self, v: u8) {
        self.a &= v;
        self.f = H;
        if self.a == 0 {
            self.f |= Z;
        }
        if self.a & 0x80 != 0 {
            self.f |= S;
        }
        if self.a.count_ones().is_multiple_of(2) {
            self.f |= PV;
        }
    }

    fn alu_or(&mut self, v: u8) {
        self.a |= v;
        self.f = 0;
        if self.a == 0 {
            self.f |= Z;
        }
        if self.a & 0x80 != 0 {
            self.f |= S;
        }
        if self.a.count_ones().is_multiple_of(2) {
            self.f |= PV;
        }
    }

    fn alu_xor(&mut self, v: u8) {
        self.a ^= v;
        self.f = 0;
        if self.a == 0 {
            self.f |= Z;
        }
        if self.a & 0x80 != 0 {
            self.f |= S;
        }
        if self.a.count_ones().is_multiple_of(2) {
            self.f |= PV;
        }
    }

    fn alu_cp(&mut self, v: u8) {
        let r = i16::from(self.a) - i16::from(v);
        let out = r as u8;
        let h = (self.a as i16 & 0xF) - (v as i16 & 0xF) < 0;
        self.set_nz_pv_c(out, r < 0, h, true);
    }

    pub fn load(&mut self, addr: u16, data: &[u8]) {
        for (i, b) in data.iter().enumerate() {
            self.mem[(usize::from(addr) + i) & 0xFFFF] = *b;
        }
    }

    /// Single instruction step; returns cycles.
    pub fn step(&mut self) -> u32 {
        if self.halted {
            return 0;
        }
        let opc = self.read(self.pc);
        self.pc = self.pc.wrapping_add(1);

        let cyc: u32;

        if opc == 0x00 {
            // NOP
            cyc = 4;
        } else if opc == 0x76 {
            self.halted = true;
            cyc = 4;
        } else if opc == 0xED {
            let op2 = self.read(self.pc);
            self.pc = self.pc.wrapping_add(1);
            match op2 {
                0xB0 => {
                    // LDIR — simplified: not full block repeat
                    let v = self.read(self.hl());
                    self.write(self.de(), v);
                    self.set_hl(self.hl().wrapping_add(1));
                    self.set_de(self.de().wrapping_add(1));
                    let bc = self.bc().wrapping_sub(1);
                    self.set_bc(bc);
                    if bc != 0 {
                        self.pc = self.pc.wrapping_sub(2);
                    }
                    cyc = 21;
                }
                _ => {
                    cyc = 8;
                }
            }
        } else if (0x40..=0x7F).contains(&opc) && opc != 0x76 {
            // LD r,r' (including HALT excluded)
            let dst = (opc >> 3) & 7;
            let src = opc & 7;
            let v = self.get_r(src);
            self.set_r(dst, v);
            cyc = if src == 6 || dst == 6 { 7 } else { 4 };
        } else if (0x80..=0xBF).contains(&opc) {
            let y = (opc >> 3) & 7;
            let src = opc & 7;
            let v = self.get_r(src);
            match y {
                0 => self.alu_add(v, false),
                1 => self.alu_add(v, true),
                2 => self.alu_sub(v, false),
                3 => self.alu_sub(v, true),
                4 => self.alu_and(v),
                5 => self.alu_xor(v),
                6 => self.alu_or(v),
                7 => self.alu_cp(v),
                _ => {}
            }
            cyc = if src == 6 { 7 } else { 4 };
        } else {
            match opc {
                0x01 => {
                    let v = self.read16_imm();
                    self.set_bc(v);
                    cyc = 10;
                }
                0x11 => {
                    let v = self.read16_imm();
                    self.set_de(v);
                    cyc = 10;
                }
                0x21 => {
                    let v = self.read16_imm();
                    self.set_hl(v);
                    cyc = 10;
                }
                0x31 => {
                    self.sp = self.read16_imm();
                    cyc = 10;
                }
                0xC3 => {
                    self.pc = self.read16_imm();
                    cyc = 10;
                }
                0x18 => {
                    let d = self.read8_imm() as i8;
                    self.pc = self.pc.wrapping_add(d as u16);
                    cyc = 12;
                }
                0xCD => {
                    let t = self.read16_imm();
                    let ret = self.pc;
                    self.push16(ret);
                    self.pc = t;
                    cyc = 17;
                }
                0xC9 => {
                    self.pc = self.pop16();
                    cyc = 10;
                }
                0xF5 => {
                    self.push16(u16::from(self.a) << 8 | u16::from(self.f));
                    cyc = 11;
                }
                0xF1 => {
                    let af = self.pop16();
                    self.a = (af >> 8) as u8;
                    self.f = af as u8;
                    cyc = 10;
                }
                0xC5 => {
                    self.push16(self.bc());
                    cyc = 11;
                }
                0xC1 => {
                    let v = self.pop16();
                    self.set_bc(v);
                    cyc = 10;
                }
                0x03 => {
                    self.set_bc(self.bc().wrapping_add(1));
                    cyc = 6;
                }
                0x0B => {
                    self.set_bc(self.bc().wrapping_sub(1));
                    cyc = 6;
                }
                0x34 => {
                    let a = self.hl();
                    let v = self.read(a).wrapping_add(1);
                    self.write(a, v);
                    self.set_nz_pv_c(v, false, false, false);
                    cyc = 11;
                }
                0x35 => {
                    let a = self.hl();
                    let v = self.read(a).wrapping_sub(1);
                    self.write(a, v);
                    self.set_nz_pv_c(v, false, false, true);
                    cyc = 11;
                }
                0x3E => {
                    self.a = self.read8_imm();
                    self.f = 0;
                    if self.a == 0 {
                        self.f |= Z;
                    }
                    if self.a & 0x80 != 0 {
                        self.f |= S;
                    }
                    if self.a.count_ones().is_multiple_of(2) {
                        self.f |= PV;
                    }
                    cyc = 7;
                }
                _ => {
                    cyc = 4;
                }
            }
        }

        self.cycles += u64::from(cyc);
        cyc
    }

    fn get_r(&self, r: u8) -> u8 {
        match r {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.read(self.hl()),
            7 => self.a,
            _ => 0,
        }
    }

    fn set_r(&mut self, r: u8, v: u8) {
        match r {
            0 => self.b = v,
            1 => self.c = v,
            2 => self.d = v,
            3 => self.e = v,
            4 => self.h = v,
            5 => self.l = v,
            6 => self.write(self.hl(), v),
            7 => self.a = v,
            _ => {}
        }
    }

    fn push16(&mut self, v: u16) {
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, (v >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, v as u8);
    }

    fn pop16(&mut self) -> u16 {
        let lo = u16::from(self.read(self.sp));
        self.sp = self.sp.wrapping_add(1);
        let hi = u16::from(self.read(self.sp));
        self.sp = self.sp.wrapping_add(1);
        lo | (hi << 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ld_r_r() {
        let mut z = Z80Cpu::new();
        z.b = 0x12;
        z.load(0x0000, &[0x78]); // LD A,B
        z.pc = 0;
        z.step();
        assert_eq!(z.a, 0x12);
    }

    #[test]
    fn xor_b() {
        let mut z = Z80Cpu::new();
        z.a = 0xFF;
        z.b = 0x0F;
        z.load(0x0000, &[0xA8]); // XOR B
        z.pc = 0;
        z.step();
        assert_eq!(z.a, 0xF0);
    }

    #[test]
    fn jp_imm() {
        let mut z = Z80Cpu::new();
        z.load(0x0000, &[0xC3, 0x34, 0x12]);
        z.pc = 0;
        z.step();
        assert_eq!(z.pc, 0x1234);
    }

    #[test]
    fn call_ret() {
        let mut z = Z80Cpu::new();
        z.load(0x0100, &[0xCD, 0x00, 0x02]);
        z.load(0x0200, &[0x3E, 0x42, 0xC9]); // LD A,n; RET
        z.pc = 0x0100;
        z.step();
        assert_eq!(z.pc, 0x0200);
        z.step();
        assert_eq!(z.a, 0x42);
        z.step();
        assert_eq!(z.pc, 0x0103);
    }
}
