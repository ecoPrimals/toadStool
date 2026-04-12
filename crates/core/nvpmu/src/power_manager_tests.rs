// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use hw_learn::applicator::RegisterAccess;

struct MockRegs {
    values: std::collections::HashMap<u64, u32>,
}

impl MockRegs {
    fn new() -> Self {
        Self {
            values: std::collections::HashMap::new(),
        }
    }

    fn with_warm_state() -> Self {
        let mut m = Self::new();
        m.values
            .insert(registers::PMC_ENABLE, registers::PMC_ENABLE_WARM);
        m.values.insert(registers::PFIFO_ENABLE, 0x0000_0000);
        m.values.insert(registers::GPU_TEMP, 0x0000_2E00); // 46 C
        m.values.insert(registers::PBUS_EXT_CG, 0x0000_0000);
        m
    }

    fn with_glow_state() -> Self {
        let mut m = Self::new();
        m.values
            .insert(registers::PMC_ENABLE, registers::PMC_ENABLE_GATED);
        m.values
            .insert(registers::PFIFO_ENABLE, registers::PFIFO_GATED_SENTINEL);
        m.values.insert(registers::GPU_TEMP, 0x0000_2600); // 38 C
        m.values.insert(registers::PBUS_EXT_CG, 0x0000_0000);
        m
    }
}

impl RegisterAccess for MockRegs {
    fn read_u32(&self, offset: u64) -> std::result::Result<u32, String> {
        self.values
            .get(&offset)
            .copied()
            .ok_or_else(|| format!("unmapped register {offset:#x}"))
    }

    fn write_u32(&mut self, offset: u64, value: u32) -> std::result::Result<(), String> {
        self.values.insert(offset, value);
        Ok(())
    }
}

#[test]
fn clock_gate_config_roundtrip() {
    let cfg = ClockGateConfig {
        idle_delay: 5,
        idle_cg_en: true,
        stall_cg_en: true,
        wakeup_delay: 3,
    };
    let encoded = cfg.encode();
    let decoded = ClockGateConfig::decode(encoded);
    assert_eq!(cfg, decoded);
}

#[test]
fn clock_gate_config_default_is_zero() {
    assert_eq!(ClockGateConfig::default().encode(), 0);
}

#[test]
fn gpu_temp_read_warm() {
    let regs = MockRegs::with_warm_state();
    let pm = PowerManager::new(regs, "ffff:ff:ff.f");
    let temp = pm.read_gpu_temp_c();
    assert!(temp.is_ok());
    assert!((temp.unwrap() - 46.0).abs() < 0.01);
}

#[test]
fn gpu_temp_read_glow() {
    let regs = MockRegs::with_glow_state();
    let pm = PowerManager::new(regs, "ffff:ff:ff.f");
    let temp = pm.read_gpu_temp_c();
    assert!(temp.is_ok());
    assert!((temp.unwrap() - 38.0).abs() < 0.01);
}

#[test]
fn clock_gate_encode_individual_bits() {
    let idle_only = ClockGateConfig {
        idle_delay: 0,
        idle_cg_en: true,
        stall_cg_en: false,
        wakeup_delay: 0,
    };
    assert_eq!(idle_only.encode(), registers::CG_IDLE_EN);

    let stall_only = ClockGateConfig {
        idle_delay: 0,
        idle_cg_en: false,
        stall_cg_en: true,
        wakeup_delay: 0,
    };
    assert_eq!(stall_only.encode(), registers::CG_STALL_EN);
}

#[test]
fn power_state_display() {
    assert_eq!(GpuPowerState::Sovereign.to_string(), "Sovereign");
    assert_eq!(GpuPowerState::Warm.to_string(), "Warm");
    assert_eq!(GpuPowerState::Glow.to_string(), "Glow");
    assert_eq!(GpuPowerState::Sleep.to_string(), "Sleep");
    assert_eq!(GpuPowerState::Off.to_string(), "Off");
}
