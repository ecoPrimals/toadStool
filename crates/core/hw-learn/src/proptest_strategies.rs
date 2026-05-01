// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`proptest`] strategies for [`crate::distiller::InitRecipe`] and related distiller types.

use std::path::PathBuf;

use proptest::collection::vec;
use proptest::prelude::*;

use crate::distiller::{
    DriverKind, Engine, GpuArch, InitRecipe, InitStep, RegFunction, Vendor, VerifyCheck,
};

fn arb_vendor() -> impl Strategy<Value = Vendor> {
    prop_oneof![Just(Vendor::Amd), Just(Vendor::Intel), Just(Vendor::Nvidia)]
}

fn arb_driver_kind() -> impl Strategy<Value = DriverKind> {
    prop_oneof![
        Just(DriverKind::Amdgpu),
        Just(DriverKind::Nouveau),
        Just(DriverKind::NvidiaDrm),
        Just(DriverKind::I915),
        Just(DriverKind::Xe),
        "[a-zA-Z0-9 _.-]{0,32}".prop_map(DriverKind::Custom),
    ]
}

fn arb_gpu_arch() -> impl Strategy<Value = GpuArch> {
    (
        arb_vendor(),
        "[a-zA-Z0-9._-]{1,16}",
        "[a-zA-Z0-9._-]{1,16}",
        "[a-z0-9._-]{1,16}",
    )
        .prop_map(|(vendor, generation, chip, compute_class)| GpuArch {
            vendor,
            generation,
            chip,
            compute_class,
        })
}

fn arb_reg_function() -> impl Strategy<Value = RegFunction> {
    prop_oneof![
        Just(RegFunction::ClockEnable),
        Just(RegFunction::PowerGate),
        Just(RegFunction::EngineReset),
        Just(RegFunction::ContextAlloc),
        Just(RegFunction::ChannelBind),
        Just(RegFunction::InterruptEnable),
        Just(RegFunction::ThermalConfig),
        Just(RegFunction::MemoryConfig),
        Just(RegFunction::Unknown),
    ]
}

fn arb_engine() -> impl Strategy<Value = Engine> {
    prop_oneof![
        Just(Engine::Pmu),
        Just(Engine::Gsp),
        Just(Engine::Acr),
        Just(Engine::Gr),
        Just(Engine::Ce),
        Just(Engine::Sec2),
        Just(Engine::GuC),
        Just(Engine::HuC),
        "[a-zA-Z0-9_-]{1,16}".prop_map(Engine::Custom),
    ]
}

fn arb_verify_check() -> impl Strategy<Value = VerifyCheck> {
    prop_oneof![
        (any::<u64>(), any::<u64>(), any::<u64>()).prop_map(|(offset, expected, mask)| {
            VerifyCheck::RegisterMatch {
                offset,
                expected,
                mask,
            }
        }),
        any::<u64>().prop_map(|ioctl_nr| VerifyCheck::IoctlSucceeds { ioctl_nr }),
        Just(VerifyCheck::ComputeReadback),
        ("[A-Za-z0-9]{1,12}", any::<u64>(), any::<u64>()).prop_map(
            |(aperture, offset, sentinel)| {
                VerifyCheck::MemoryAccessible {
                    aperture,
                    offset,
                    sentinel,
                }
            }
        ),
    ]
}

fn arb_init_step() -> impl Strategy<Value = InitStep> {
    prop_oneof![
        (any::<u64>(), any::<u64>(), arb_reg_function()).prop_map(|(offset, value, function)| {
            InitStep::RegisterWrite {
                offset,
                value,
                function,
            }
        }),
        (any::<u64>(), vec(any::<u8>(), 0..64))
            .prop_map(|(ioctl_nr, args)| InitStep::IoctlCall { ioctl_nr, args }),
        (arb_engine(), "[a-zA-Z0-9./_-]{0,64}").prop_map(|(engine, p)| InitStep::FirmwareLoad {
            engine,
            path: PathBuf::from(p),
        }),
        (0u64..50_000u64).prop_map(|us| InitStep::Delay { us }),
        arb_verify_check().prop_map(|check| InitStep::Verify { check }),
    ]
}

/// Random [`InitRecipe`] with bounded step count for fast shrinking.
pub fn arb_init_recipe() -> impl Strategy<Value = InitRecipe> {
    (
        arb_gpu_arch(),
        arb_driver_kind(),
        arb_gpu_arch(),
        vec(arb_init_step(), 0..16usize),
        0.0f64..1.0f64,
        "[a-zA-Z0-9 _.,;:\\-]{0,120}",
    )
        .prop_map(
            |(source_arch, source_driver, target_arch, steps, confidence, description)| {
                InitRecipe {
                    source_arch,
                    source_driver,
                    target_arch,
                    steps,
                    confidence,
                    description,
                }
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::applicator::{ApplyVerdict, RecipeApplicator};
    use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

    proptest! {
        #[test]
        fn init_recipe_serde_roundtrip(recipe in arb_init_recipe()) {
            let json = serde_json::to_string(&recipe).unwrap();
            let back: InitRecipe = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(recipe.source_arch, back.source_arch);
            prop_assert_eq!(recipe.target_arch, back.target_arch);
            prop_assert_eq!(recipe.description, back.description);
            prop_assert!((recipe.confidence - back.confidence).abs() < f64::EPSILON * 1024.0);
            prop_assert_eq!(
                serde_json::to_string(&recipe.steps).unwrap(),
                serde_json::to_string(&back.steps).unwrap()
            );
            prop_assert_eq!(format!("{:?}", recipe.source_driver), format!("{:?}", back.source_driver));
        }

        #[test]
        fn dry_run_apply_matches_after_json_roundtrip(recipe in arb_init_recipe()) {
            let json = serde_json::to_string(&recipe).unwrap();
            let recipe2: InitRecipe = serde_json::from_str(&json).unwrap();
            let mut a1 = RecipeApplicator::new(true);
            let mut a2 = RecipeApplicator::new(true);
            let r1 = a1.apply(&recipe, "/dev/dri/card0");
            let r2 = a2.apply(&recipe2, "/dev/dri/card0");
            assert_eq!(r1.verdict, r2.verdict);
            assert_eq!(r1.steps_executed, r2.steps_executed);
            assert_eq!(r1.verdict, ApplyVerdict::Success);
        }
    }
}
