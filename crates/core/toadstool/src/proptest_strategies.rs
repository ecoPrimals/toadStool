// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`proptest`] strategies for [`crate::WorkloadType`] and [`crate::ResourceRequirements`].
//!
//! For [`hw_learn::distiller::InitRecipe`], see [`hw_learn::proptest_strategies`].

use proptest::prelude::*;

use crate::{GpuRequirements, ResourceRequirements, WorkloadType};

/// Random [`WorkloadType`] variant.
pub fn arb_workload_type() -> impl Strategy<Value = WorkloadType> {
    prop_oneof![
        Just(WorkloadType::Native),
        Just(WorkloadType::Wasm),
        Just(WorkloadType::Container),
        Just(WorkloadType::Gpu),
        Just(WorkloadType::Python),
        Just(WorkloadType::AiMl),
        Just(WorkloadType::Cuda),
    ]
}

/// [`ResourceRequirements`] within validator-friendly ranges (`validate()`-able).
pub fn arb_resource_requirements() -> impl Strategy<Value = ResourceRequirements> {
    (
        0.5f64..256.0f64,
        prop::option::of(1.0f64..512.0f64),
        prop::option::of("[a-z0-9_-]{0,16}"),
        4096u64..(1u64 << 30),
        prop::option::of(8192u64..(1u64 << 31)),
        1024u64 * 1024..(1u64 << 34),
        prop::option::of(1024u64 * 1024 * 512..(1u64 << 35)),
        prop::option::of("[a-z]{2,12}"),
        prop::option::of((
            1u32..16u32,
            prop::option::of(16u32..64u32),
            prop::option::of("[A-Za-z0-9 ]{0,24}"),
            prop::option::of(256u64..(1u64 << 29)),
        )),
        prop::option::of(1_000u64..1_000_000_000u64),
        prop::option::of(1_000u64..1_000_000_000u64),
        prop::option::of(1u64..2_000u64),
    )
        .prop_map(
            |(
                min_cores,
                max_cores,
                arch,
                min_mem,
                max_mem,
                min_st,
                max_st,
                storage_type,
                gpu,
                min_bw,
                max_bw,
                max_lat,
            )| {
                let mut r = ResourceRequirements::default();
                r.cpu.min_cores = min_cores;
                r.cpu.max_cores = max_cores;
                r.cpu.architecture = arch.map(String::from);
                r.memory.min_bytes = min_mem;
                r.memory.max_bytes = max_mem;
                r.storage.min_bytes = min_st;
                r.storage.max_bytes = max_st;
                r.storage.storage_type = storage_type.map(String::from);
                r.gpu =
                    gpu.map(
                        |(min_units, max_units, gpu_type, min_memory_bytes)| GpuRequirements {
                            min_units,
                            max_units,
                            gpu_type: gpu_type.map(String::from),
                            min_memory_bytes,
                        },
                    );
                r.network.min_bandwidth = min_bw;
                r.network.max_bandwidth = max_bw;
                r.network.max_latency_ms = max_lat;
                r
            },
        )
}
