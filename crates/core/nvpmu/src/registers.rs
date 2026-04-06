// SPDX-License-Identifier: AGPL-3.0-or-later
//! GV100 (Volta) BAR0 register offsets and known values.
//!
//! Sourced from hotSpring VFIO experiments (Exp 058-060) and nouveau
//! kernel driver. These constants define the register map needed for
//! sovereign power management on desktop Volta GPUs (Titan V) which
//! have no PMU firmware — all power control is BAR0 register writes.
//!
//! # References
//!
//! - hotSpring `experiments/059_GPU_POWER_MANAGEMENT_DESIGN.md`
//! - hotSpring `experiments/060_BAR2_SELF_WARM_GLOW_PLUG.md`
//! - nouveau `nvkm/subdev/mc/gk104.c`, `nvkm/subdev/pmu/gm200.c`
//! - nouveau `nvkm/engine/fifo/gv100.c`

// -- PMC (Power Management Controller) ----------------------------------------

/// Master engine enable register. Writing `0xFFFF_FFFF` enables all engine
/// clock domains; hardware masks to supported engines. Readback reveals
/// which engines are actually clocked.
pub const PMC_ENABLE: u64 = 0x0000_0200;

/// Per-device enable register (finer granularity than `PMC_ENABLE`).
pub const PMC_DEVICE_ENABLE: u64 = 0x0000_0204;

/// `PMC_ENABLE` value when all engine clocks are active (readback after
/// writing `ENABLE_ALL` on GV100). Indicates Warm or Sovereign state.
pub const PMC_ENABLE_WARM: u32 = 0x5FEC_DFF1;

/// `PMC_ENABLE` value when engine clocks are gated (GPU internally
/// suspended). Only PMC + PTIMER remain active. Indicates Glow state.
pub const PMC_ENABLE_GATED: u32 = 0x4000_0020;

/// Value to write to `PMC_ENABLE` to request all engines enabled.
/// Hardware masks to supported engines on readback.
pub const PMC_ENABLE_ALL: u32 = 0xFFFF_FFFF;

// -- PFIFO (Scheduler / DMA Engine) -------------------------------------------

/// PFIFO master enable register. Toggle 0→1 to initialize PFIFO after
/// engine clock warm-up.
pub const PFIFO_ENABLE: u64 = 0x0000_2200;

/// Sentinel value read from PFIFO registers when the PFIFO engine is
/// clock-gated. Any register in the PFIFO range returns this.
pub const PFIFO_GATED_SENTINEL: u32 = 0xBAD0_DA00;

// -- PBUS (Bus Interface) ----------------------------------------------------

/// Bus-level clock gating control register. Nouveau leaves this at zero
/// on desktop Volta — untapped power savings headroom.
///
/// Bit layout (GV100):
/// - Bits  \[3:0\]: `IDLE_CG_DLY_CNT` — idle cycles before gating
/// - Bit      6 : `IDLE_CG_EN` — enable idle clock gating
/// - Bit     14 : `STALL_CG_EN` — gate during stalls
/// - Bits \[19:16\]: `WAKEUP_DLY_CNT` — wake latency in cycles
pub const PBUS_EXT_CG: u64 = 0x0000_1C00;

/// Sub-unit level clock gating (SLCG). Hardware default is `0x0000_03FE`
/// (all 9 sub-units enabled). Persists across power states.
pub const PBUS_EXT_CG1: u64 = 0x0000_1C04;

/// SLCG hardware default: all 9 sub-units enabled.
pub const PBUS_EXT_CG1_DEFAULT: u32 = 0x0000_03FE;

// -- PBUS_EXT_CG bitfield masks -----------------------------------------------

/// Bits \[3:0\]: idle cycles before clock gating activates.
pub const CG_IDLE_DLY_MASK: u32 = 0x0000_000F;

/// Bit 6: enable idle clock gating on the bus.
pub const CG_IDLE_EN: u32 = 1 << 6;

/// Bit 14: enable stall-based clock gating.
pub const CG_STALL_EN: u32 = 1 << 14;

/// Bits \[19:16\]: wake-up delay count after clock gate release.
pub const CG_WAKEUP_DLY_MASK: u32 = 0x000F_0000;
/// Bit shift to extract wake-up delay from `PBUS_EXT_CG`.
pub const CG_WAKEUP_DLY_SHIFT: u32 = 16;

// -- Thermal -------------------------------------------------------------------

/// GPU die temperature register. Bits \[15:8\] contain temperature in
/// degrees Celsius. Oracle reads ~46 C warm-idle, ~38 C cold.
pub const GPU_TEMP: u64 = 0x0002_0460;

/// Mask to extract temperature from `GPU_TEMP` register.
pub const GPU_TEMP_MASK: u32 = 0x0000_FF00;
/// Bit shift to extract temperature (bits \[15:8\]) from `GPU_TEMP`.
pub const GPU_TEMP_SHIFT: u32 = 8;

// -- PCIe Power Management ----------------------------------------------------

/// BAR0 reads `0xFFFF_FFFF` from every register when the GPU is in
/// `PCIe` D3hot sleep. Used by `current_state()` to detect Sleep state.
pub const BAR0_D3HOT_SENTINEL: u32 = 0xFFFF_FFFF;

// -- BOOT0 (Chip Identification) ----------------------------------------------

/// Chip identification register. Readable in D0; returns `BAR0_D3HOT_SENTINEL`
/// in D3hot. Used to verify BAR0 accessibility.
pub const BOOT0: u64 = 0x0000_0000;

/// BOOT0 value for GV100 (Titan V).
pub const BOOT0_GV100: u32 = 0x1400_00A1;

// -- Framebuffer / Memory Controller ------------------------------------------
// Offsets from nouveau `ramgv100.c` / `gf100_fb_oneinit()`.
// These are for future HBM2 init work — read-only probing only for now.

/// Base offset for the framebuffer/memory controller register block.
pub const FB_BASE: u64 = 0x0010_0000;

/// Memory controller configuration registers (`0x009A_0000`+ range).
/// Used by nouveau for HBM2 link training and DRAM init.
pub const FBPA_BASE: u64 = 0x009A_0000;

/// Number of FBPA units on GV100 (4 HBM2 stacks, 4 FBPA).
pub const GV100_FBPA_COUNT: u32 = 4;

/// Per-FBPA stride in the register space.
pub const FBPA_STRIDE: u64 = 0x4000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pmc_enable_values_are_distinct() {
        assert_ne!(PMC_ENABLE_WARM, PMC_ENABLE_GATED);
        assert_ne!(PMC_ENABLE_ALL, PMC_ENABLE_WARM);
        assert_ne!(PMC_ENABLE_ALL, PMC_ENABLE_GATED);
    }

    #[test]
    fn cg_bitfield_no_overlap() {
        assert_eq!(CG_IDLE_EN & CG_STALL_EN, 0);
        assert_eq!(CG_IDLE_EN & CG_IDLE_DLY_MASK, 0);
        assert_eq!(CG_STALL_EN & CG_WAKEUP_DLY_MASK, 0);
    }

    #[test]
    fn gpu_temp_extraction() {
        let raw = 0x0000_2E00_u32; // 0x2E = 46 decimal = 46 C
        let temp = (raw & GPU_TEMP_MASK) >> GPU_TEMP_SHIFT;
        assert_eq!(temp, 46);
    }

    #[test]
    fn boot0_gv100_sanity() {
        assert_ne!(BOOT0_GV100, BAR0_D3HOT_SENTINEL);
    }
}
