// SPDX-License-Identifier: AGPL-3.0-or-later
//! RM trigger — Full RM compute channel client for catalyst pipeline.
//!
//! Opens nvidiactl + nvidia0, allocates the complete RM object tree needed for
//! a GR compute channel: root → device → subdevice → VA space → memory objects
//! → TSG → context share → GPFIFO channel → compute engine → BIND → SCHEDULE
//! → work submit token. This establishes FECS ctx-switch state that survives
//! the catalyst warm swap (Exp 229).
//!
//! Usage: rm_trigger <major> [--channel]
//!
//! Without --channel: legacy mode (root/device/subdevice/GR_GET_INFO only).
//! With --channel: full RM compute channel creation (Exp 229).
//!
//! Outputs structured JSON on stdout, diagnostics on stderr.

// SAFETY: Raw NVIDIA RM ioctls require unsafe ioctl() calls.
#![allow(unsafe_code)]

use std::os::fd::AsRawFd;
use std::process::ExitCode;

use toadstool_cylinder::nv::rm_abi::{
    self,
    class,
    NvChannelAllocParams,
    NvChannelBindParams,
    NvChannelGroupAllocParams,
    NvCtxShareAllocParams,
    NvMemoryAllocParams,
    NvVaspaceAllocParams,
    NvA06fGetWorkSubmitTokenParams,
    NV2080_ENGINE_TYPE_GR0,
    NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED,
};

const NV_IOCTL_MAGIC: u8 = b'F';
const NV_ESC_RM_ALLOC: u8 = 0x2B;
const NV_ESC_RM_CONTROL: u8 = 0x2A;

/// 470.x uses 32-byte NVOS64 for RM_ALLOC (not 48-byte NvRmAllocParams from 580.x).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Nvos64Parameters {
    h_root: u32,
    h_object_parent: u32,
    h_object_new: u32,
    h_class: u32,
    p_alloc_parms: u64,
    params_size: u32,
    status: u32,
}

/// 470.x RM_CONTROL uses 32-byte NVOS54.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Nvos54Parameters {
    h_client: u32,
    h_object: u32,
    cmd: u32,
    flags: u32,
    params: u64,
    params_size: u32,
    status: u32,
}

/// Scheduling control — enable field for TSG schedule.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct NvGpfifoScheduleParams {
    b_enable: u32,
}

const fn iowr(magic: u8, nr: u8, size: usize) -> u64 {
    let dir: u64 = 3;
    (dir << 30) | ((size as u64 & 0x3FFF) << 16) | ((magic as u64) << 8) | nr as u64
}

const RM_ALLOC_CMD: u64 = iowr(NV_IOCTL_MAGIC, NV_ESC_RM_ALLOC, size_of::<Nvos64Parameters>());
const RM_CTRL_CMD: u64 = iowr(NV_IOCTL_MAGIC, NV_ESC_RM_CONTROL, size_of::<Nvos54Parameters>());

const STATUS_SENTINEL: u32 = 0xDEAD_BEEF;

// ── Handle namespace ────────────────────────────────────────────────────
const H_ROOT: u32 = 0xCAFE_0001;
const H_DEVICE: u32 = 0xCAFE_0002;
const H_SUBDEVICE: u32 = 0xCAFE_0003;
const H_VASPACE: u32 = 0xCAFE_0010;
const H_MEM_USERD: u32 = 0xCAFE_0020;
const H_MEM_GPFIFO: u32 = 0xCAFE_0021;
const H_MEM_ERR_NOTIFIER: u32 = 0xCAFE_0022;
const H_TSG: u32 = 0xCAFE_0030;
const H_CTX_SHARE: u32 = 0xCAFE_0031;
const H_CHANNEL: u32 = 0xCAFE_0040;
const H_COMPUTE: u32 = 0xCAFE_0041;

/// Issue NV_ESC_RM_ALLOC. Returns (ioctl_rc, rm_status).
fn rm_alloc(
    fd: std::os::fd::RawFd,
    root: u32,
    parent: u32,
    handle: u32,
    class: u32,
    params_ptr: u64,
    params_size: u32,
) -> (i32, u32) {
    let mut p = Nvos64Parameters {
        h_root: root,
        h_object_parent: parent,
        h_object_new: handle,
        h_class: class,
        p_alloc_parms: params_ptr,
        params_size,
        status: STATUS_SENTINEL,
    };
    let rc = unsafe { libc::ioctl(fd, RM_ALLOC_CMD, &mut p as *mut Nvos64Parameters) };
    let errno = if rc < 0 {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
    } else {
        0
    };
    eprintln!(
        "  RM_ALLOC(cls=0x{:04x}, h=0x{:08x}): rc={} errno={} status=0x{:x}",
        class, handle, rc, errno, p.status
    );
    (rc, p.status)
}

/// Issue NV_ESC_RM_CONTROL. Returns (ioctl_rc, rm_status).
fn rm_ctrl(
    fd: std::os::fd::RawFd,
    client: u32,
    object: u32,
    cmd: u32,
    params_ptr: u64,
    params_size: u32,
) -> (i32, u32) {
    let mut p = Nvos54Parameters {
        h_client: client,
        h_object: object,
        cmd,
        params: params_ptr,
        params_size,
        status: STATUS_SENTINEL,
        ..Default::default()
    };
    let rc = unsafe { libc::ioctl(fd, RM_CTRL_CMD, &mut p as *mut Nvos54Parameters) };
    let errno = if rc < 0 {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
    } else {
        0
    };
    eprintln!(
        "  RM_CTRL(cmd=0x{:08x}, obj=0x{:08x}): rc={} errno={} status=0x{:x}",
        cmd, object, rc, errno, p.status
    );
    (rc, p.status)
}

fn step_json(name: &str, ok: bool, detail: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"step": name, "ok": ok, "detail": detail})
}

fn print_result(result: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(result).unwrap_or_default());
}

fn cleanup(ctl_path: &str, gpu_path: &str) {
    let _ = std::fs::remove_file(ctl_path);
    let _ = std::fs::remove_file(gpu_path);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <chardev_major> [--channel]", args[0]);
        return ExitCode::from(1);
    }

    let major: u32 = match args[1].parse() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Invalid major number '{}': {e}", args[1]);
            return ExitCode::from(1);
        }
    };

    let channel_mode = args.iter().any(|a| a == "--channel");

    eprintln!("rm_trigger: major={major}, channel_mode={channel_mode}");
    eprintln!("sizeof(Nvos64Parameters) = {}", size_of::<Nvos64Parameters>());
    eprintln!("sizeof(NvChannelAllocParams) = {}", size_of::<NvChannelAllocParams>());
    eprintln!("RM_ALLOC_CMD = 0x{RM_ALLOC_CMD:x}");
    eprintln!("RM_CTRL_CMD  = 0x{RM_CTRL_CMD:x}");

    let ctl_path = "/dev/toadstool-rm-nvidiactl";
    let gpu_path = "/dev/toadstool-rm-nvidia0";

    let _ = std::fs::remove_file(ctl_path);
    let _ = std::fs::remove_file(gpu_path);

    let mode = rustix::fs::Mode::from_raw_mode(0o666);
    let char_type = rustix::fs::FileType::CharacterDevice;

    if let Err(e) = rustix::fs::mknodat(
        rustix::fs::CWD, ctl_path, char_type, mode,
        rustix::fs::makedev(major, 255),
    ) {
        eprintln!("mknod ctl: {e}");
        return ExitCode::from(1);
    }

    if let Err(e) = rustix::fs::mknodat(
        rustix::fs::CWD, gpu_path, char_type, mode,
        rustix::fs::makedev(major, 0),
    ) {
        eprintln!("mknod gpu: {e}");
        let _ = std::fs::remove_file(ctl_path);
        return ExitCode::from(1);
    }

    let mut steps = Vec::new();
    let mut success = true;
    let mut channel_id: Option<u32> = None;
    let mut work_submit_token: Option<u32> = None;

    // ── Open GPU (triggers rm_init_adapter) ──────────────────────────────
    eprintln!("\nOpening GPU device (minor 0) to trigger rm_init_adapter...");
    let gpu_fd = std::fs::OpenOptions::new().read(true).write(true).open(gpu_path);
    match &gpu_fd {
        Ok(f) => eprintln!("  GPU open ok (fd={})", f.as_raw_fd()),
        Err(e) => {
            eprintln!("  GPU open failed: {e}");
            success = false;
        }
    }

    // ── Open nvidiactl ──────────────────────────────────────────────────
    eprintln!("\nOpening nvidiactl (minor 255)...");
    let ctl_file = match std::fs::OpenOptions::new().read(true).write(true).open(ctl_path) {
        Ok(f) => {
            eprintln!("  ctl open ok (fd={})", f.as_raw_fd());
            f
        }
        Err(e) => {
            eprintln!("  ctl open failed: {e}");
            steps.push(step_json("open_ctl", false, serde_json::json!({"error": e.to_string()})));
            cleanup(ctl_path, gpu_path);
            print_result(&serde_json::json!({"success": false, "major": major, "steps": steps}));
            return ExitCode::from(1);
        }
    };
    let fd = ctl_file.as_raw_fd();

    // ═══════════════════════════════════════════════════════════════════
    // Phase 1: Core object tree (root → device → subdevice)
    // ═══════════════════════════════════════════════════════════════════

    // Step 1: root client (0x0000 works on 470; 0x0041 is canonical but may
    // require newer RM — stick with what's HW-validated)
    eprintln!("\n[Phase 1] Step 1: root client...");
    let (_, status) = rm_alloc(fd, 0, 0, H_ROOT, 0x0000, 0, 0);
    steps.push(step_json("root_alloc", status == 0, serde_json::json!({"class": "0x0000", "status": format!("0x{status:x}")})));
    if status != 0 { success = false; }

    // Step 2: device
    if success {
        eprintln!("\n[Phase 1] Step 2: device...");
        let mut dp = rm_abi::Nv0080AllocParams::default();
        dp.device_id = 0;
        let (_, status) = rm_alloc(fd, H_ROOT, H_ROOT, H_DEVICE, class::NV01_DEVICE_0,
            &dp as *const _ as u64, size_of::<rm_abi::Nv0080AllocParams>() as u32);
        steps.push(step_json("device_alloc", status == 0, serde_json::json!({"class": "NV01_DEVICE_0", "status": format!("0x{status:x}")})));
        if status != 0 { success = false; }
    }

    // Step 3: subdevice
    if success {
        eprintln!("\n[Phase 1] Step 3: subdevice...");
        let mut sp = rm_abi::Nv2080AllocParams::default();
        sp.sub_device_id = 0;
        let (_, status) = rm_alloc(fd, H_ROOT, H_DEVICE, H_SUBDEVICE, class::NV20_SUBDEVICE_0,
            &sp as *const _ as u64, size_of::<rm_abi::Nv2080AllocParams>() as u32);
        steps.push(step_json("subdevice_alloc", status == 0, serde_json::json!({"class": "NV20_SUBDEVICE_0", "status": format!("0x{status:x}")})));
        if status != 0 { success = false; }
    }

    // Step 4: GR_GET_INFO (triggers full GR init)
    if success {
        eprintln!("\n[Phase 1] Step 4: GR_GET_INFO...");
        let (_, status) = rm_ctrl(fd, H_ROOT, H_SUBDEVICE, 0x2080_1201, 0, 0);
        steps.push(step_json("gr_get_info", true, serde_json::json!({"cmd": "GR_GET_INFO", "status": format!("0x{status:x}")})));
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 2+: Full channel creation (Exp 229, --channel mode)
    // ═══════════════════════════════════════════════════════════════════

    if success && channel_mode {
        eprintln!("\n════ Channel mode: creating full RM compute channel ════\n");

        // Step 5: VA space
        eprintln!("[Phase 2] Step 5: VA space (FERMI_VASPACE_A)...");
        let mut vp = NvVaspaceAllocParams::default();
        vp.flags = 0; // no faulting — we don't have UVM loaded
        let (_, status) = rm_alloc(fd, H_ROOT, H_DEVICE, H_VASPACE, class::FERMI_VASPACE_A,
            &vp as *const _ as u64, size_of::<NvVaspaceAllocParams>() as u32);
        steps.push(step_json("vaspace_alloc", status == 0, serde_json::json!({"class": "FERMI_VASPACE_A", "status": format!("0x{status:x}")})));
        if status != 0 { success = false; }

        // Step 6: USERD memory (sysmem — VRAM alloc requires more params,
        // try sysmem first; if RM rejects, we'll iterate)
        if success {
            eprintln!("[Phase 2] Step 6: USERD memory (NV01_MEMORY_SYSTEM)...");
            let mut mp = NvMemoryAllocParams::default();
            mp.owner = H_ROOT;
            mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
            mp.size = 4096;
            mp.alignment = 4096;
            let (_, status) = rm_alloc(fd, H_ROOT, H_DEVICE, H_MEM_USERD, class::NV01_MEMORY_SYSTEM,
                &mp as *const _ as u64, size_of::<NvMemoryAllocParams>() as u32);
            steps.push(step_json("userd_mem_alloc", status == 0, serde_json::json!({"class": "NV01_MEMORY_SYSTEM", "size": 4096, "status": format!("0x{status:x}")})));
            if status != 0 { success = false; }
        }

        // Step 7: GPFIFO ring memory (sysmem)
        if success {
            eprintln!("[Phase 2] Step 7: GPFIFO ring memory...");
            let mut mp = NvMemoryAllocParams::default();
            mp.owner = H_ROOT;
            mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
            mp.size = 4096;
            mp.alignment = 4096;
            let (_, status) = rm_alloc(fd, H_ROOT, H_DEVICE, H_MEM_GPFIFO, class::NV01_MEMORY_SYSTEM,
                &mp as *const _ as u64, size_of::<NvMemoryAllocParams>() as u32);
            steps.push(step_json("gpfifo_mem_alloc", status == 0, serde_json::json!({"class": "NV01_MEMORY_SYSTEM", "size": 4096, "status": format!("0x{status:x}")})));
            if status != 0 { success = false; }
        }

        // Step 8: Error notifier memory (sysmem, type=13, owner=device)
        if success {
            eprintln!("[Phase 2] Step 8: Error notifier memory...");
            let mut mp = NvMemoryAllocParams::default();
            mp.owner = H_DEVICE;
            mp.mem_type = 13; // NOTIFIER
            mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
            mp.size = 4096;
            mp.alignment = 4096;
            let (_, status) = rm_alloc(fd, H_ROOT, H_DEVICE, H_MEM_ERR_NOTIFIER, class::NV01_MEMORY_SYSTEM,
                &mp as *const _ as u64, size_of::<NvMemoryAllocParams>() as u32);
            steps.push(step_json("err_notifier_mem_alloc", status == 0, serde_json::json!({"class": "NV01_MEMORY_SYSTEM", "mem_type": 13, "status": format!("0x{status:x}")})));
            if status != 0 { success = false; }
        }

        // Step 9: TSG (Kepler channel group)
        if success {
            eprintln!("[Phase 3] Step 9: TSG (KEPLER_CHANNEL_GROUP_A)...");
            let mut tsg = NvChannelGroupAllocParams::default();
            tsg.h_object_error = H_MEM_ERR_NOTIFIER;
            tsg.h_vaspace = H_VASPACE;
            tsg.engine_type = NV2080_ENGINE_TYPE_GR0;
            let (_, status) = rm_alloc(fd, H_ROOT, H_DEVICE, H_TSG, class::KEPLER_CHANNEL_GROUP_A,
                &tsg as *const _ as u64, size_of::<NvChannelGroupAllocParams>() as u32);
            steps.push(step_json("tsg_alloc", status == 0, serde_json::json!({"class": "KEPLER_CHANNEL_GROUP_A", "status": format!("0x{status:x}")})));
            if status != 0 { success = false; }
        }

        // Step 10: Context share (under TSG)
        if success {
            eprintln!("[Phase 3] Step 10: Context share (FERMI_CONTEXT_SHARE_A)...");
            let mut cs = NvCtxShareAllocParams::default();
            cs.h_vaspace = H_VASPACE;
            cs.h_subdevice = H_SUBDEVICE;
            let (_, status) = rm_alloc(fd, H_ROOT, H_TSG, H_CTX_SHARE, class::FERMI_CONTEXT_SHARE_A,
                &cs as *const _ as u64, size_of::<NvCtxShareAllocParams>() as u32);
            steps.push(step_json("ctx_share_alloc", status == 0, serde_json::json!({"class": "FERMI_CONTEXT_SHARE_A", "status": format!("0x{status:x}")})));
            if status != 0 { success = false; }
        }

        // Step 11: GPFIFO channel (VOLTA_CHANNEL_GPFIFO_A, under TSG)
        if success {
            eprintln!("[Phase 3] Step 11: GPFIFO channel (VOLTA_CHANNEL_GPFIFO_A)...");
            let mut ch = NvChannelAllocParams::default();
            ch.h_object_error = H_MEM_ERR_NOTIFIER;
            ch.h_object_buffer = H_MEM_GPFIFO;
            ch.gpfifo_entries = 64; // 64 entries = 512 bytes
            ch.h_context_share = H_CTX_SHARE;
            ch.h_vaspace = H_VASPACE;
            ch.h_userd_memory[0] = H_MEM_USERD;
            ch.engine_type = NV2080_ENGINE_TYPE_GR0;

            let (_, status) = rm_alloc(fd, H_ROOT, H_TSG, H_CHANNEL, class::VOLTA_CHANNEL_GPFIFO_A,
                &ch as *const _ as u64, size_of::<NvChannelAllocParams>() as u32);

            let ch_ok = status == 0;
            if ch_ok {
                channel_id = Some(ch.cid);
                eprintln!("  Channel allocated, cid={}", ch.cid);
            }
            steps.push(step_json("channel_alloc", ch_ok, serde_json::json!({
                "class": "VOLTA_CHANNEL_GPFIFO_A",
                "status": format!("0x{status:x}"),
                "channel_id": ch.cid,
            })));
            if !ch_ok { success = false; }
        }

        // Step 12: Compute engine object (under channel)
        if success {
            eprintln!("[Phase 3] Step 12: Compute engine (VOLTA_COMPUTE_A)...");
            let (_, status) = rm_alloc(fd, H_ROOT, H_CHANNEL, H_COMPUTE, class::VOLTA_COMPUTE_A, 0, 0);
            steps.push(step_json("compute_alloc", status == 0, serde_json::json!({"class": "VOLTA_COMPUTE_A", "status": format!("0x{status:x}")})));
            if status != 0 { success = false; }
        }

        // Step 13: BIND channel to GR engine
        if success {
            eprintln!("[Phase 4] Step 13: BIND channel to GR...");
            let mut bp = NvChannelBindParams::default();
            bp.h_engine_object = H_COMPUTE;
            bp.engine_class_1 = class::VOLTA_COMPUTE_A;
            bp.engine_class_2 = class::VOLTA_COMPUTE_A;
            bp.engine_type = NV2080_ENGINE_TYPE_GR0;
            let (_, status) = rm_ctrl(fd, H_ROOT, H_CHANNEL, rm_abi::NV906F_CTRL_CMD_BIND,
                &bp as *const _ as u64, size_of::<NvChannelBindParams>() as u32);
            steps.push(step_json("channel_bind", status == 0, serde_json::json!({"cmd": "BIND", "status": format!("0x{status:x}")})));
            if status != 0 { success = false; }
        }

        // Step 14: SCHEDULE TSG (on TSG handle, not channel!)
        if success {
            eprintln!("[Phase 4] Step 14: SCHEDULE TSG...");
            let mut sp = NvGpfifoScheduleParams { b_enable: 1 };
            let (_, status) = rm_ctrl(fd, H_ROOT, H_TSG, rm_abi::NVA06C_CTRL_CMD_GPFIFO_SCHEDULE,
                &mut sp as *mut _ as u64, size_of::<NvGpfifoScheduleParams>() as u32);
            steps.push(step_json("tsg_schedule", status == 0, serde_json::json!({"cmd": "GPFIFO_SCHEDULE", "target": "TSG", "status": format!("0x{status:x}")})));
            if status != 0 { success = false; }
        }

        // Step 15: GET_WORK_SUBMIT_TOKEN (on channel, Volta class cmd)
        if success {
            eprintln!("[Phase 4] Step 15: GET_WORK_SUBMIT_TOKEN...");
            let mut tp = NvA06fGetWorkSubmitTokenParams::default();
            let (_, status) = rm_ctrl(fd, H_ROOT, H_CHANNEL, rm_abi::NVA06F_CTRL_CMD_GPFIFO_GET_WORK_SUBMIT_TOKEN,
                &mut tp as *mut _ as u64, size_of::<NvA06fGetWorkSubmitTokenParams>() as u32);
            if status == 0 {
                work_submit_token = Some(tp.work_submit_token);
                eprintln!("  work_submit_token = 0x{:08x}", tp.work_submit_token);
            }
            steps.push(step_json("work_submit_token", status == 0, serde_json::json!({
                "cmd": "GET_WORK_SUBMIT_TOKEN",
                "status": format!("0x{status:x}"),
                "token": format!("0x{:08x}", tp.work_submit_token),
            })));
            if status != 0 { success = false; }
        }
    }

    // Hold fds open for RM async work (shorter in channel mode since we
    // already did the full alloc sequence)
    let hold_secs = if channel_mode { 3 } else { 5 };
    eprintln!("\nHolding fds open for {hold_secs}s...");
    std::thread::sleep(std::time::Duration::from_secs(hold_secs));
    eprintln!("Done.");

    drop(ctl_file);
    drop(gpu_fd);
    cleanup(ctl_path, gpu_path);

    let mut result = serde_json::json!({
        "success": success,
        "major": major,
        "channel_mode": channel_mode,
        "steps": steps,
    });

    if channel_mode {
        result["channel_id"] = serde_json::json!(channel_id);
        result["work_submit_token"] = match work_submit_token {
            Some(t) => serde_json::json!(format!("0x{t:08x}")),
            None => serde_json::json!(null),
        };
        result["handles"] = serde_json::json!({
            "root": format!("0x{H_ROOT:08x}"),
            "device": format!("0x{H_DEVICE:08x}"),
            "subdevice": format!("0x{H_SUBDEVICE:08x}"),
            "vaspace": format!("0x{H_VASPACE:08x}"),
            "tsg": format!("0x{H_TSG:08x}"),
            "ctx_share": format!("0x{H_CTX_SHARE:08x}"),
            "channel": format!("0x{H_CHANNEL:08x}"),
            "compute": format!("0x{H_COMPUTE:08x}"),
            "mem_userd": format!("0x{H_MEM_USERD:08x}"),
            "mem_gpfifo": format!("0x{H_MEM_GPFIFO:08x}"),
            "mem_err_notifier": format!("0x{H_MEM_ERR_NOTIFIER:08x}"),
        });
    }

    print_result(&result);
    ExitCode::SUCCESS
}
