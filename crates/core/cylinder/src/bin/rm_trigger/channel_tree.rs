// SPDX-License-Identifier: AGPL-3.0-or-later
//! Phase 2–4: VA space → memory → TSG → GPFIFO channel → BIND → SCHEDULE → token.

use std::os::fd::AsFd;

use toadstool_cylinder::nv::rm_abi::{
    self, NV2080_ENGINE_TYPE_GR0, NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED,
    NvA06fGetWorkSubmitTokenParams, NvChannelAllocParams, NvChannelBindParams,
    NvChannelGroupAllocParams, NvCtxShareAllocParams, NvMemoryAllocParams, NvVaspaceAllocParams,
    class,
};

use super::rm_ioctl::{self, NvGpfifoScheduleParams};
use super::rm_object_tree::{
    H_CHANNEL, H_COMPUTE, H_CTX_SHARE, H_MEM_ERR_NOTIFIER, H_MEM_GPFIFO, H_MEM_USERD, H_TSG,
    H_VASPACE, step_json,
};

/// Result of full channel tree allocation (Phase 2–4).
pub struct ChannelTreeResult {
    pub channel_id: Option<u32>,
    pub work_submit_token: Option<u32>,
}

/// Phase 2–4: Full RM compute channel (VA → memory → TSG → GPFIFO → BIND → SCHEDULE → token).
pub fn alloc_compute_channel(
    fd: &impl AsFd,
    rm_root: u32,
    rm_device: u32,
    rm_subdevice: u32,
    success: &mut bool,
) -> (ChannelTreeResult, Vec<serde_json::Value>) {
    let mut steps = Vec::new();
    let mut channel_id: Option<u32> = None;
    let mut work_submit_token: Option<u32> = None;

    eprintln!("\n════ Channel mode: full RM compute channel (kernel-assigned handles) ════\n");

    // Step 5: VA space
    let mut rm_vaspace = 0u32;
    {
        eprintln!("[Phase 2] Step 5: VA space (FERMI_VASPACE_A)...");
        let mut vp = NvVaspaceAllocParams::default();
        vp.flags = 0;
        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_device,
            H_VASPACE,
            class::FERMI_VASPACE_A,
            &vp as *const _ as u64,
            size_of::<NvVaspaceAllocParams>() as u32,
        );
        rm_vaspace = r.h_object_new;
        let va_ok = r.rc == 0 && r.status == 0;
        steps.push(step_json(
            "vaspace_alloc",
            va_ok,
            serde_json::json!({
                "class": "FERMI_VASPACE_A", "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_vaspace:08x}"),
            }),
        ));
        if !va_ok {
            *success = false;
        }
    }

    // Step 6: USERD memory
    let mut rm_userd = 0u32;
    if *success {
        eprintln!("[Phase 2] Step 6: USERD memory...");
        let mut mp = NvMemoryAllocParams::default();
        mp.owner = rm_root;
        mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
        mp.size = 4096;
        mp.alignment = 4096;
        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_device,
            H_MEM_USERD,
            class::NV01_MEMORY_SYSTEM,
            &mp as *const _ as u64,
            size_of::<NvMemoryAllocParams>() as u32,
        );
        rm_userd = r.h_object_new;
        let ok = r.rc == 0 && r.status == 0;
        steps.push(step_json(
            "userd_mem_alloc",
            ok,
            serde_json::json!({
                "class": "NV01_MEMORY_SYSTEM", "size": 4096,
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_userd:08x}"),
            }),
        ));
        if !ok {
            *success = false;
        }
    }

    // Step 7: GPFIFO ring memory
    let mut rm_gpfifo_mem = 0u32;
    if *success {
        eprintln!("[Phase 2] Step 7: GPFIFO ring memory...");
        let mut mp = NvMemoryAllocParams::default();
        mp.owner = rm_root;
        mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
        mp.size = 4096;
        mp.alignment = 4096;
        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_device,
            H_MEM_GPFIFO,
            class::NV01_MEMORY_SYSTEM,
            &mp as *const _ as u64,
            size_of::<NvMemoryAllocParams>() as u32,
        );
        rm_gpfifo_mem = r.h_object_new;
        let ok = r.rc == 0 && r.status == 0;
        steps.push(step_json(
            "gpfifo_mem_alloc",
            ok,
            serde_json::json!({
                "class": "NV01_MEMORY_SYSTEM", "size": 4096,
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_gpfifo_mem:08x}"),
            }),
        ));
        if !ok {
            *success = false;
        }
    }

    // Step 8: Error notifier memory
    let mut rm_err_notifier = 0u32;
    if *success {
        eprintln!("[Phase 2] Step 8: Error notifier memory...");
        let mut mp = NvMemoryAllocParams::default();
        mp.owner = rm_device;
        mp.mem_type = 13;
        mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
        mp.size = 4096;
        mp.alignment = 4096;
        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_device,
            H_MEM_ERR_NOTIFIER,
            class::NV01_MEMORY_SYSTEM,
            &mp as *const _ as u64,
            size_of::<NvMemoryAllocParams>() as u32,
        );
        rm_err_notifier = r.h_object_new;
        let ok = r.rc == 0 && r.status == 0;
        steps.push(step_json(
            "err_notifier_mem_alloc",
            ok,
            serde_json::json!({
                "class": "NV01_MEMORY_SYSTEM", "mem_type": 13,
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_err_notifier:08x}"),
            }),
        ));
        if !ok {
            *success = false;
        }
    }

    // Step 9: TSG
    let mut rm_tsg = 0u32;
    if *success {
        eprintln!("[Phase 3] Step 9: TSG...");
        let mut tsg = NvChannelGroupAllocParams::default();
        tsg.h_object_error = rm_err_notifier;
        tsg.h_vaspace = rm_vaspace;
        tsg.engine_type = NV2080_ENGINE_TYPE_GR0;
        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_device,
            H_TSG,
            class::KEPLER_CHANNEL_GROUP_A,
            &tsg as *const _ as u64,
            size_of::<NvChannelGroupAllocParams>() as u32,
        );
        rm_tsg = r.h_object_new;
        let ok = r.rc == 0 && r.status == 0;
        steps.push(step_json(
            "tsg_alloc",
            ok,
            serde_json::json!({
                "class": "KEPLER_CHANNEL_GROUP_A",
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_tsg:08x}"),
            }),
        ));
        if !ok {
            *success = false;
        }
    }

    // Step 10: Context share
    let mut rm_ctx_share = 0u32;
    if *success {
        eprintln!("[Phase 3] Step 10: Context share...");
        let mut cs = NvCtxShareAllocParams::default();
        cs.h_vaspace = rm_vaspace;
        cs.h_subdevice = rm_subdevice;
        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_tsg,
            H_CTX_SHARE,
            class::FERMI_CONTEXT_SHARE_A,
            &cs as *const _ as u64,
            size_of::<NvCtxShareAllocParams>() as u32,
        );
        rm_ctx_share = r.h_object_new;
        let ok = r.rc == 0 && r.status == 0;
        steps.push(step_json(
            "ctx_share_alloc",
            ok,
            serde_json::json!({
                "class": "FERMI_CONTEXT_SHARE_A",
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_ctx_share:08x}"),
            }),
        ));
        if !ok {
            *success = false;
        }
    }

    // Step 11: GPFIFO channel
    let mut rm_channel = 0u32;
    if *success {
        eprintln!("[Phase 3] Step 11: GPFIFO channel (VOLTA_CHANNEL_GPFIFO_A)...");
        let mut ch = NvChannelAllocParams::default();
        ch.h_object_error = rm_err_notifier;
        ch.h_object_buffer = rm_gpfifo_mem;
        ch.gpfifo_entries = 64;
        ch.h_context_share = rm_ctx_share;
        ch.h_vaspace = rm_vaspace;
        ch.h_userd_memory[0] = rm_userd;
        ch.engine_type = NV2080_ENGINE_TYPE_GR0;

        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_tsg,
            H_CHANNEL,
            class::VOLTA_CHANNEL_GPFIFO_A,
            &ch as *const _ as u64,
            size_of::<NvChannelAllocParams>() as u32,
        );
        rm_channel = r.h_object_new;

        let ch_ok = r.rc == 0 && r.status == 0;
        if ch_ok {
            channel_id = Some(ch.cid);
            eprintln!(
                "  Channel allocated, cid={}, kernel_handle=0x{rm_channel:08x}",
                ch.cid
            );
        }
        steps.push(step_json(
            "channel_alloc",
            ch_ok,
            serde_json::json!({
                "class": "VOLTA_CHANNEL_GPFIFO_A",
                "status": format!("0x{:x}", r.status),
                "channel_id": ch.cid,
                "kernel_handle": format!("0x{rm_channel:08x}"),
            }),
        ));
        if !ch_ok {
            *success = false;
        }
    }

    // Step 12: Compute engine object
    let mut rm_compute = 0u32;
    if *success {
        eprintln!("[Phase 3] Step 12: Compute engine (VOLTA_COMPUTE_A)...");
        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_channel,
            H_COMPUTE,
            class::VOLTA_COMPUTE_A,
            0,
            0,
        );
        rm_compute = r.h_object_new;
        let ok = r.rc == 0 && r.status == 0;
        steps.push(step_json(
            "compute_alloc",
            ok,
            serde_json::json!({
                "class": "VOLTA_COMPUTE_A",
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_compute:08x}"),
            }),
        ));
        if !ok {
            *success = false;
        }
    }

    // Step 13: BIND channel to GR engine
    if *success {
        eprintln!("[Phase 4] Step 13: BIND channel to GR...");
        let mut bp = NvChannelBindParams::default();
        bp.h_engine_object = rm_compute;
        bp.engine_class_1 = class::VOLTA_COMPUTE_A;
        bp.engine_class_2 = class::VOLTA_COMPUTE_A;
        bp.engine_type = NV2080_ENGINE_TYPE_GR0;
        let (rc, status) = rm_ioctl::rm_ctrl(
            fd,
            rm_root,
            rm_channel,
            rm_abi::NV906F_CTRL_CMD_BIND,
            &bp as *const _ as u64,
            size_of::<NvChannelBindParams>() as u32,
        );
        let bind_ok = rc == 0 && status == 0;
        steps.push(step_json(
            "channel_bind",
            bind_ok,
            serde_json::json!({
                "cmd": "BIND", "status": format!("0x{status:x}"),
            }),
        ));
        if !bind_ok {
            eprintln!("  BIND status=0x{status:x} — Volta may auto-bind via TSG, proceeding...");
        }
    }

    // Step 14: SCHEDULE TSG
    if *success {
        eprintln!("[Phase 4] Step 14: SCHEDULE TSG (handle=0x{rm_tsg:08x})...");
        let mut sp = NvGpfifoScheduleParams { b_enable: 1 };
        let (rc, status) = rm_ioctl::rm_ctrl(
            fd,
            rm_root,
            rm_tsg,
            rm_abi::NVA06C_CTRL_CMD_GPFIFO_SCHEDULE,
            &mut sp as *mut _ as u64,
            size_of::<NvGpfifoScheduleParams>() as u32,
        );
        steps.push(step_json(
            "tsg_schedule",
            rc == 0 && status == 0,
            serde_json::json!({
                "cmd": "GPFIFO_SCHEDULE", "status": format!("0x{status:x}"),
            }),
        ));
        if rc != 0 || status != 0 {
            *success = false;
        }
    }

    // Step 15: GET_WORK_SUBMIT_TOKEN
    if *success {
        eprintln!("[Phase 4] Step 15: GET_WORK_SUBMIT_TOKEN (handle=0x{rm_channel:08x})...");
        let mut tp = NvA06fGetWorkSubmitTokenParams::default();
        let (rc, status) = rm_ioctl::rm_ctrl(
            fd,
            rm_root,
            rm_channel,
            rm_abi::NVA06F_CTRL_CMD_GPFIFO_GET_WORK_SUBMIT_TOKEN,
            &mut tp as *mut _ as u64,
            size_of::<NvA06fGetWorkSubmitTokenParams>() as u32,
        );
        if rc == 0 && status == 0 {
            work_submit_token = Some(tp.work_submit_token);
            eprintln!("  work_submit_token = 0x{:08x}", tp.work_submit_token);
        }
        steps.push(step_json(
            "work_submit_token",
            rc == 0 && status == 0,
            serde_json::json!({
                "cmd": "GET_WORK_SUBMIT_TOKEN",
                "status": format!("0x{status:x}"),
                "token": format!("0x{:08x}", tp.work_submit_token),
            }),
        ));
        if status != 0 {
            *success = false;
        }
    }

    (
        ChannelTreeResult {
            channel_id,
            work_submit_token,
        },
        steps,
    )
}
