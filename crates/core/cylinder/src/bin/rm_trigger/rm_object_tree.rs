// SPDX-License-Identifier: AGPL-3.0-or-later
//! RM object tree allocation — root → device → subdevice → VA → memory → TSG → GPFIFO.

use std::os::fd::{AsFd, AsRawFd};

use toadstool_cylinder::nv::rm_abi::{self, class};

use super::rm_ioctl::{self, RmAllocResult};

pub use crate::channel_tree::alloc_compute_channel;

// ── Handle namespace ────────────────────────────────────────────────────
pub const H_ROOT: u32 = 0xCAFE_0001;
pub const H_DEVICE: u32 = 0xCAFE_0002;
pub const H_SUBDEVICE: u32 = 0xCAFE_0003;
pub const H_VASPACE: u32 = 0xCAFE_0010;
pub const H_MEM_USERD: u32 = 0xCAFE_0020;
pub const H_MEM_GPFIFO: u32 = 0xCAFE_0021;
pub const H_MEM_ERR_NOTIFIER: u32 = 0xCAFE_0022;
pub const H_TSG: u32 = 0xCAFE_0030;
pub const H_CTX_SHARE: u32 = 0xCAFE_0031;
pub const H_CHANNEL: u32 = 0xCAFE_0040;
pub const H_COMPUTE: u32 = 0xCAFE_0041;

pub(crate) fn step_json(name: &str, ok: bool, detail: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"step": name, "ok": ok, "detail": detail})
}

/// Result of Phase 0 root client allocation.
pub struct RootClientResult {
    pub root_test: RmAllocResult,
    pub rm_root: u32,
    pub gpu_id: u32,
    pub step: serde_json::Value,
}

/// Phase 0: Root client — kernel REWRITES h_object_new.
pub fn alloc_root_client(fd: &impl AsFd) -> RootClientResult {
    eprintln!("\n[Phase 0] Allocating root client (class 0x0041 NV01_ROOT_CLIENT)...");
    let root_test = rm_ioctl::rm_alloc(fd, 0, 0, H_ROOT, class::NV01_ROOT_CLIENT, 0, 0);
    let rm_root = if root_test.status == 0 {
        root_test.h_object_new
    } else {
        eprintln!("  ROOT_CLIENT(0x41) failed, falling back to NV01_ROOT(0x0000)...");
        let r2 = rm_ioctl::rm_alloc(fd, 0, 0, H_ROOT, class::NV01_ROOT, 0, 0);
        r2.h_object_new
    };
    eprintln!("  Kernel assigned root handle: 0x{rm_root:08x} (we asked for 0x{H_ROOT:08x})");

    let mut gpu_ids = [0u32; 32];
    let (_, ctrl_st) = rm_ioctl::rm_ctrl(
        fd,
        rm_root,
        rm_root,
        0x0000_0201,
        gpu_ids.as_mut_ptr() as u64,
        size_of::<[u32; 32]>() as u32,
    );
    let attached: Vec<u32> = gpu_ids
        .iter()
        .copied()
        .filter(|&id| id != 0 && id != 0xFFFF_FFFF)
        .collect();
    eprintln!("[Phase 0] GPU_GET_ATTACHED_IDS: status=0x{ctrl_st:x} ids={attached:?}");

    let step = step_json(
        "root_client",
        root_test.status == 0,
        serde_json::json!({
            "requested_handle": format!("0x{H_ROOT:08x}"),
            "kernel_assigned": format!("0x{rm_root:08x}"),
            "attached_ids": attached,
        }),
    );

    let gpu_id = attached.first().copied().unwrap_or(0);

    RootClientResult {
        root_test,
        rm_root,
        gpu_id,
        step,
    }
}

/// Pre-attach diagnostics: GPU_GET_PROBED_IDS query.
pub fn pre_attach_diagnostics(fd: &impl AsFd, rm_root: u32) {
    let mut pre_probed = [0u32; 32];
    let (_, st) = rm_ioctl::rm_ctrl(
        fd,
        rm_root,
        rm_root,
        0x0000_0214,
        pre_probed.as_mut_ptr() as u64,
        size_of::<[u32; 32]>() as u32,
    );
    let ids: Vec<u32> = pre_probed
        .iter()
        .copied()
        .filter(|&id| id != 0 && id != 0xFFFF_FFFF)
        .collect();
    eprintln!("[Diag] PRE-ATTACH GPU_GET_PROBED_IDS: status=0x{st:x} ids={ids:?}");
}

/// GPU_ATTACH_IDS ctrl cmd (0x0280) — manually register GPU with RM's GPU manager.
pub fn gpu_attach_ids_ctrl(fd: &impl AsFd, rm_root: u32, gpu_id: u32) -> serde_json::Value {
    let mut attach_ctrl_ids = [gpu_id, 0u32];
    let (_, st) = rm_ioctl::rm_ctrl(
        fd,
        rm_root,
        rm_root,
        0x0000_0280,
        attach_ctrl_ids.as_mut_ptr() as u64,
        size_of::<[u32; 2]>() as u32,
    );
    eprintln!("[Diag] GPU_ATTACH_IDS(0x{gpu_id:x}): status=0x{st:x}");
    step_json(
        "gpu_attach_ids_ctrl",
        st == 0,
        serde_json::json!({
            "gpu_id": format!("0x{gpu_id:x}"), "status": format!("0x{st:x}"),
        }),
    )
}

/// Post-attach-ctrl check: GPU_GET_PROBED_IDS query.
pub fn post_attach_diagnostics(fd: &impl AsFd, rm_root: u32) {
    let mut post_probed = [0u32; 32];
    let (_, st) = rm_ioctl::rm_ctrl(
        fd,
        rm_root,
        rm_root,
        0x0000_0214,
        post_probed.as_mut_ptr() as u64,
        size_of::<[u32; 32]>() as u32,
    );
    let ids: Vec<u32> = post_probed
        .iter()
        .copied()
        .filter(|&id| id != 0 && id != 0xFFFF_FFFF)
        .collect();
    eprintln!("[Diag] POST-ATTACH-CTRL GPU_GET_PROBED_IDS: status=0x{st:x} ids={ids:?}");
}

/// Core object tree handles from Phase 1.
pub struct CoreTreeHandles {
    pub rm_device: u32,
    pub rm_subdevice: u32,
}

/// Phase 1: Core object tree using kernel-assigned handles (device → subdevice → GR_GET_INFO).
pub fn alloc_core_tree(
    fd: &impl AsFd,
    rm_root: u32,
    gpu_id: u32,
    root_ok: bool,
    success: &mut bool,
) -> (CoreTreeHandles, Vec<serde_json::Value>) {
    let mut steps = Vec::new();
    let mut rm_device = 0u32;
    let mut rm_subdevice = 0u32;

    if root_ok && *success {
        eprintln!("\n[Phase 1] Step 2: device alloc (parent=0x{rm_root:08x})...");

        let mut probed_ids = [0u32; 32];
        let (_, probed_st) = rm_ioctl::rm_ctrl(
            fd,
            rm_root,
            rm_root,
            0x0000_0214,
            probed_ids.as_mut_ptr() as u64,
            size_of::<[u32; 32]>() as u32,
        );
        let probed: Vec<u32> = probed_ids
            .iter()
            .copied()
            .filter(|&id| id != 0 && id != 0xFFFF_FFFF)
            .collect();
        eprintln!("  GPU_GET_PROBED_IDS: status=0x{probed_st:x} ids={probed:?}");

        let mut dp = rm_abi::Nv0080AllocParams::default();
        dp.device_id = 0;
        dp.h_client_share = rm_root;

        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_root,
            H_DEVICE,
            class::NV01_DEVICE_0,
            &dp as *const _ as u64,
            size_of::<rm_abi::Nv0080AllocParams>() as u32,
        );
        rm_device = r.h_object_new;
        let mut device_ok = r.rc == 0 && r.status == 0;
        eprintln!(
            "  device_alloc(device_id=0): status=0x{:x} handle=0x{:08x}",
            r.status, r.h_object_new
        );

        if !device_ok && gpu_id != 0 {
            eprintln!("  Retrying with device_id=gpu_id (0x{gpu_id:x})...");
            dp.device_id = gpu_id;
            let r2 = rm_ioctl::rm_alloc(
                fd,
                rm_root,
                rm_root,
                H_DEVICE,
                class::NV01_DEVICE_0,
                &dp as *const _ as u64,
                size_of::<rm_abi::Nv0080AllocParams>() as u32,
            );
            rm_device = r2.h_object_new;
            device_ok = r2.rc == 0 && r2.status == 0;
            eprintln!(
                "  device_alloc(device_id=0x{gpu_id:x}): status=0x{:x} handle=0x{:08x}",
                r2.status, r2.h_object_new
            );
        }

        steps.push(step_json(
            "device_alloc",
            device_ok,
            serde_json::json!({
                "class": "NV01_DEVICE_0",
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_device:08x}"),
                "probed_ids": probed,
            }),
        ));

        if !device_ok {
            let mut id_info = [0u32; 6];
            id_info[0] = gpu_id;
            let (_, st) = rm_ioctl::rm_ctrl(
                fd,
                rm_root,
                rm_root,
                0x0000_0202,
                id_info.as_mut_ptr() as u64,
                24,
            );
            eprintln!(
                "[Diag] GPU_GET_ID_INFO(0x{gpu_id:x}): status=0x{st:x} devInst={} subDevInst={} gpuInst={}",
                id_info[2], id_info[3], id_info[5]
            );
            *success = false;
        }
    }

    if root_ok && *success {
        eprintln!("\n[Phase 1] Step 3: subdevice (parent=0x{rm_device:08x})...");
        let mut sp = rm_abi::Nv2080AllocParams::default();
        sp.sub_device_id = 0;
        let r = rm_ioctl::rm_alloc(
            fd,
            rm_root,
            rm_device,
            H_SUBDEVICE,
            class::NV20_SUBDEVICE_0,
            &sp as *const _ as u64,
            size_of::<rm_abi::Nv2080AllocParams>() as u32,
        );
        rm_subdevice = r.h_object_new;
        let sub_ok = r.rc == 0 && r.status == 0;
        steps.push(step_json(
            "subdevice_alloc",
            sub_ok,
            serde_json::json!({
                "class": "NV20_SUBDEVICE_0",
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_subdevice:08x}"),
            }),
        ));
        if !sub_ok {
            *success = false;
        }
    }

    if root_ok && *success {
        eprintln!("\n[Phase 1] Step 4: GR_GET_INFO (obj=0x{rm_subdevice:08x})...");
        let (rc, status) = rm_ioctl::rm_ctrl(fd, rm_root, rm_subdevice, 0x2080_1201, 0, 0);
        steps.push(step_json(
            "gr_get_info",
            rc == 0,
            serde_json::json!({
                "cmd": "GR_GET_INFO",
                "status": format!("0x{status:x}"),
            }),
        ));
    }

    (
        CoreTreeHandles {
            rm_device,
            rm_subdevice,
        },
        steps,
    )
}

/// Diagnostic: try root alloc on GPU fd.
pub fn diag_gpu_fd_root_alloc(gpu_file: &(impl AsFd + AsRawFd)) {
    let gpu_raw = gpu_file.as_raw_fd();
    eprintln!("\n[Diag] Trying root alloc on GPU fd ({gpu_raw})...");
    let r_gpu = rm_ioctl::rm_alloc(gpu_file, 0, 0, 0xBEEF_0001, class::NV01_ROOT_CLIENT, 0, 0);
    if r_gpu.status == 0 {
        let gpu_root = r_gpu.h_object_new;
        eprintln!("  ✓ Root client on GPU fd: handle=0x{gpu_root:08x}");
        let mut p_ids = [0u32; 32];
        let (_, st) = rm_ioctl::rm_ctrl(
            gpu_file,
            gpu_root,
            gpu_root,
            0x0000_0214,
            p_ids.as_mut_ptr() as u64,
            size_of::<[u32; 32]>() as u32,
        );
        let ids: Vec<u32> = p_ids
            .iter()
            .copied()
            .filter(|&id| id != 0 && id != 0xFFFF_FFFF)
            .collect();
        eprintln!("  GPU_GET_PROBED_IDS on GPU fd: status=0x{st:x} ids={ids:?}");

        let mut dp2 = rm_abi::Nv0080AllocParams::default();
        dp2.device_id = 0;
        dp2.h_client_share = gpu_root;
        let r2 = rm_ioctl::rm_alloc(
            gpu_file,
            gpu_root,
            gpu_root,
            0xBEEF_0002,
            class::NV01_DEVICE_0,
            &dp2 as *const _ as u64,
            size_of::<rm_abi::Nv0080AllocParams>() as u32,
        );
        eprintln!(
            "  device_alloc on GPU fd(device_id=0): status=0x{:x}",
            r2.status
        );
    } else {
        eprintln!(
            "  ✗ Root client on GPU fd FAILED: status=0x{:x}",
            r_gpu.status
        );
    }
}
