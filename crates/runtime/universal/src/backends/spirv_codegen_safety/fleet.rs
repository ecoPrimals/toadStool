// SPDX-License-Identifier: AGPL-3.0-only

use super::PrecisionBrain;
use hw_learn::brain_ext::learning_advisor::{FleetGpu, LearningAdvisor, LearningOpportunity};
use hw_learn::distiller::{GpuArch, Vendor};
use toadstool_sysmon::{FirmwareInventory, GpuDevice};

pub struct FleetMember {
    pub device: GpuDevice,
    pub brain: PrecisionBrain,
    pub firmware: FirmwareInventory,
}

pub fn learning_opportunities(fleet: &[FleetMember]) -> Vec<LearningOpportunity> {
    let fleet_gpus: Vec<FleetGpu> = fleet
        .iter()
        .map(|m| {
            let vendor = match m.device.vendor {
                toadstool_sysmon::GpuVendor::Amd => Vendor::Amd,
                toadstool_sysmon::GpuVendor::Intel => Vendor::Intel,
                toadstool_sysmon::GpuVendor::Nvidia => Vendor::Nvidia,
                toadstool_sysmon::GpuVendor::Unknown => Vendor::Nvidia,
            };

            let compute_works = m.firmware.compute_viable && m.brain.calibration().has_any_f64;

            FleetGpu {
                id: format!("card{}", m.device.card_index),
                arch: GpuArch {
                    vendor,
                    generation: infer_generation(&m.brain.calibration().adapter_name),
                    chip: format!("dev{:04x}", m.device.device_id),
                    compute_class: infer_compute_class(&m.brain.calibration().adapter_name),
                },
                firmware: m.firmware.clone(),
                compute_works,
                driver: m.device.driver.clone(),
            }
        })
        .collect();

    let advisor = LearningAdvisor::new(fleet_gpus);
    advisor.opportunities()
}

fn infer_generation(adapter_name: &str) -> String {
    let name = adapter_name.to_uppercase();
    if name.contains("RTX 40") || name.contains("AD1") {
        "Ada".into()
    } else if name.contains("RTX 30") || name.contains("GA1") {
        "Ampere".into()
    } else if name.contains("RTX 20") || name.contains("TU1") {
        "Turing".into()
    } else if name.contains("TITAN V") || name.contains("GV1") {
        "Volta".into()
    } else if name.contains("RX 6") || name.contains("NAVI") {
        "RDNA2".into()
    } else if name.contains("RX 7") {
        "RDNA3".into()
    } else if name.contains("ARC") || name.contains("DG2") {
        "Alchemist".into()
    } else {
        "Unknown".into()
    }
}

fn infer_compute_class(adapter_name: &str) -> String {
    let name = adapter_name.to_uppercase();
    if name.contains("RTX 40") {
        "sm89".into()
    } else if name.contains("RTX 30") {
        "sm86".into()
    } else if name.contains("RTX 20") {
        "sm75".into()
    } else if name.contains("TITAN V") {
        "sm70".into()
    } else if name.contains("RX 6") {
        "gfx1030".into()
    } else if name.contains("RX 7") {
        "gfx1100".into()
    } else if name.contains("ARC") {
        "gen12".into()
    } else {
        "unknown".into()
    }
}
