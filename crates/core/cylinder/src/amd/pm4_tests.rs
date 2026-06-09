// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn pm4_header_format() {
    let header = pm4_type3_header(PM4_SET_SH_REG, 3);
    assert_eq!(header >> 30, 3);
    assert_eq!((header >> 8) & 0xFF, PM4_SET_SH_REG);
    assert_eq!((header >> 16) & 0x3FFF, 2);
}

#[test]
fn compute_dispatch_non_empty() {
    let info = ShaderInfo {
        gpr_count: 16,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [64, 1, 1],
    };
    let pm4 = build_compute_dispatch(0x1_0000_0000, DispatchDims::linear(64), &info, &[], 10);
    assert!(!pm4.is_empty());
    assert!(pm4.len() >= 10);
}

#[test]
fn compute_dispatch_with_buffer_vas() {
    let info = ShaderInfo {
        gpr_count: 16,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [64, 1, 1],
    };
    let buf_vas = [0x1_0000_0000_u64, 0x2_0000_0000_u64];
    let pm4 =
        build_compute_dispatch(0x3_0000_0000, DispatchDims::linear(64), &info, &buf_vas, 10);
    assert!(!pm4.is_empty());
    assert!(pm4.len() > 14, "PM4 should contain user data packets");
}

#[test]
fn pgm_rsrc1_encoding() {
    let rsrc1 = compute_pgm_rsrc1(16, 16, 8, 9);
    let vgprs = rsrc1 & 0x3F;
    let sgprs = (rsrc1 >> 6) & 0xF;
    assert_eq!(vgprs, 1); // (16/8) - 1
    assert_eq!(sgprs, 0); // (16/16) - 1
    let float_mode = (rsrc1 >> 12) & 0xFF;
    assert_eq!(float_mode, 0xC0); // f64 denorms
    assert_eq!((rsrc1 >> 21) & 1, 1); // DX10_CLAMP
    assert_eq!((rsrc1 >> 23) & 1, 1); // IEEE_MODE
}

#[test]
fn pgm_rsrc1_gfx10_sets_mem_ordered() {
    let rsrc1 = compute_pgm_rsrc1(16, 16, 8, 10);
    assert_ne!(rsrc1 & (1 << 29), 0, "WGP_MODE for GFX10+");
    assert_ne!(rsrc1 & (1 << 30), 0, "MEM_ORDERED for GFX10+");
    assert_ne!(rsrc1 & (1 << 31), 0, "FWD_PROGRESS for GFX10+");
}

#[test]
fn pgm_rsrc1_gfx9_no_mem_ordered() {
    let rsrc1 = compute_pgm_rsrc1(16, 16, 4, 9);
    assert_eq!(rsrc1 & (1 << 29), 0, "no WGP_MODE on GFX9");
    assert_eq!(rsrc1 & (1 << 30), 0, "no MEM_ORDERED on GFX9");
    assert_eq!(rsrc1 & (1 << 31), 0, "no FWD_PROGRESS on GFX9");
}

#[test]
fn dispatch_dims_linear() {
    let d = DispatchDims::linear(128);
    assert_eq!(d.x, 128);
    assert_eq!(d.y, 1);
    assert_eq!(d.z, 1);
}

#[test]
fn pm4_compute_dispatch_empty_buffer_vas() {
    let info = ShaderInfo {
        gpr_count: 4,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [1, 1, 1],
    };
    let pm4 = build_compute_dispatch(0, DispatchDims::new(1, 1, 1), &info, &[], 10);
    assert!(!pm4.is_empty());
    assert!(pm4.len() >= 8);
}

#[test]
fn pm4_compute_dispatch_minimal_gpr() {
    let info = ShaderInfo {
        gpr_count: 0,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [32, 1, 1],
    };
    let pm4 = build_compute_dispatch(0x1000, DispatchDims::linear(32), &info, &[], 10);
    assert!(!pm4.is_empty());
    assert!(
        pm4.len() >= 8,
        "PM4 with gpr_count=0 should still produce valid stream"
    );
}

#[test]
fn pm4_compute_dispatch_multiple_buffer_vas() {
    let info = ShaderInfo {
        gpr_count: 32,
        shared_mem_bytes: 256,
        barrier_count: 1,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [64, 2, 1],
    };
    let buf_vas = [0x1_0000_0000_u64, 0x2_0000_0000_u64, 0x3_0000_0000_u64];
    let pm4 = build_compute_dispatch(
        0x4_0000_0000,
        DispatchDims::new(128, 4, 2),
        &info,
        &buf_vas,
        10,
    );
    assert!(pm4.len() > 20);
}

#[test]
fn pm4_compute_dispatch_ends_with_nop() {
    let info = ShaderInfo {
        gpr_count: 8,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [16, 1, 1],
    };
    let pm4 = build_compute_dispatch(0x1000, DispatchDims::linear(16), &info, &[], 10);
    assert!(pm4.len() >= 2);
    let last_header = pm4[pm4.len() - 2];
    assert_eq!(last_header >> 30, 3, "trailing packet should be Type 3");
}

#[test]
fn compute_pgm_rsrc2_encoding() {
    let rsrc2_zero = compute_pgm_rsrc2(0);
    assert_eq!(rsrc2_zero & 0x7E, 4, "zero user_sgpr uses default 2");
    let rsrc2_with_user = compute_pgm_rsrc2(4);
    assert_eq!((rsrc2_with_user >> 1) & 0x3F, 4);
    assert_eq!((rsrc2_with_user >> 7) & 1, 1, "tgid_x_en");
    assert_eq!((rsrc2_with_user >> 8) & 1, 1, "tgid_y_en");
    assert_eq!((rsrc2_with_user >> 9) & 1, 1, "tgid_z_en");
}

#[test]
fn pm4_dispatch_direct_dims() {
    let info = ShaderInfo {
        gpr_count: 16,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [32, 4, 2],
    };
    let dims = DispatchDims::new(128, 64, 8);
    let pm4 = build_compute_dispatch(0x1000, dims, &info, &[], 10);
    let dispatch_start = pm4
        .iter()
        .position(|&w| (w >> 8) & 0xFF == PM4_DISPATCH_DIRECT && w >> 30 == 3)
        .expect("DISPATCH_DIRECT packet not found");
    assert_eq!(pm4[dispatch_start + 1], 128);
    assert_eq!(pm4[dispatch_start + 2], 64);
    assert_eq!(pm4[dispatch_start + 3], 8);
}

#[test]
fn pm4_shader_address_encoding() {
    let shader_va = 0x1_2345_6789_ABCD_u64;
    let info = ShaderInfo {
        gpr_count: 16,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [64, 1, 1],
    };
    let pm4 = build_compute_dispatch(shader_va, DispatchDims::linear(1), &info, &[], 10);
    let pgm_lo_expected = (shader_va >> 8) as u32;
    let pgm_hi_expected = (shader_va >> 40) as u32;
    assert!(
        pm4.windows(3)
            .any(|w| w[1] == pgm_lo_expected && w[2] == pgm_hi_expected),
        "PGM_LO/HI values should appear in stream"
    );
}

#[test]
fn pm4_nop_opcode() {
    let info = ShaderInfo {
        gpr_count: 4,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [1, 1, 1],
    };
    let pm4 = build_compute_dispatch(0, DispatchDims::new(1, 1, 1), &info, &[], 10);
    let last_header = pm4[pm4.len() - 2];
    assert_eq!((last_header >> 8) & 0xFF, PM4_NOP, "IB should end with NOP");
}

#[test]
fn compute_pgm_rsrc1_minimum_vgpr() {
    let rsrc1 = compute_pgm_rsrc1(4, 16, 8, 10);
    let vgprs = rsrc1 & 0x3F;
    assert_eq!(vgprs, 0, "4 VGPRs encodes as 0 (ceil(4/8)-1)");
    assert_eq!((rsrc1 >> 12) & 0xFF, 0xC0); // FLOAT_MODE
}

#[test]
fn compute_pgm_rsrc1_gcn5_granularity() {
    let rsrc1 = compute_pgm_rsrc1(26, 16, 4, 9);
    let vgprs = rsrc1 & 0x3F;
    assert_eq!(vgprs, 6, "26 VGPRs at granularity 4: ceil(26/4)-1 = 6");
}

#[test]
fn pm4_set_sh_reg_packet_structure() {
    let info = ShaderInfo {
        gpr_count: 8,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [1, 1, 1],
    };
    let pm4 = build_compute_dispatch(0x1000, DispatchDims::new(1, 1, 1), &info, &[], 10);
    assert!(pm4.len() >= 4);
    let first_header = pm4[0];
    assert_eq!(first_header >> 30, 3, "Type 3 packet");
    assert_eq!((first_header >> 8) & 0xFF, PM4_SET_SH_REG);
}

#[test]
fn pm4_user_data_va_split() {
    let va = 0x1234_5678_9ABC_DEF0_u64;
    let lo = va as u32;
    let hi = (va >> 32) as u32;
    assert_eq!(lo, 0x9ABC_DEF0);
    assert_eq!(hi, 0x1234_5678);
}

#[test]
fn compute_resource_limits_waves_multiple_of_four_sets_simd_dest() {
    let info = ShaderInfo {
        gpr_count: 16,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 64,
        local_mem_bytes: None,
        workgroup: [256, 1, 1],
    };
    let lim = compute_resource_limits(&info);
    assert_eq!(
        (lim >> 4) & 1,
        1,
        "SIMD_DEST_CNTL when waves/threadgroup % 4 == 0"
    );
    assert_eq!((lim >> 12) & 0xFFFF, 600);
}

#[test]
fn compute_resource_limits_waves_not_multiple_of_four_clears_simd_dest() {
    let info = ShaderInfo {
        gpr_count: 16,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 64,
        local_mem_bytes: None,
        workgroup: [64, 1, 1],
    };
    let lim = compute_resource_limits(&info);
    assert_eq!((lim >> 4) & 1, 0);
}

#[test]
fn pm4_dispatch_initiator_wave32_sets_cs_w32_en() {
    let info = ShaderInfo {
        gpr_count: 16,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [32, 1, 1],
    };
    let pm4 = build_compute_dispatch(0x1000, DispatchDims::linear(1), &info, &[], 10);
    let dispatch_start = pm4
        .iter()
        .position(|&w| (w >> 8) & 0xFF == PM4_DISPATCH_DIRECT && w >> 30 == 3)
        .expect("DISPATCH_DIRECT header");
    let initiator = pm4[dispatch_start + 4];
    assert_ne!(initiator & (1 << 15), 0, "CS_W32_EN for wave32");
}

#[test]
fn pm4_dispatch_initiator_wave64_clears_cs_w32_en() {
    let info = ShaderInfo {
        gpr_count: 16,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 64,
        local_mem_bytes: None,
        workgroup: [64, 1, 1],
    };
    let pm4 = build_compute_dispatch(0x1000, DispatchDims::linear(1), &info, &[], 9);
    let dispatch_start = pm4
        .iter()
        .position(|&w| (w >> 8) & 0xFF == PM4_DISPATCH_DIRECT && w >> 30 == 3)
        .expect("DISPATCH_DIRECT header");
    let initiator = pm4[dispatch_start + 4];
    assert_eq!(
        initiator & (1 << 15),
        0,
        "GCN5 wave64 leaves CS_W32_EN clear"
    );
}

#[test]
fn pm4_acquire_mem_after_dispatch_gfx9_has_tc_wb() {
    let info = ShaderInfo {
        gpr_count: 8,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 64,
        local_mem_bytes: None,
        workgroup: [64, 1, 1],
    };
    let pm4 = build_compute_dispatch(0x2000, DispatchDims::linear(1), &info, &[], 9);
    let dispatch_idx = pm4
        .iter()
        .position(|&w| (w >> 8) & 0xFF == PM4_DISPATCH_DIRECT && w >> 30 == 3)
        .expect("dispatch");
    let post_dispatch = &pm4[dispatch_idx..];
    let acquire_after = post_dispatch
        .windows(2)
        .find(|w| (w[0] >> 8) & 0xFF == PM4_ACQUIRE_MEM && w[0] >> 30 == 3)
        .expect("post-dispatch ACQUIRE_MEM");
    assert_ne!(
        acquire_after[1] & (1 << 18),
        0,
        "L2 writeback (TC_WB_ACTION_ENA) after dispatch on GFX9"
    );
}

#[test]
fn pm4_acquire_mem_after_dispatch_gfx10_has_gl2_wb() {
    let info = ShaderInfo {
        gpr_count: 8,
        shared_mem_bytes: 0,
        barrier_count: 0,
        wave_size: 32,
        local_mem_bytes: None,
        workgroup: [32, 1, 1],
    };
    let pm4 = build_compute_dispatch(0x2000, DispatchDims::linear(1), &info, &[], 10);
    let dispatch_idx = pm4
        .iter()
        .position(|&w| (w >> 8) & 0xFF == PM4_DISPATCH_DIRECT && w >> 30 == 3)
        .expect("dispatch");
    let post_dispatch = &pm4[dispatch_idx..];
    let acquire_idx = post_dispatch
        .iter()
        .position(|&w| (w >> 8) & 0xFF == PM4_ACQUIRE_MEM && w >> 30 == 3)
        .expect("post-dispatch ACQUIRE_MEM");
    // GFX10+ ACQUIRE_MEM: 7 body dwords, CP_COHER_CNTL (body[0]) unused,
    // GCR_CNTL at body[6] = header + 7
    assert_eq!(
        post_dispatch[acquire_idx + 1],
        0,
        "CP_COHER_CNTL should be 0 on GFX10+"
    );
    let gcr_cntl = post_dispatch[acquire_idx + 7];
    assert_ne!(gcr_cntl & (1 << 15), 0, "GL2_WB [15] in GCR_CNTL");
    assert_ne!(gcr_cntl & (1 << 14), 0, "GL2_INV [14] in GCR_CNTL");
    assert_ne!(gcr_cntl & (1 << 9), 0, "GL1_INV [9] in GCR_CNTL");
}
