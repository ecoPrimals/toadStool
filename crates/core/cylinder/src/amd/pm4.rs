// SPDX-License-Identifier: AGPL-3.0-or-later
//! PM4 command buffer construction for AMD RDNA2 compute dispatch.
//!
//! PM4 (Packet Manager 4) is the command packet format used by AMD GPUs.
//! Compute dispatch requires:
//! 1. `COMPUTE_PGM_LO`/`HI` — shader program base address
//! 2. `COMPUTE_PGM_RSRC1`/`2` — resource descriptors (VGPRs, SGPRs, etc.)
//! 3. `COMPUTE_NUM_THREAD_X`/`Y`/`Z` — workgroup size
//! 4. `DISPATCH_DIRECT` — launch the compute shader

use super::generation::{AmdGenerationProfile, CacheMethod};
use crate::{DispatchDims, ShaderInfo};

// PM4 packet types
const PM4_TYPE3: u32 = 3 << 30;

// PM4 opcodes for compute
const PM4_SET_SH_REG: u32 = 0x76;
const PM4_DISPATCH_DIRECT: u32 = 0x15;
const PM4_NOP: u32 = 0x10;
const PM4_ACQUIRE_MEM: u32 = 0x58;

// Compute shader register offsets (dword addresses, from SI_SH_REG_OFFSET)
const COMPUTE_START_X: u32 = 0x2E04;
const COMPUTE_NUM_THREAD_X: u32 = 0x2E07;
const COMPUTE_PERFCOUNT_ENABLE: u32 = 0x2E0B;
const COMPUTE_PGM_LO: u32 = 0x2E0C;
const COMPUTE_PGM_RSRC1: u32 = 0x2E12;
const COMPUTE_PGM_RSRC2: u32 = 0x2E13;
const COMPUTE_RESOURCE_LIMITS: u32 = 0x2E15;
const COMPUTE_STATIC_THREAD_MGMT_SE0: u32 = 0x2E16;
const COMPUTE_STATIC_THREAD_MGMT_SE1: u32 = 0x2E17;
const COMPUTE_TMPRING_SIZE: u32 = 0x2E18;
const COMPUTE_STATIC_THREAD_MGMT_SE2: u32 = 0x2E19;
const COMPUTE_STATIC_THREAD_MGMT_SE3: u32 = 0x2E1A;
const COMPUTE_USER_DATA_0: u32 = 0x2E40;

// SI shader register base for SET_SH_REG
const SI_SH_REG_BASE: u32 = 0x2C00;

/// Build a PM4 command stream for a compute dispatch (legacy interface).
///
/// Delegates to [`build_compute_dispatch_profiled`] via [`profile_for_gfx`](super::generation::profile_for_gfx).
#[must_use]
pub fn build_compute_dispatch(
    shader_va: u64,
    dims: DispatchDims,
    info: &ShaderInfo,
    buffer_vas: &[u64],
    gfx_major: u8,
) -> Vec<u32> {
    let profile = super::generation::profile_for_gfx(gfx_major);
    build_compute_dispatch_profiled(shader_va, dims, info, buffer_vas, profile)
}

/// Build a PM4 command stream for a compute dispatch.
///
/// `buffer_vas` contains the GPU virtual addresses of each bound buffer.
/// These are loaded into `COMPUTE_USER_DATA` registers so the shader can
/// read them from user SGPRs (2 SGPRs per 64-bit VA).
///
/// Uses the [`AmdGenerationProfile`] for register encoding differences
/// (WGP_MODE, MEM_ORDERED, cache method, VGPR granularity).
///
/// Uses compiler-derived `info` for workgroup size and register allocation.
/// Returns the PM4 words ready for submission via `DRM_AMDGPU_CS`.
#[must_use]
pub fn build_compute_dispatch_profiled(
    shader_va: u64,
    dims: DispatchDims,
    info: &ShaderInfo,
    buffer_vas: &[u64],
    profile: &AmdGenerationProfile,
) -> Vec<u32> {
    let mut pm4 = Vec::with_capacity(96);

    emit_set_sh_reg(&mut pm4, COMPUTE_PERFCOUNT_ENABLE, &[0]);

    let cu_en = 0xFFFF_FFFFu32;
    emit_set_sh_reg(&mut pm4, COMPUTE_STATIC_THREAD_MGMT_SE0, &[cu_en]);
    emit_set_sh_reg(&mut pm4, COMPUTE_STATIC_THREAD_MGMT_SE1, &[cu_en]);
    emit_set_sh_reg(&mut pm4, COMPUTE_STATIC_THREAD_MGMT_SE2, &[cu_en]);
    emit_set_sh_reg(&mut pm4, COMPUTE_STATIC_THREAD_MGMT_SE3, &[cu_en]);

    emit_set_sh_reg(&mut pm4, COMPUTE_START_X, &[0, 0, 0]);

    #[expect(
        clippy::cast_possible_truncation,
        reason = "ISA register field is 32-bit wide"
    )]
    let pgm_lo = (shader_va >> 8) as u32;
    let pgm_hi = (shader_va >> 40) as u32;
    emit_set_sh_reg(&mut pm4, COMPUTE_PGM_LO, &[pgm_lo, pgm_hi]);

    let vgpr_count = (info.gpr_count + 5).max(4);
    let sgpr_count = 16_u32;
    let rsrc1 = compute_pgm_rsrc1_profiled(vgpr_count, sgpr_count, profile);
    emit_set_sh_reg(&mut pm4, COMPUTE_PGM_RSRC1, &[rsrc1]);

    #[expect(
        clippy::cast_possible_truncation,
        reason = "buffer count limited to 5 (10 + 6 system = 16 user SGPRs max)"
    )]
    let user_sgpr_count = (buffer_vas.len() as u32) * 2 + 6;

    {
        let mut user_data = Vec::with_capacity(user_sgpr_count as usize);
        for &va in buffer_vas {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "splitting 64-bit VA into 32-bit halves"
            )]
            {
                user_data.push(va as u32);
                user_data.push((va >> 32) as u32);
            }
        }
        user_data.push(info.workgroup[0]);
        user_data.push(info.workgroup[1]);
        user_data.push(info.workgroup[2]);
        user_data.push(dims.x);
        user_data.push(dims.y);
        user_data.push(dims.z);
        emit_set_sh_reg(&mut pm4, COMPUTE_USER_DATA_0, &user_data);
    }

    let rsrc2 = compute_pgm_rsrc2(user_sgpr_count);
    emit_set_sh_reg(&mut pm4, COMPUTE_PGM_RSRC2, &[rsrc2]);

    let resource_limits = compute_resource_limits_profiled(info, profile);
    emit_set_sh_reg(&mut pm4, COMPUTE_RESOURCE_LIMITS, &[resource_limits]);

    emit_set_sh_reg(&mut pm4, COMPUTE_TMPRING_SIZE, &[0]);

    emit_set_sh_reg(&mut pm4, COMPUTE_NUM_THREAD_X, &info.workgroup);

    emit_cache_invalidate_profiled(&mut pm4, profile);

    emit_dispatch_direct(&mut pm4, dims, info.wave_size);

    emit_acquire_mem_profiled(&mut pm4, profile);

    emit_nop(&mut pm4);

    pm4
}

/// Emit a PM4 `SET_SH_REG` packet.
fn emit_set_sh_reg(pm4: &mut Vec<u32>, reg_offset: u32, values: &[u32]) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "register values list is always small"
    )]
    let count = values.len() as u32;
    let header = pm4_type3_header(PM4_SET_SH_REG, count + 1);
    pm4.push(header);
    pm4.push(reg_offset - SI_SH_REG_BASE);
    pm4.extend_from_slice(values);
}

/// Emit a PM4 `DISPATCH_DIRECT` packet.
fn emit_dispatch_direct(pm4: &mut Vec<u32>, dims: DispatchDims, wave_size: u32) {
    let header = pm4_type3_header(PM4_DISPATCH_DIRECT, 4);
    pm4.push(header);
    pm4.push(dims.x);
    pm4.push(dims.y);
    pm4.push(dims.z);
    // DISPATCH_INITIATOR: COMPUTE_SHADER_EN=1 | FORCE_START_AT_000=4 | ORDER_MODE=16
    // CS_W32_EN (bit 15) only for RDNA wave32; GCN5 wave64 must leave it clear.
    let mut initiator = 1 | 4 | 16;
    if wave_size <= 32 {
        initiator |= 1 << 15;
    }
    pm4.push(initiator);
}

/// Emit a PM4 `ACQUIRE_MEM` to invalidate caches before dispatch (profile-driven).
fn emit_cache_invalidate_profiled(pm4: &mut Vec<u32>, profile: &AmdGenerationProfile) {
    match profile.cache_method {
        CacheMethod::Gcr => {
            let header = pm4_type3_header(PM4_ACQUIRE_MEM, 7);
            pm4.push(header);
            pm4.push(0); // CP_COHER_CNTL (unused on GCR path)
            pm4.push(0xFFFF_FFFF); // COHER_SIZE
            pm4.push(0x0000_00FF); // COHER_SIZE_HI
            pm4.push(0); // COHER_BASE_LO
            pm4.push(0); // COHER_BASE_HI
            pm4.push(0); // reserved
            // GCR_CNTL: GL2_INV [14] | GL1_INV [9] | GLV_INV [8] | GLK_INV [7] | GLM_INV [5]
            pm4.push((1 << 14) | (1 << 9) | (1 << 8) | (1 << 7) | (1 << 5));
        }
        CacheMethod::CpCoher => {
            let header = pm4_type3_header(PM4_ACQUIRE_MEM, 6);
            pm4.push(header);
            // CP_COHER_CNTL: TC_ACTION_ENA [23] | TCL1_ACTION_ENA [25]
            pm4.push((1 << 23) | (1 << 25));
            pm4.push(0xFFFF_FFFF); // COHER_SIZE
            pm4.push(0x0000_00FF); // COHER_SIZE_HI
            pm4.push(0); // COHER_BASE_LO
            pm4.push(0); // COHER_BASE_HI
            pm4.push(10); // POLL_INTERVAL
        }
    }
}

/// Emit a PM4 `ACQUIRE_MEM` to flush the GPU L2 cache after dispatch (profile-driven).
fn emit_acquire_mem_profiled(pm4: &mut Vec<u32>, profile: &AmdGenerationProfile) {
    match profile.cache_method {
        CacheMethod::Gcr => {
            let header = pm4_type3_header(PM4_ACQUIRE_MEM, 7);
            pm4.push(header);
            pm4.push(0); // CP_COHER_CNTL (unused on GCR path)
            pm4.push(0xFFFF_FFFF); // COHER_SIZE
            pm4.push(0x0000_00FF); // COHER_SIZE_HI
            pm4.push(0); // COHER_BASE_LO
            pm4.push(0); // COHER_BASE_HI
            pm4.push(0); // reserved
            // GCR_CNTL: GL2_WB [15] | GL2_INV [14] | GL1_INV [9]
            pm4.push((1 << 15) | (1 << 14) | (1 << 9));
        }
        CacheMethod::CpCoher => {
            let header = pm4_type3_header(PM4_ACQUIRE_MEM, 6);
            pm4.push(header);
            // CP_COHER_CNTL: TC_WB_ACTION_ENA [18] | TC_ACTION_ENA [23]
            pm4.push((1 << 18) | (1 << 23));
            pm4.push(0xFFFF_FFFF); // COHER_SIZE
            pm4.push(0x0000_00FF); // COHER_SIZE_HI
            pm4.push(0); // COHER_BASE_LO
            pm4.push(0); // COHER_BASE_HI
            pm4.push(10); // POLL_INTERVAL
        }
    }
}

/// Emit a PM4 NOP packet (used for IB padding).
fn emit_nop(pm4: &mut Vec<u32>) {
    let header = pm4_type3_header(PM4_NOP, 1);
    pm4.push(header);
    pm4.push(0);
}

/// Build a PM4 Type 3 packet header.
///
/// Format: [31:30]=3 (type), [29:16]=count-1, [15:8]=opcode, [7:0]=reserved
const fn pm4_type3_header(opcode: u32, count: u32) -> u32 {
    PM4_TYPE3 | (((count - 1) & 0x3FFF) << 16) | ((opcode & 0xFF) << 8)
}

/// Build `COMPUTE_PGM_RSRC1` register value (legacy interface).
#[cfg(test)]
const fn compute_pgm_rsrc1(
    vgpr_count: u32,
    sgpr_count: u32,
    vgpr_granularity: u32,
    gfx_major: u8,
) -> u32 {
    let vgpr_encoded = (vgpr_count.div_ceil(vgpr_granularity)).saturating_sub(1);
    let sgpr_encoded = (sgpr_count.div_ceil(16)).saturating_sub(1);
    let float_mode = 0xC0_u32;
    let mut rsrc1 = vgpr_encoded | (sgpr_encoded << 6) | (float_mode << 12) | (1 << 21) | (1 << 23);
    if gfx_major >= 10 {
        rsrc1 |= 1 << 29;
        rsrc1 |= 1 << 30;
        rsrc1 |= 1 << 31;
    }
    rsrc1
}

/// Build `COMPUTE_PGM_RSRC1` register value (profile-driven).
///
/// Uses [`AmdGenerationProfile`] fields for VGPR granularity and RDNA mode bits
/// instead of branching on raw `gfx_major`.
fn compute_pgm_rsrc1_profiled(
    vgpr_count: u32,
    sgpr_count: u32,
    profile: &AmdGenerationProfile,
) -> u32 {
    let vgpr_encoded = (vgpr_count.div_ceil(profile.vgpr_granularity)).saturating_sub(1);
    let sgpr_encoded = (sgpr_count.div_ceil(16)).saturating_sub(1);
    // FLOAT_MODE [19:12] = 0xC0 (IEEE f64 denorms enabled, matches Mesa default)
    // DX10_CLAMP [21] = 1 (clamp NaN to 0, required by Mesa/RADV)
    // IEEE_MODE  [23] = 1 (IEEE compliance for f64)
    let float_mode = 0xC0_u32;
    let mut rsrc1 = vgpr_encoded
        | (sgpr_encoded << 6)
        | (float_mode << 12)
        | (1 << 21) // DX10_CLAMP
        | (1 << 23); // IEEE_MODE

    // RDNA mode bits (WGP_MODE, MEM_ORDERED, FWD_PROGRESS) are profile-driven:
    // MEM_ORDERED is CRITICAL — without it, GLOBAL_STORE may be silently
    // dropped when the wave retires.
    if profile.has_wgp_mode {
        rsrc1 |= 1 << 29; // WGP_MODE
    }
    if profile.has_mem_ordered {
        rsrc1 |= 1 << 30; // MEM_ORDERED
        rsrc1 |= 1 << 31; // FWD_PROGRESS
    }

    rsrc1
}

/// Build `COMPUTE_RESOURCE_LIMITS` register value (profile-driven).
fn compute_resource_limits_profiled(info: &ShaderInfo, profile: &AmdGenerationProfile) -> u32 {
    let threads_per_wg = info.workgroup[0] * info.workgroup[1] * info.workgroup[2];
    let waves_per_threadgroup = threads_per_wg.div_ceil(info.wave_size);

    let simd_dest = if waves_per_threadgroup.is_multiple_of(4) {
        1_u32
    } else {
        0
    };

    (simd_dest << 4) | (profile.max_waves_per_sh << 12)
}

/// Build `COMPUTE_RESOURCE_LIMITS` register value (legacy interface).
#[cfg(test)]
const fn compute_resource_limits(info: &ShaderInfo) -> u32 {
    let threads_per_wg = info.workgroup[0] * info.workgroup[1] * info.workgroup[2];
    let waves_per_threadgroup = threads_per_wg.div_ceil(info.wave_size);
    let simd_dest = if waves_per_threadgroup.is_multiple_of(4) {
        1_u32
    } else {
        0
    };
    let max_waves_per_sh = 600_u32;
    (simd_dest << 4) | (max_waves_per_sh << 12)
}

/// Build `COMPUTE_PGM_RSRC2` register value.
///
/// `user_sgpr_count` is the number of SGPRs populated from `COMPUTE_USER_DATA`
/// (0..16). Workgroup IDs (TGID X/Y/Z) are placed by hardware starting at the
/// first SGPR after user data.
///
/// TIDIG_COMP_CNT controls how many thread ID dimensions the hardware
/// initializes in VGPRs: 0=X only (v0), 1=X+Y (v0,v1), 2=X+Y+Z (v0,v1,v2).
const fn compute_pgm_rsrc2(user_sgpr_count: u32) -> u32 {
    let user_sgpr = if user_sgpr_count > 0 {
        user_sgpr_count
    } else {
        2
    };
    let tgid_x_en = 1_u32;
    let tgid_y_en = 1_u32;
    let tgid_z_en = 1_u32;
    let tidig_comp_cnt = 2_u32; // initialize v0=TID.X, v1=TID.Y, v2=TID.Z
    (user_sgpr << 1)
        | (tgid_x_en << 7)
        | (tgid_y_en << 8)
        | (tgid_z_en << 9)
        | (tidig_comp_cnt << 11)
}

#[cfg(test)]
#[path = "pm4_tests.rs"]
mod tests;
