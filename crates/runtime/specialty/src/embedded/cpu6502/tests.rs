// SPDX-License-Identifier: AGPL-3.0-or-later

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
