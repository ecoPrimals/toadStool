// SPDX-License-Identifier: AGPL-3.0-only
//! Build init recipes from classified events.

use super::classify::ClassifiedEvent;
use super::{DriverKind, GpuArch, InitRecipe, InitStep, RegFunction, VerifyCheck};
use crate::observer::TraceEventKind;

/// Build a recipe from classified events.
///
/// Ordering: power → clock → memory → engine reset → context → channel → verify.
pub fn build_recipe(
    events: Vec<ClassifiedEvent>,
    target_arch: GpuArch,
    driver: &str,
) -> InitRecipe {
    let mut steps = Vec::new();

    let priority = |f: &RegFunction| -> u8 {
        match f {
            RegFunction::PowerGate => 0,
            RegFunction::ClockEnable => 1,
            RegFunction::ThermalConfig => 2,
            RegFunction::MemoryConfig => 3,
            RegFunction::EngineReset => 4,
            RegFunction::ContextAlloc => 5,
            RegFunction::ChannelBind => 6,
            RegFunction::InterruptEnable => 7,
            RegFunction::Unknown => 8,
        }
    };

    let mut classified = events;
    classified.sort_by_key(|e| priority(&e.function));

    for ce in &classified {
        match &ce.event.kind {
            TraceEventKind::RegisterWrite { offset, value, .. } => {
                steps.push(InitStep::RegisterWrite {
                    offset: *offset,
                    value: *value,
                    function: ce.function,
                });
            }
            TraceEventKind::IoctlCall { ioctl_nr, .. } => {
                steps.push(InitStep::IoctlCall {
                    ioctl_nr: *ioctl_nr,
                    args: Vec::new(),
                });
            }
            TraceEventKind::FirmwareLoad { engine, path } => {
                steps.push(InitStep::FirmwareLoad {
                    engine: parse_engine(engine),
                    path: path.into(),
                });
            }
            TraceEventKind::Gap { duration_us } if *duration_us > 100 => {
                steps.push(InitStep::Delay { us: *duration_us });
            }
            _ => {}
        }
    }

    steps.push(InitStep::Verify {
        check: VerifyCheck::ComputeReadback,
    });

    let source_driver = infer_driver_kind(driver);

    InitRecipe {
        source_arch: target_arch.clone(),
        source_driver,
        target_arch,
        steps,
        confidence: 0.0,
        description: format!(
            "Auto-distilled from {} trace ({} events)",
            driver,
            classified.len()
        ),
    }
}

fn parse_engine(name: &str) -> super::Engine {
    match name.to_lowercase().as_str() {
        "pmu" => super::Engine::Pmu,
        "gsp" => super::Engine::Gsp,
        "acr" => super::Engine::Acr,
        "gr" => super::Engine::Gr,
        "ce" => super::Engine::Ce,
        "sec2" => super::Engine::Sec2,
        "guc" => super::Engine::GuC,
        "huc" => super::Engine::HuC,
        other => super::Engine::Custom(other.to_string()),
    }
}

fn infer_driver_kind(driver: &str) -> DriverKind {
    let d = driver.to_lowercase();
    if d.contains("amdgpu") || d.contains("pm4") {
        DriverKind::Amdgpu
    } else if d.contains("nouveau") || d.contains("gsp") {
        DriverKind::Nouveau
    } else if d.contains("nvidia") {
        DriverKind::NvidiaDrm
    } else if d.contains("xe") {
        DriverKind::Xe
    } else if d.contains("i915") || d.contains("batch") {
        DriverKind::I915
    } else {
        DriverKind::Custom(driver.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_amdgpu() {
        assert_eq!(infer_driver_kind("amdgpu-pm4"), DriverKind::Amdgpu);
    }

    #[test]
    fn infer_nouveau() {
        assert_eq!(infer_driver_kind("nouveau-gsp"), DriverKind::Nouveau);
    }

    #[test]
    fn infer_intel() {
        assert_eq!(infer_driver_kind("i915-batch"), DriverKind::I915);
    }
}
