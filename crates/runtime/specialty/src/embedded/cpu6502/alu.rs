// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{C, Cpu6502, N, V, Z};

impl Cpu6502 {
    pub(super) fn set_nz(&mut self, v: u8) {
        self.p &= !(N | Z);
        if v == 0 {
            self.p |= Z;
        }
        if v & 0x80 != 0 {
            self.p |= N;
        }
    }

    pub(super) fn cmp_val(&mut self, r: u8, m: u8) {
        let d = u16::from(r).wrapping_sub(u16::from(m));
        self.set_nz(d as u8);
        self.p &= !C;
        if r >= m {
            self.p |= C;
        }
    }

    pub(super) fn adc(&mut self, m: u8) {
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

    pub(super) fn sbc(&mut self, m: u8) {
        self.adc(!m);
    }
}
