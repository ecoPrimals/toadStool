// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from amd_metal.rs (S335).

use super::amd_metal::{BUSY_BIT_MASK, GRBM_STATUS, GRBM_STATUS2, SRBM_STATUS, VegaInit};
use crate::hardware::{BootPipeline, Vendor};
use crate::vfio::device::{ApplyError, RegisterAccess};

struct FakeBar {
    grbm: u32,
    grbm2: u32,
    srbm: u32,
}

impl RegisterAccess for FakeBar {
    fn read_u32(&self, offset: u32) -> Result<u32, ApplyError> {
        match offset as usize {
            GRBM_STATUS => Ok(self.grbm),
            GRBM_STATUS2 => Ok(self.grbm2),
            SRBM_STATUS => Ok(self.srbm),
            _ => Ok(0),
        }
    }

    fn write_u32(&mut self, _offset: u32, _value: u32) -> Result<(), ApplyError> {
        Ok(())
    }
}

#[test]
fn vega_probe_warm_detection() {
    let bar = FakeBar {
        grbm: 0x0000_3000,
        grbm2: 0x0000_0100,
        srbm: 0x0000_0000,
    };
    let vega = VegaInit::new();
    let probe = BootPipeline::probe(&vega, &bar).unwrap();
    assert!(vega.is_warm(&probe));
    assert_eq!(probe.grbm_status, 0x0000_3000);

    let summary = vega.probe_summary(&probe);
    assert_eq!(summary.vendor, Vendor::Amd);
    assert_eq!(summary.family, "Vega 20");
    assert!(summary.warm);
}

#[test]
fn vega_probe_cold_detection() {
    let bar = FakeBar {
        grbm: 0xFFFF_FFFF,
        grbm2: 0xFFFF_FFFF,
        srbm: 0xFFFF_FFFF,
    };
    let vega = VegaInit::new();
    let probe = BootPipeline::probe(&vega, &bar).unwrap();
    assert!(!vega.is_warm(&probe));
}

#[test]
fn vega_warm_devinit_succeeds() {
    let bar = FakeBar {
        grbm: 0x0000_3000,
        grbm2: 0,
        srbm: 0,
    };
    let vega = VegaInit::new();
    let probe = BootPipeline::probe(&vega, &bar).unwrap();
    let init = BootPipeline::devinit(&vega, &bar, &probe).unwrap();
    assert!(init.memory_alive);
    assert_eq!(init.method, "warm-skip");
}

#[test]
fn vega_cold_devinit_unsupported() {
    let bar = FakeBar {
        grbm: 0xFFFF_FFFF,
        grbm2: 0xFFFF_FFFF,
        srbm: 0xFFFF_FFFF,
    };
    let vega = VegaInit::new();
    let probe = BootPipeline::probe(&vega, &bar).unwrap();
    assert!(BootPipeline::devinit(&vega, &bar, &probe).is_err());
}

#[test]
fn vega_verify_idle() {
    let bar = FakeBar {
        grbm: 0x0000_3000,
        grbm2: 0,
        srbm: 0,
    };
    let vega = VegaInit::new();
    assert!(BootPipeline::verify(&vega, &bar).unwrap());
}

#[test]
fn vega_verify_busy() {
    let bar = FakeBar {
        grbm: BUSY_BIT_MASK | 0x100,
        grbm2: 0,
        srbm: 0,
    };
    let vega = VegaInit::new();
    assert!(!BootPipeline::verify(&vega, &bar).unwrap());
}

#[test]
fn vega_device_family() {
    let vega = VegaInit::new();
    assert_eq!(vega.device_family(), "Vega 20");
}

#[test]
fn vega_with_bdf() {
    let vega = VegaInit::with_bdf("0000:03:00.0");
    assert_eq!(vega.bdf.as_deref(), Some("0000:03:00.0"));
}
