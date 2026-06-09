// SPDX-License-Identifier: AGPL-3.0-or-later
//! Clock gating and power gating control registers.
//!
//! Volta GPUs have three levels of clock gating:
//! - ELCG (Engine-Level): Controlled per-engine, coarsest
//! - BLCG (Block-Level): Per-block within engines
//! - SLCG (Second-Level): Finest-grained, within blocks
//!
//! Disabling these gates is required to access domains that report
//! `0xBADF1100` (BLCG/SLCG gated) or `0xBADF3000` (hub clock gated).
//!
//! Register map derived from nouveau `nvkm/subdev/therm/` and open-gpu-doc.

/// PTHERM CG control — master clock gating override.
/// Writing 0x0 disables CG at the top level.
pub const PTHERM_GATE_CTRL: usize = 0x0002_0200;

/// PMC clock gating control registers.
/// Each engine domain has a CG register at PMC + 0x800 + engine*4.
pub const PMC_CG_BASE: usize = 0x0000_0800;

/// PRIV_RING CG control (hub level).
pub const PRIV_RING_CG: usize = 0x0012_0100;

/// FBPA per-partition CG controls.
/// Within each FBPA (base + stride*N), the CG register is at +0x0028.
pub const FBPA_CG_OFFSET: usize = 0x0028;

/// LTC cache-level CG controls.
/// Within each LTC (base + stride*N), the CG register is at +0x01C8.
pub const LTC_CG_OFFSET: usize = 0x01C8;

/// PFB memory controller CG.
pub const PFB_CG: usize = 0x0010_0C00;

/// PCLOCK/CLK engine CG.
pub const PCLOCK_CG: usize = 0x0013_7018;

/// Standard CG disable value: all gating disabled.
pub const CG_DISABLE: u32 = 0x0000_0000;

/// Standard CG enable value (nouveau default): auto gating.
pub const CG_AUTO: u32 = 0x0000_0500;

/// Volta FBPA base and stride.
pub const FBPA0_BASE: usize = 0x009A_0000;
pub const FBPA_STRIDE: usize = 0x0000_4000;
pub const FBPA_COUNT: usize = 4;

/// Volta LTC base and stride.
pub const LTC0_BASE: usize = 0x0017_E000;
pub const LTC_STRIDE: usize = 0x0000_2000;
pub const LTC_COUNT: usize = 6;

/// Known CG control registers to sweep for disabling clock gating.
/// Format: (register_offset, description).
pub const CG_SWEEP_TARGETS: &[(usize, &str)] = &[
    (PTHERM_GATE_CTRL, "PTHERM master gate"),
    (0x0002_0204, "PTHERM CG1"),
    (0x0002_0208, "PTHERM CG2"),
    (PRIV_RING_CG, "PRIV_RING CG"),
    (0x0012_0104, "PRIV_RING CG1"),
    (PFB_CG, "PFB CG"),
    (PCLOCK_CG, "PCLOCK CG"),
    // Engine PMC CG slots (0x800+engine*4)
    (0x0000_0800, "PMC CG slot 0"),
    (0x0000_0804, "PMC CG slot 1"),
    (0x0000_0808, "PMC CG slot 2"),
    (0x0000_080C, "PMC CG slot 3"),
];
