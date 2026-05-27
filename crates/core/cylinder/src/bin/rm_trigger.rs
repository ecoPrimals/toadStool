// SPDX-License-Identifier: AGPL-3.0-or-later
//! RM trigger — Minimal RM ioctl client to trigger full GR initialization.
//!
//! Opens nvidiactl (minor 255), allocates root -> device -> subdevice,
//! which triggers RM's deferred GPU state loading (GPCCS/TPC init).
//!
//! Usage: rm_trigger <major>
//!
//! This is the pure Rust replacement for the C `rm_trigger.c` tool.
//! Uses `rustix` for ioctl/mknod, outputs structured JSON.

// SAFETY: This binary issues raw NVIDIA RM ioctls which require unsafe ioctl() calls.
// The unsafe blocks are narrowly scoped to the ioctl invocations themselves.
#![allow(unsafe_code)]

use std::os::fd::AsRawFd;
use std::process::ExitCode;

/// NVIDIA ioctl magic number.
const NV_IOCTL_MAGIC: u8 = b'F';
const NV_ESC_RM_ALLOC: u8 = 0x2b;
const NV_ESC_RM_CONTROL: u8 = 0x2a;

/// RM class IDs.
const NV01_ROOT: u32 = 0x0000;
const NV01_DEVICE_0: u32 = 0x0080;
const NV20_SUBDEVICE_0: u32 = 0x2080;
/// NV2080_CTRL_CMD_GR_GET_INFO
const NV2080_CTRL_CMD_GR_GET_INFO: u32 = 0x2080_1201;

/// Handle base for our RM allocations.
const HANDLE_ROOT: u32 = 0xCAFE_0001;
const HANDLE_DEVICE: u32 = 0xCAFE_0002;
const HANDLE_SUBDEVICE: u32 = 0xCAFE_0003;

/// Sentinel status value to detect whether RM actually wrote the field.
const STATUS_SENTINEL: u32 = 0xDEAD_BEEF;

/// NVOS64_PARAMETERS — used by NV_ESC_RM_ALLOC in 470.x
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

/// NV0080_ALLOC_PARAMETERS — device allocation.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Nv0080AllocParameters {
    device_id: u32,
    h_client_share: u32,
    h_target_client: u32,
    h_target_device: u32,
    flags: u32,
    _pad: [u32; 3],
    va_space_size: u64,
}

/// NVOS54_PARAMETERS — used by NV_ESC_RM_CONTROL.
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

/// Build an ioctl request number for _IOWR(magic, nr, type).
///
/// Linux ioctl encoding:
///   direction (2 bits) | size (14 bits) | type (8 bits) | nr (8 bits)
const fn iowr(magic: u8, nr: u8, size: usize) -> u64 {
    let dir: u64 = 3; // _IOC_READ | _IOC_WRITE
    (dir << 30) | ((size as u64 & 0x3FFF) << 16) | ((magic as u64) << 8) | nr as u64
}

const RM_ALLOC_CMD: u64 = iowr(NV_IOCTL_MAGIC, NV_ESC_RM_ALLOC, size_of::<Nvos64Parameters>());
const RM_CTRL_CMD: u64 = iowr(NV_IOCTL_MAGIC, NV_ESC_RM_CONTROL, size_of::<Nvos54Parameters>());

/// Issue an NV_ESC_RM_ALLOC ioctl.
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
    eprintln!(
        "  RM_ALLOC(cls=0x{:04x}): ioctl rc={} errno={} status=0x{:x}",
        class,
        rc,
        if rc < 0 {
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
        } else {
            0
        },
        p.status
    );
    (rc, p.status)
}

/// JSON output for structured results.
fn print_result(result: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(result).unwrap_or_default());
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <chardev_major>", args[0]);
        return ExitCode::from(1);
    }

    let major: u32 = match args[1].parse() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Invalid major number '{}': {e}", args[1]);
            return ExitCode::from(1);
        }
    };

    eprintln!("sizeof(Nvos64Parameters) = {}", size_of::<Nvos64Parameters>());
    eprintln!(
        "sizeof(Nv0080AllocParameters) = {}",
        size_of::<Nv0080AllocParameters>()
    );
    eprintln!("RM_ALLOC_CMD = 0x{RM_ALLOC_CMD:x}");
    eprintln!("RM_CTRL_CMD  = 0x{RM_CTRL_CMD:x}");

    let ctl_path = "/dev/toadstool-rm-nvidiactl";
    let gpu_path = "/dev/toadstool-rm-nvidia0";

    let _ = std::fs::remove_file(ctl_path);
    let _ = std::fs::remove_file(gpu_path);

    // Create device nodes via rustix
    let mode = rustix::fs::Mode::from_raw_mode(0o666);
    let char_type = rustix::fs::FileType::CharacterDevice;

    if let Err(e) = rustix::fs::mknodat(
        rustix::fs::CWD,
        ctl_path,
        char_type,
        mode,
        rustix::fs::makedev(major, 255),
    ) {
        eprintln!("mknod ctl: {e}");
        return ExitCode::from(1);
    }

    if let Err(e) = rustix::fs::mknodat(
        rustix::fs::CWD,
        gpu_path,
        char_type,
        mode,
        rustix::fs::makedev(major, 0),
    ) {
        eprintln!("mknod gpu: {e}");
        let _ = std::fs::remove_file(ctl_path);
        return ExitCode::from(1);
    }

    let mut steps = Vec::new();
    let mut success = true;

    // Open GPU device (minor 0) -> triggers rm_init_adapter
    eprintln!("\nOpening GPU device (minor 0) to trigger rm_init_adapter...");
    let gpu_fd = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(gpu_path);
    match &gpu_fd {
        Ok(f) => eprintln!("  GPU open succeeded (fd={}) — RM init triggered", f.as_raw_fd()),
        Err(e) => {
            eprintln!("  GPU open failed: {e}");
            success = false;
        }
    }

    // Open ctl device for RM ioctls
    eprintln!("\nOpening nvidiactl (minor 255) for RM ioctls...");
    let ctl_fd = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(ctl_path);
    let ctl_file = match ctl_fd {
        Ok(f) => {
            eprintln!("  ctl open succeeded (fd={})", f.as_raw_fd());
            f
        }
        Err(e) => {
            eprintln!("  ctl open failed: {e}");
            steps.push(serde_json::json!({"step": "open_ctl", "ok": false, "error": e.to_string()}));
            cleanup(ctl_path, gpu_path);
            print_result(&serde_json::json!({
                "success": false,
                "major": major,
                "steps": steps,
            }));
            return ExitCode::from(1);
        }
    };
    let fd = ctl_file.as_raw_fd();

    // Step 1: root client
    eprintln!("\nStep 1: Allocating root client (NV01_ROOT)...");
    let (rc, status) = rm_alloc(fd, 0, 0, HANDLE_ROOT, NV01_ROOT, 0, 0);
    steps.push(serde_json::json!({"step": "root_alloc", "class": "NV01_ROOT", "rc": rc, "status": format!("0x{status:x}")}));
    if status != 0 {
        eprintln!("  Root alloc failed (status=0x{status:x})");
        success = false;
    }

    if success {
        // Step 2: device
        eprintln!("\nStep 2: Allocating device (NV01_DEVICE_0)...");
        let mut dev_params = Nv0080AllocParameters::default();
        dev_params.device_id = 0;
        let (rc, status) = rm_alloc(
            fd,
            HANDLE_ROOT,
            HANDLE_ROOT,
            HANDLE_DEVICE,
            NV01_DEVICE_0,
            &dev_params as *const _ as u64,
            size_of::<Nv0080AllocParameters>() as u32,
        );
        steps.push(serde_json::json!({"step": "device_alloc", "class": "NV01_DEVICE_0", "rc": rc, "status": format!("0x{status:x}")}));
        if status != 0 {
            eprintln!("  Device alloc failed (status=0x{status:x})");
            success = false;
        }
    }

    if success {
        // Step 3: subdevice
        eprintln!("\nStep 3: Allocating subdevice (NV20_SUBDEVICE_0)...");
        let mut sub_id: u32 = 0;
        let (rc, status) = rm_alloc(
            fd,
            HANDLE_ROOT,
            HANDLE_DEVICE,
            HANDLE_SUBDEVICE,
            NV20_SUBDEVICE_0,
            &mut sub_id as *mut u32 as u64,
            size_of::<u32>() as u32,
        );
        steps.push(serde_json::json!({"step": "subdevice_alloc", "class": "NV20_SUBDEVICE_0", "rc": rc, "status": format!("0x{status:x}")}));
        if status != 0 {
            eprintln!("  Subdevice alloc failed (status=0x{status:x})");
            success = false;
        }
    }

    if success {
        // Step 4: GR control
        eprintln!("\nStep 4: GR control (NV2080_CTRL_CMD_GR_GET_INFO)...");
        let mut ctrl = Nvos54Parameters {
            h_client: HANDLE_ROOT,
            h_object: HANDLE_SUBDEVICE,
            cmd: NV2080_CTRL_CMD_GR_GET_INFO,
            status: STATUS_SENTINEL,
            ..Nvos54Parameters::default()
        };
        let rc = unsafe { libc::ioctl(fd, RM_CTRL_CMD, &mut ctrl as *mut Nvos54Parameters) };
        let errno = if rc < 0 {
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
        } else {
            0
        };
        eprintln!(
            "  GR_GET_INFO: ioctl rc={rc} errno={errno} status=0x{:x}",
            ctrl.status
        );
        steps.push(serde_json::json!({
            "step": "gr_get_info",
            "cmd": "NV2080_CTRL_CMD_GR_GET_INFO",
            "rc": rc,
            "errno": errno,
            "status": format!("0x{:x}", ctrl.status),
        }));
    }

    // Hold fds open briefly for async RM work
    eprintln!("\nHolding fds open for 5s...");
    std::thread::sleep(std::time::Duration::from_secs(5));
    eprintln!("Done.");

    drop(ctl_file);
    drop(gpu_fd);
    cleanup(ctl_path, gpu_path);

    print_result(&serde_json::json!({
        "success": success,
        "major": major,
        "steps": steps,
    }));

    ExitCode::SUCCESS
}

fn cleanup(ctl_path: &str, gpu_path: &str) {
    let _ = std::fs::remove_file(ctl_path);
    let _ = std::fs::remove_file(gpu_path);
}
