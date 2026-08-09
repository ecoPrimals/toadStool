// SPDX-License-Identifier: AGPL-3.0-or-later
//! PCI device ID to silicon capability lookup tables.
//!
//! Maps known NVIDIA, AMD, and Intel GPU device IDs to their hardware
//! capabilities (tensor cores, RT cores, TMU/ROP counts). Data sourced
//! from published vendor specifications.

use toadstool_core::silicon::{RtCoreGen, SiliconCapabilities, SiliconUnit, TensorCoreGen};

/// NVIDIA silicon capabilities by PCI device ID.
///
/// Covers Volta, Turing, Ampere, Ada families. TMU and ROP counts
/// come from published GPU specs (not estimated from name strings).
pub fn nvidia_silicon(device_id: u32) -> SiliconCapabilities {
    let (tensor_gen, rt_gen, tmu, rop) = match device_id {
        // Volta (GV100): Titan V, Tesla V100
        0x1D81 | 0x1DB1 | 0x1DB4..=0x1DBA => (Some(TensorCoreGen::Volta), None, 320_u32, 96_u32),
        // Turing (TU102): RTX 2080 Ti, Titan RTX, Quadro RTX 8000/6000
        0x1E02..=0x1E3F | 0x1E82..=0x1EBF => (
            Some(TensorCoreGen::Turing),
            Some(RtCoreGen::Turing),
            288,
            96,
        ),
        // Turing (TU104): RTX 2080/2080 Super, Quadro RTX 5000
        0x1E04..=0x1E7F | 0x1F02..=0x1F3F => (
            Some(TensorCoreGen::Turing),
            Some(RtCoreGen::Turing),
            192,
            64,
        ),
        // Turing (TU106): RTX 2070/2060
        0x1E84..=0x1EFF | 0x1F82..=0x1FBF => (
            Some(TensorCoreGen::Turing),
            Some(RtCoreGen::Turing),
            120,
            64,
        ),
        // Turing (TU116/TU117): GTX 1660/1650 — no tensor/RT cores
        0x2182..=0x21FF => (None, None, 96, 48),
        // Ampere (GA102): RTX 3090/3080 Ti/3080
        0x2204..=0x223F => (
            Some(TensorCoreGen::Ampere),
            Some(RtCoreGen::Ampere),
            328,
            112,
        ),
        // Ampere (GA104): RTX 3070/3060 Ti
        0x2484..=0x24BF => (
            Some(TensorCoreGen::Ampere),
            Some(RtCoreGen::Ampere),
            192,
            96,
        ),
        // Ampere (GA106): RTX 3060
        0x2504..=0x253F => (
            Some(TensorCoreGen::Ampere),
            Some(RtCoreGen::Ampere),
            112,
            48,
        ),
        // A100 (GA100)
        0x20B0..=0x20BF | 0x20F1..=0x20FF => (Some(TensorCoreGen::Ampere), None, 432, 160),
        // Ada Lovelace (AD102): RTX 4090/4080
        0x2684..=0x26BF => (Some(TensorCoreGen::Ada), Some(RtCoreGen::Ada), 512, 176),
        // Ada Lovelace (AD104): RTX 4070 Ti/4070
        0x2704..=0x273F => (Some(TensorCoreGen::Ada), Some(RtCoreGen::Ada), 240, 80),
        // Ada Lovelace (AD106): RTX 4060 Ti/4060
        0x2784..=0x27BF => (Some(TensorCoreGen::Ada), Some(RtCoreGen::Ada), 136, 48),
        // Ada Lovelace (AD107): RTX 4060 mobile / 4050
        0x2804..=0x283F => (Some(TensorCoreGen::Ada), Some(RtCoreGen::Ada), 96, 32),
        // Unknown NVIDIA — conservative baseline
        _ => (None, None, 128, 64),
    };

    let has_tensor = tensor_gen.is_some();
    let has_rt = rt_gen.is_some();

    let mut units = vec![
        SiliconUnit::ShaderCore,
        SiliconUnit::TextureUnit,
        SiliconUnit::Rop,
        SiliconUnit::Rasterizer,
        SiliconUnit::DepthBuffer,
        SiliconUnit::Tessellator,
    ];
    if has_tensor {
        units.push(SiliconUnit::TensorCore);
    }
    if has_rt {
        units.push(SiliconUnit::RtCore);
    }
    units.push(SiliconUnit::VideoEncoder);

    SiliconCapabilities {
        tensor_cores: tensor_gen,
        rt_cores: rt_gen,
        has_video_encoder: true,
        estimated_tmu_count: tmu,
        estimated_rop_count: rop,
        rasterizer_available: true,
        tessellator_available: true,
        available_units: units,
        compiler_backends: Vec::new(),
    }
}

/// AMD silicon capabilities by device ID.
pub fn amd_silicon(device_id: u32) -> SiliconCapabilities {
    let (rt_gen, tmu, rop) = match device_id {
        // RDNA 3: Navi 31 (RX 7900 XTX/XT)
        0x744C | 0x7448 => (Some(RtCoreGen::Ampere), 384, 192),
        // RDNA 3: Navi 32 (RX 7800 XT/7700 XT)
        0x7480..=0x749F => (Some(RtCoreGen::Ampere), 240, 96),
        // RDNA 3: Navi 33 (RX 7600)
        0x7400..=0x743F => (Some(RtCoreGen::Ampere), 128, 64),
        // RDNA 2: Navi 21 (RX 6950 XT/6900 XT/6800 XT)
        0x73BF | 0x73A5 | 0x73AF => (Some(RtCoreGen::Turing), 320, 128),
        // RDNA 2: Navi 22 (RX 6700 XT)
        0x73DF | 0x73FF => (Some(RtCoreGen::Turing), 160, 64),
        // CDNA: MI50/MI60 (no RT, no rasterizer in compute mode)
        0x66A0..=0x66AF => (None, 256, 64),
        // Unknown AMD
        _ => (None, 128, 64),
    };

    let mut units = vec![
        SiliconUnit::ShaderCore,
        SiliconUnit::TextureUnit,
        SiliconUnit::Rop,
        SiliconUnit::Rasterizer,
        SiliconUnit::DepthBuffer,
        SiliconUnit::Tessellator,
    ];
    if rt_gen.is_some() {
        units.push(SiliconUnit::RtCore);
    }
    units.push(SiliconUnit::VideoEncoder);

    SiliconCapabilities {
        tensor_cores: None,
        rt_cores: rt_gen,
        has_video_encoder: true,
        estimated_tmu_count: tmu,
        estimated_rop_count: rop,
        rasterizer_available: true,
        tessellator_available: true,
        available_units: units,
        compiler_backends: Vec::new(),
    }
}

/// Intel GPU silicon — conservative baseline (no tensor/RT).
pub fn intel_silicon() -> SiliconCapabilities {
    SiliconCapabilities {
        tensor_cores: None,
        rt_cores: None,
        has_video_encoder: true,
        estimated_tmu_count: 64,
        estimated_rop_count: 32,
        rasterizer_available: true,
        tessellator_available: true,
        available_units: vec![
            SiliconUnit::ShaderCore,
            SiliconUnit::TextureUnit,
            SiliconUnit::Rop,
            SiliconUnit::Rasterizer,
            SiliconUnit::DepthBuffer,
            SiliconUnit::Tessellator,
            SiliconUnit::VideoEncoder,
        ],
        compiler_backends: Vec::new(),
    }
}
