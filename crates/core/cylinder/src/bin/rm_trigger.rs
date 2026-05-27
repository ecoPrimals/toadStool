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

/// nvidia-470 uses NVOS21 (28 bytes) for NV_ESC_RM_ALLOC (0x2B).
/// Status is at offset 24 — NOT at offset 28 like in NVOS64 from 510+.
/// NVOS21 has no `params_size` field; RM infers size from hClass.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Nvos21Parameters {
    h_root: u32,           // 0
    h_object_parent: u32,  // 4
    h_object_new: u32,     // 8
    h_class: u32,          // 12
    p_alloc_parms: u64,    // 16
    status: u32,           // 24 — the REAL status field on 470.x
    _pad: u32,             // 28 — alignment padding (never used by kernel)
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

const RM_ALLOC_CMD: u64 = iowr(NV_IOCTL_MAGIC, NV_ESC_RM_ALLOC, size_of::<Nvos21Parameters>());
const RM_CTRL_CMD: u64 = iowr(NV_IOCTL_MAGIC, NV_ESC_RM_CONTROL, size_of::<Nvos54Parameters>());

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

/// Result of an RM_ALLOC ioctl.
struct RmAllocResult {
    rc: i32,
    status: u32,
    /// The handle actually assigned by the kernel (may differ from requested).
    h_object_new: u32,
}

/// Issue NV_ESC_RM_ALLOC via raw 32-byte buffer.
///
/// nvidia-470 RM may REWRITE h_object_new with its own RM-assigned handle.
/// Callers MUST use the returned `h_object_new` for subsequent operations.
fn rm_alloc(
    fd: std::os::fd::RawFd,
    root: u32,
    parent: u32,
    handle: u32,
    class: u32,
    params_ptr: u64,
    params_size: u32,
) -> RmAllocResult {
    let mut buf = [0xDDu8; 32];
    buf[0..4].copy_from_slice(&root.to_ne_bytes());
    buf[4..8].copy_from_slice(&parent.to_ne_bytes());
    buf[8..12].copy_from_slice(&handle.to_ne_bytes());
    buf[12..16].copy_from_slice(&class.to_ne_bytes());
    buf[16..24].copy_from_slice(&params_ptr.to_ne_bytes());
    let sentinel_24: u32 = if params_size > 0 { params_size } else { 0xAAAA_AAAA };
    buf[24..28].copy_from_slice(&sentinel_24.to_ne_bytes());
    buf[28..32].copy_from_slice(&0xDEAD_BEEFu32.to_ne_bytes());

    let rc = unsafe { libc::ioctl(fd, RM_ALLOC_CMD, buf.as_mut_ptr()) };
    let errno = if rc < 0 {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
    } else {
        0
    };

    let h_new_out = u32::from_ne_bytes(buf[8..12].try_into().unwrap());
    let val_24 = u32::from_ne_bytes(buf[24..28].try_into().unwrap());
    let val_28 = u32::from_ne_bytes(buf[28..32].try_into().unwrap());
    let status = if val_28 != 0xDEAD_BEEF { val_28 } else { val_24 };

    eprintln!(
        "  RM_ALLOC(cls=0x{:04x}, h=0x{:08x}→0x{:08x}): rc={} errno={} status=0x{:x}",
        class, handle, h_new_out, rc, errno, status
    );

    RmAllocResult { rc, status, h_object_new: h_new_out }
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
        status: 0,
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
    eprintln!("sizeof(Nvos21Parameters) = {}", size_of::<Nvos21Parameters>());
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
    // Phase 0: RM dispatch diagnostics — test if RM is alive
    // ═══════════════════════════════════════════════════════════════════

    // NV_ESC_REGISTER_FD (201/0xC9) — associates a GPU fd with a ctl fd.
    // On nvidia-470, this is how the control channel gets linked to a device.
    // The parameter is nv_ioctl_register_fd_t = { NvS32 ctl_fd; } = 4 bytes.
    // nvidia uses _IOWR for all escapes, but the NR encodes the size.
    // Try all possible ioctl encodings to find the right one.
    if let Ok(ref gpu_f) = gpu_fd {
        let gpu_raw = gpu_f.as_raw_fd();
        eprintln!("\n[Diag] NV_ESC_REGISTER_FD: linking GPU fd={gpu_raw} to ctl fd={fd}...");

        // The parameter struct for REGISTER_FD on nvidia-470 is:
        //   struct nv_ioctl_register_fd_t { NvS32 ctl_fd; };
        // This ioctl is called on the GPU fd (not the ctl fd!).
        // It tells the GPU fd which ctl fd to use for RM operations.
        for (label, target_fd, param_fd) in [
            ("gpu→ctl", gpu_raw, fd),
            ("ctl→gpu", fd, gpu_raw),
        ] {
            let mut reg_fd_buf = [0u8; 4];
            reg_fd_buf[0..4].copy_from_slice(&(param_fd as u32).to_ne_bytes());
            const NV_ESC_REGISTER_FD_NR: u8 = 201;
            let reg_fd_cmd: u64 = iowr(NV_IOCTL_MAGIC, NV_ESC_REGISTER_FD_NR, 4);
            let rc = unsafe { libc::ioctl(target_fd, reg_fd_cmd, reg_fd_buf.as_mut_ptr()) };
            let errno = if rc < 0 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) } else { 0 };
            eprintln!("[Diag] REGISTER_FD({label}): rc={rc} errno={errno}");
            if rc == 0 {
                steps.push(step_json("register_fd", true, serde_json::json!({
                    "rc": rc, "direction": label, "target_fd": target_fd, "param_fd": param_fd
                })));
                break;
            }
        }

        // If both failed, also try the _IOW variant (write-only, not read-write)
        {
            let mut reg_fd_buf = [0u8; 4];
            reg_fd_buf[0..4].copy_from_slice(&(fd as u32).to_ne_bytes());
            const NV_ESC_REGISTER_FD_NR: u8 = 201;
            let iow_cmd: u64 = {
                let dir: u64 = 1; // _IOC_WRITE only
                (dir << 30) | ((4u64 & 0x3FFF) << 16) | ((NV_IOCTL_MAGIC as u64) << 8) | NV_ESC_REGISTER_FD_NR as u64
            };
            let rc = unsafe { libc::ioctl(gpu_raw, iow_cmd, reg_fd_buf.as_mut_ptr()) };
            let errno = if rc < 0 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) } else { 0 };
            eprintln!("[Diag] REGISTER_FD(_IOW, gpu→ctl): rc={rc} errno={errno}");
        }

        // And try with no direction (bare ioctl number)
        {
            let mut reg_fd_buf = [0u8; 4];
            reg_fd_buf[0..4].copy_from_slice(&(fd as u32).to_ne_bytes());
            const NV_ESC_REGISTER_FD_NR: u8 = 201;
            let bare_cmd: u64 = ((NV_IOCTL_MAGIC as u64) << 8) | NV_ESC_REGISTER_FD_NR as u64;
            let rc = unsafe { libc::ioctl(gpu_raw, bare_cmd, reg_fd_buf.as_mut_ptr()) };
            let errno = if rc < 0 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) } else { 0 };
            eprintln!("[Diag] REGISTER_FD(bare, gpu→ctl): rc={rc} errno={errno}");
        }

        steps.push(step_json("register_fd_probes", true, serde_json::json!({
            "note": "tried multiple ioctl encodings"
        })));
    }

    // NV_ESC_CHECK_VERSION_STR (0x23) — nvidia-470 uses nv_rm_api_version_t
    // which is { u32 cmd, u32 reply, char version[NV_RM_API_VERSION_STRING_LENGTH] }.
    // Try both 64-byte and 128-byte string lengths (72 and 136 total).
    for (label, total_size) in [("72B", 72usize), ("136B", 136), ("64B", 64)] {
        const NV_ESC_CHECK_VERSION_STR: u8 = 0x23;
        let check_ver_cmd: u64 = iowr(NV_IOCTL_MAGIC, NV_ESC_CHECK_VERSION_STR, total_size);
        let mut ver_buf = vec![0u8; total_size];
        let rc = unsafe { libc::ioctl(fd, check_ver_cmd, ver_buf.as_mut_ptr()) };
        let errno = if rc < 0 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) } else { 0 };
        let ver_str = if total_size > 8 {
            std::str::from_utf8(&ver_buf[8..]).unwrap_or("(invalid)")
                .trim_end_matches('\0')
        } else {
            ""
        };
        let nonzero = ver_buf.iter().filter(|&&b| b != 0).count();
        eprintln!("[Diag] CHECK_VERSION({label}): rc={rc} errno={errno} nonzero={nonzero} ver=\"{ver_str}\"");
        if rc == 0 {
            steps.push(step_json("check_version", true, serde_json::json!({
                "rc": rc, "size": total_size, "version": ver_str, "nonzero": nonzero
            })));
            break;
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 0b: Root client class comparison — NV01_ROOT (0x0) vs NV01_ROOT_CLIENT (0x41)
    //
    // nvidia-470 may only support class 0x0000 for root allocation.
    // Test both and compare: which one creates a REAL object?
    // ═══════════════════════════════════════════════════════════════════

    const H_TEST_ROOT_00: u32 = 0xBEEF_0000;
    const H_TEST_ROOT_41: u32 = 0xBEEF_0041;

    // ═══════════════════════════════════════════════════════════════════
    // Phase 0b: Root client test — kernel REWRITES h_object_new!
    //
    // DISCOVERY: nvidia-470 RM ignores the user-supplied handle and assigns
    // its own (e.g. 0xc1d00007). We MUST use the kernel-assigned handle.
    // ═══════════════════════════════════════════════════════════════════

    eprintln!("\n[Phase 0b] Allocating root client (class 0x0000)...");
    let root_test = rm_alloc(fd, 0, 0, H_ROOT, class::NV01_ROOT, 0, 0);
    let rm_root = root_test.h_object_new;
    eprintln!("  Kernel assigned root handle: 0x{rm_root:08x} (we asked for 0x{H_ROOT:08x})");

    // Test RM_CTRL with the KERNEL-assigned handle
    let mut gpu_ids = [0u32; 32];
    let (ctrl_rc, ctrl_st) = rm_ctrl(fd, rm_root, rm_root, 0x0000_0201,
        gpu_ids.as_mut_ptr() as u64, size_of::<[u32; 32]>() as u32);
    let attached: Vec<u32> = gpu_ids.iter().copied().filter(|&id| id != 0 && id != 0xFFFF_FFFF).collect();
    eprintln!("[Phase 0b] GPU_GET_ATTACHED_IDS(h=0x{rm_root:08x}): status=0x{ctrl_st:x} ids={attached:?}");

    // Also try with our original handle for comparison
    let mut gpu_ids_orig = [0u32; 32];
    let (_, ctrl_st_orig) = rm_ctrl(fd, H_ROOT, H_ROOT, 0x0000_0201,
        gpu_ids_orig.as_mut_ptr() as u64, size_of::<[u32; 32]>() as u32);
    eprintln!("[Phase 0b] GPU_GET_ATTACHED_IDS(h=0x{H_ROOT:08x}): status=0x{ctrl_st_orig:x} ← WRONG handle for comparison");

    steps.push(step_json("root_handle_discovery", root_test.status == 0, serde_json::json!({
        "requested_handle": format!("0x{H_ROOT:08x}"),
        "kernel_assigned": format!("0x{rm_root:08x}"),
        "ctrl_with_kernel_handle": format!("0x{ctrl_st:x}"),
        "ctrl_with_user_handle": format!("0x{ctrl_st_orig:x}"),
        "attached_ids": attached,
    })));

    // ═══════════════════════════════════════════════════════════════════
    // Phase 1: Core object tree using KERNEL-ASSIGNED handles
    // ═══════════════════════════════════════════════════════════════════

    let root_ok = root_test.rc == 0 && root_test.status == 0;
    if !root_ok { success = false; }

    // Step 2: device — use rm_root as both h_root and h_parent
    // Try with GPU_ID from GET_ATTACHED_IDS if available.
    let mut rm_device = 0u32;
    if root_ok {
        // First get the GPU ID
        let mut gpu_ids = [0u32; 32];
        rm_ctrl(fd, rm_root, rm_root, 0x0000_0201,
            gpu_ids.as_mut_ptr() as u64, size_of::<[u32; 32]>() as u32);
        let gpu_id = gpu_ids.iter().copied().find(|&id| id != 0 && id != 0xFFFF_FFFF).unwrap_or(0);

        eprintln!("\n[Phase 1] Step 2: device (parent=0x{rm_root:08x}, gpu_id=0x{gpu_id:x})...");
        let mut dp = rm_abi::Nv0080AllocParams::default();
        dp.device_id = gpu_id;
        dp.h_client_share = rm_root;
        let r = rm_alloc(fd, rm_root, rm_root, H_DEVICE, class::NV01_DEVICE_0,
            &dp as *const _ as u64, size_of::<rm_abi::Nv0080AllocParams>() as u32);
        rm_device = r.h_object_new;
        let dev_ok = r.rc == 0 && r.status == 0;
        steps.push(step_json("device_alloc", dev_ok, serde_json::json!({
            "class": "NV01_DEVICE_0",
            "status": format!("0x{:x}", r.status),
            "kernel_handle": format!("0x{rm_device:08x}"),
            "device_id": format!("0x{gpu_id:x}"),
        })));
        if !dev_ok {
            // Retry with device_id=0
            eprintln!("[Diag] Retrying device alloc with device_id=0...");
            let dp0 = rm_abi::Nv0080AllocParams::default();
            let r2 = rm_alloc(fd, rm_root, rm_root, H_DEVICE + 1, class::NV01_DEVICE_0,
                &dp0 as *const _ as u64, size_of::<rm_abi::Nv0080AllocParams>() as u32);
            eprintln!("[Diag] device_alloc(id=0): status=0x{:x}", r2.status);

            // Try with NULL params (let RM use defaults)
            eprintln!("[Diag] Retrying device alloc with NULL params...");
            let r3 = rm_alloc(fd, rm_root, rm_root, H_DEVICE + 2, class::NV01_DEVICE_0, 0, 0);
            eprintln!("[Diag] device_alloc(null): status=0x{:x}", r3.status);

            steps.push(step_json("device_alloc_retries", true, serde_json::json!({
                "id0_status": format!("0x{:x}", r2.status),
                "null_status": format!("0x{:x}", r3.status),
            })));

            // Use whichever succeeded
            if r2.status == 0 {
                rm_device = r2.h_object_new;
            } else if r3.status == 0 {
                rm_device = r3.h_object_new;
            } else {
                success = false;
            }
        }
    }

    // Step 3: subdevice
    let mut rm_subdevice = 0u32;
    if root_ok && success {
        eprintln!("\n[Phase 1] Step 3: subdevice (parent=0x{rm_device:08x})...");
        let mut sp = rm_abi::Nv2080AllocParams::default();
        sp.sub_device_id = 0;
        let r = rm_alloc(fd, rm_root, rm_device, H_SUBDEVICE, class::NV20_SUBDEVICE_0,
            &sp as *const _ as u64, size_of::<rm_abi::Nv2080AllocParams>() as u32);
        rm_subdevice = r.h_object_new;
        let sub_ok = r.rc == 0 && r.status == 0;
        steps.push(step_json("subdevice_alloc", sub_ok, serde_json::json!({
            "class": "NV20_SUBDEVICE_0",
            "status": format!("0x{:x}", r.status),
            "kernel_handle": format!("0x{rm_subdevice:08x}"),
        })));
        if !sub_ok { success = false; }
    }

    // Step 4: GR_GET_INFO (triggers full GR init)
    if root_ok && success {
        eprintln!("\n[Phase 1] Step 4: GR_GET_INFO (obj=0x{rm_subdevice:08x})...");
        let (rc, status) = rm_ctrl(fd, rm_root, rm_subdevice, 0x2080_1201, 0, 0);
        steps.push(step_json("gr_get_info", rc == 0, serde_json::json!({
            "cmd": "GR_GET_INFO",
            "status": format!("0x{status:x}"),
        })));
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 2+: Full channel creation (Exp 229, --channel mode)
    // ═══════════════════════════════════════════════════════════════════

    if root_ok && channel_mode {
        eprintln!("\n════ Channel mode: full RM compute channel (kernel-assigned handles) ════\n");

        // Step 5: VA space
        let mut rm_vaspace = 0u32;
        {
            eprintln!("[Phase 2] Step 5: VA space (FERMI_VASPACE_A)...");
            let mut vp = NvVaspaceAllocParams::default();
            vp.flags = 0;
            let r = rm_alloc(fd, rm_root, rm_device, H_VASPACE, class::FERMI_VASPACE_A,
                &vp as *const _ as u64, size_of::<NvVaspaceAllocParams>() as u32);
            rm_vaspace = r.h_object_new;
            let va_ok = r.rc == 0 && r.status == 0;
            steps.push(step_json("vaspace_alloc", va_ok, serde_json::json!({
                "class": "FERMI_VASPACE_A", "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_vaspace:08x}"),
            })));
            if !va_ok { success = false; }
        }

        // Step 6: USERD memory
        let mut rm_userd = 0u32;
        if success {
            eprintln!("[Phase 2] Step 6: USERD memory...");
            let mut mp = NvMemoryAllocParams::default();
            mp.owner = rm_root;
            mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
            mp.size = 4096;
            mp.alignment = 4096;
            let r = rm_alloc(fd, rm_root, rm_device, H_MEM_USERD, class::NV01_MEMORY_SYSTEM,
                &mp as *const _ as u64, size_of::<NvMemoryAllocParams>() as u32);
            rm_userd = r.h_object_new;
            let ok = r.rc == 0 && r.status == 0;
            steps.push(step_json("userd_mem_alloc", ok, serde_json::json!({
                "class": "NV01_MEMORY_SYSTEM", "size": 4096,
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_userd:08x}"),
            })));
            if !ok { success = false; }
        }

        // Step 7: GPFIFO ring memory
        let mut rm_gpfifo_mem = 0u32;
        if success {
            eprintln!("[Phase 2] Step 7: GPFIFO ring memory...");
            let mut mp = NvMemoryAllocParams::default();
            mp.owner = rm_root;
            mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
            mp.size = 4096;
            mp.alignment = 4096;
            let r = rm_alloc(fd, rm_root, rm_device, H_MEM_GPFIFO, class::NV01_MEMORY_SYSTEM,
                &mp as *const _ as u64, size_of::<NvMemoryAllocParams>() as u32);
            rm_gpfifo_mem = r.h_object_new;
            let ok = r.rc == 0 && r.status == 0;
            steps.push(step_json("gpfifo_mem_alloc", ok, serde_json::json!({
                "class": "NV01_MEMORY_SYSTEM", "size": 4096,
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_gpfifo_mem:08x}"),
            })));
            if !ok { success = false; }
        }

        // Step 8: Error notifier memory
        let mut rm_err_notifier = 0u32;
        if success {
            eprintln!("[Phase 2] Step 8: Error notifier memory...");
            let mut mp = NvMemoryAllocParams::default();
            mp.owner = rm_device;
            mp.mem_type = 13;
            mp.flags = NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED;
            mp.size = 4096;
            mp.alignment = 4096;
            let r = rm_alloc(fd, rm_root, rm_device, H_MEM_ERR_NOTIFIER, class::NV01_MEMORY_SYSTEM,
                &mp as *const _ as u64, size_of::<NvMemoryAllocParams>() as u32);
            rm_err_notifier = r.h_object_new;
            let ok = r.rc == 0 && r.status == 0;
            steps.push(step_json("err_notifier_mem_alloc", ok, serde_json::json!({
                "class": "NV01_MEMORY_SYSTEM", "mem_type": 13,
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_err_notifier:08x}"),
            })));
            if !ok { success = false; }
        }

        // Step 9: TSG
        let mut rm_tsg = 0u32;
        if success {
            eprintln!("[Phase 3] Step 9: TSG...");
            let mut tsg = NvChannelGroupAllocParams::default();
            tsg.h_object_error = rm_err_notifier;
            tsg.h_vaspace = rm_vaspace;
            tsg.engine_type = NV2080_ENGINE_TYPE_GR0;
            let r = rm_alloc(fd, rm_root, rm_device, H_TSG, class::KEPLER_CHANNEL_GROUP_A,
                &tsg as *const _ as u64, size_of::<NvChannelGroupAllocParams>() as u32);
            rm_tsg = r.h_object_new;
            let ok = r.rc == 0 && r.status == 0;
            steps.push(step_json("tsg_alloc", ok, serde_json::json!({
                "class": "KEPLER_CHANNEL_GROUP_A",
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_tsg:08x}"),
            })));
            if !ok { success = false; }
        }

        // Step 10: Context share
        let mut rm_ctx_share = 0u32;
        if success {
            eprintln!("[Phase 3] Step 10: Context share...");
            let mut cs = NvCtxShareAllocParams::default();
            cs.h_vaspace = rm_vaspace;
            cs.h_subdevice = rm_subdevice;
            let r = rm_alloc(fd, rm_root, rm_tsg, H_CTX_SHARE, class::FERMI_CONTEXT_SHARE_A,
                &cs as *const _ as u64, size_of::<NvCtxShareAllocParams>() as u32);
            rm_ctx_share = r.h_object_new;
            let ok = r.rc == 0 && r.status == 0;
            steps.push(step_json("ctx_share_alloc", ok, serde_json::json!({
                "class": "FERMI_CONTEXT_SHARE_A",
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_ctx_share:08x}"),
            })));
            if !ok { success = false; }
        }

        // Step 11: GPFIFO channel
        let mut rm_channel = 0u32;
        if success {
            eprintln!("[Phase 3] Step 11: GPFIFO channel (VOLTA_CHANNEL_GPFIFO_A)...");
            let mut ch = NvChannelAllocParams::default();
            ch.h_object_error = rm_err_notifier;
            ch.h_object_buffer = rm_gpfifo_mem;
            ch.gpfifo_entries = 64;
            ch.h_context_share = rm_ctx_share;
            ch.h_vaspace = rm_vaspace;
            ch.h_userd_memory[0] = rm_userd;
            ch.engine_type = NV2080_ENGINE_TYPE_GR0;

            let r = rm_alloc(fd, rm_root, rm_tsg, H_CHANNEL, class::VOLTA_CHANNEL_GPFIFO_A,
                &ch as *const _ as u64, size_of::<NvChannelAllocParams>() as u32);
            rm_channel = r.h_object_new;

            let ch_ok = r.rc == 0 && r.status == 0;
            if ch_ok {
                channel_id = Some(ch.cid);
                eprintln!("  Channel allocated, cid={}, kernel_handle=0x{rm_channel:08x}", ch.cid);
            }
            steps.push(step_json("channel_alloc", ch_ok, serde_json::json!({
                "class": "VOLTA_CHANNEL_GPFIFO_A",
                "status": format!("0x{:x}", r.status),
                "channel_id": ch.cid,
                "kernel_handle": format!("0x{rm_channel:08x}"),
            })));
            if !ch_ok { success = false; }
        }

        // Step 12: Compute engine object
        let mut rm_compute = 0u32;
        if success {
            eprintln!("[Phase 3] Step 12: Compute engine (VOLTA_COMPUTE_A)...");
            let r = rm_alloc(fd, rm_root, rm_channel, H_COMPUTE, class::VOLTA_COMPUTE_A, 0, 0);
            rm_compute = r.h_object_new;
            let ok = r.rc == 0 && r.status == 0;
            steps.push(step_json("compute_alloc", ok, serde_json::json!({
                "class": "VOLTA_COMPUTE_A",
                "status": format!("0x{:x}", r.status),
                "kernel_handle": format!("0x{rm_compute:08x}"),
            })));
            if !ok { success = false; }
        }

        // Step 13: BIND channel to GR engine
        if success {
            eprintln!("[Phase 4] Step 13: BIND channel to GR...");
            let mut bp = NvChannelBindParams::default();
            bp.h_engine_object = rm_compute;
            bp.engine_class_1 = class::VOLTA_COMPUTE_A;
            bp.engine_class_2 = class::VOLTA_COMPUTE_A;
            bp.engine_type = NV2080_ENGINE_TYPE_GR0;
            let (rc, status) = rm_ctrl(fd, rm_root, rm_channel, rm_abi::NV906F_CTRL_CMD_BIND,
                &bp as *const _ as u64, size_of::<NvChannelBindParams>() as u32);
            let bind_ok = rc == 0 && status == 0;
            steps.push(step_json("channel_bind", bind_ok, serde_json::json!({
                "cmd": "BIND", "status": format!("0x{status:x}"),
            })));
            if !bind_ok {
                eprintln!("  BIND status=0x{status:x} — Volta may auto-bind via TSG, proceeding...");
            }
        }

        // Step 14: SCHEDULE TSG
        if success {
            eprintln!("[Phase 4] Step 14: SCHEDULE TSG (handle=0x{rm_tsg:08x})...");
            let mut sp = NvGpfifoScheduleParams { b_enable: 1 };
            let (rc, status) = rm_ctrl(fd, rm_root, rm_tsg, rm_abi::NVA06C_CTRL_CMD_GPFIFO_SCHEDULE,
                &mut sp as *mut _ as u64, size_of::<NvGpfifoScheduleParams>() as u32);
            steps.push(step_json("tsg_schedule", rc == 0 && status == 0, serde_json::json!({
                "cmd": "GPFIFO_SCHEDULE", "status": format!("0x{status:x}"),
            })));
            if rc != 0 || status != 0 { success = false; }
        }

        // Step 15: GET_WORK_SUBMIT_TOKEN
        if success {
            eprintln!("[Phase 4] Step 15: GET_WORK_SUBMIT_TOKEN (handle=0x{rm_channel:08x})...");
            let mut tp = NvA06fGetWorkSubmitTokenParams::default();
            let (rc, status) = rm_ctrl(fd, rm_root, rm_channel, rm_abi::NVA06F_CTRL_CMD_GPFIFO_GET_WORK_SUBMIT_TOKEN,
                &mut tp as *mut _ as u64, size_of::<NvA06fGetWorkSubmitTokenParams>() as u32);
            if rc == 0 && status == 0 {
                work_submit_token = Some(tp.work_submit_token);
                eprintln!("  work_submit_token = 0x{:08x}", tp.work_submit_token);
            }
            steps.push(step_json("work_submit_token", rc == 0 && status == 0, serde_json::json!({
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
