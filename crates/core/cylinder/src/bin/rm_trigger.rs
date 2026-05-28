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
use std::os::unix::fs::OpenOptionsExt;

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

/// Read PMC_ENABLE (offset 0x200) from BAR0 via sysfs resource0 mmap
fn read_pmc_enable(bdf: &str) -> Option<u32> {
    let path = format!("/sys/bus/pci/devices/{bdf}/resource0");
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_RDONLY)
        .open(&path).ok()?;
    let bar0_fd = f.as_raw_fd();
    let map = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            0x1000,
            libc::PROT_READ,
            libc::MAP_SHARED,
            bar0_fd,
            0, // BAR0 start (resource0 is already the BAR0 region)
        )
    };
    if map == libc::MAP_FAILED {
        return None;
    }
    let val = unsafe { std::ptr::read_volatile((map as *const u8).add(0x200) as *const u32) };
    unsafe { libc::munmap(map, 0x1000); }
    Some(val)
}

// Volta (GV100) uses SET/CLEAR register pairs for interrupt enable:
//   0x140 — NV_PMC_INTR_EN(0): READ-ONLY, shows current enable mask
//   0x160 — NV_PMC_INTR_EN_SET(0): WRITE-ONLY, writing 1 bits enables
//   0x180 — NV_PMC_INTR_EN_CLEAR(0): WRITE-ONLY, writing 1 bits disables
// Writing to 0x140 is a NO-OP (lockup #5 confirmed: 0x7fffffff → 0x7fffffff).
const NV_PMC_INTR_EN_0: usize = 0x140;
const NV_PMC_INTR_EN_CLEAR_0: usize = 0x180;
// Also clear the top-level interrupt pending register to ACK any in-flight IRQs
const NV_PMC_INTR_0: usize = 0x100;

/// Quench all GPU interrupt generation BEFORE nvidia close tears down MSI/IRQ.
///
/// Without this, nvidia_close runs: free_irq → pci_disable_msi. The GPU is still
/// warm with active engines generating interrupts. With MSI gone and INTx enabled
/// (pci_cmd bit 10 = 0), level-triggered legacy INTx fires with no handler to ACK
/// at the GPU level → infinite interrupt storm → system lockup.
///
/// Writes 0xFFFFFFFF to NV_PMC_INTR_EN_CLEAR(0) at BAR0+0x180 to disable ALL
/// interrupt sources at the GPU level. This is the Volta SET/CLEAR register —
/// writing to 0x140 (the read-only status register) was the bug in lockups #4/#5.
fn quench_gpu_interrupts(bdf: &str) {
    let bar0_path = format!("/sys/bus/pci/devices/{bdf}/resource0");
    if let Ok(f) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&bar0_path)
    {
        let bar0_fd = f.as_raw_fd();
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                0x1000,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                bar0_fd,
                0,
            )
        };
        if map != libc::MAP_FAILED {
            // Read current INTR_EN state (read-only register at 0x140)
            let old_en = unsafe {
                std::ptr::read_volatile((map as *const u8).add(NV_PMC_INTR_EN_0) as *const u32)
            };

            // CLEAR all interrupt enables by writing 0xFFFFFFFF to INTR_EN_CLEAR (0x180)
            unsafe {
                std::ptr::write_volatile(
                    (map as *mut u8).add(NV_PMC_INTR_EN_CLEAR_0) as *mut u32,
                    0xFFFF_FFFF,
                );
            }

            // Read back INTR_EN to verify the CLEAR took effect
            let new_en = unsafe {
                std::ptr::read_volatile((map as *const u8).add(NV_PMC_INTR_EN_0) as *const u32)
            };

            // Also read and ACK any pending interrupts (read NV_PMC_INTR clears edge-triggered)
            let pending = unsafe {
                std::ptr::read_volatile((map as *const u8).add(NV_PMC_INTR_0) as *const u32)
            };

            unsafe { libc::munmap(map, 0x1000); }
            eprintln!(
                "[QUENCH] INTR_EN: 0x{old_en:08x} → 0x{new_en:08x} (wrote 0xFFFFFFFF to CLEAR@0x180) pending=0x{pending:08x}"
            );
            if new_en != 0 {
                eprintln!("[QUENCH] WARNING: INTR_EN not zero after CLEAR — GPU may still generate interrupts!");
            }
        } else {
            eprintln!("[QUENCH] BAR0 mmap failed — cannot disable GPU interrupts at source!");
        }
    } else {
        eprintln!("[QUENCH] Cannot open {bar0_path} for write — interrupt quench skipped");
    }
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

    // Read PMC_ENABLE before any driver interaction
    let pmc_pre = read_pmc_enable("0000:49:00.0");
    eprintln!("[Diag] PMC_ENABLE before opens: {:?}", pmc_pre.map(|v| format!("0x{v:08x}")));

    // Open CTL first so nvidia_ctl_open runs nv_acpi_init before any GPU init.
    eprintln!("\nOpening nvidiactl (minor 255) [CTL-first]...");
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

    let pmc_post_ctl = read_pmc_enable("0000:49:00.0");
    eprintln!("[Diag] PMC_ENABLE after CTL open: {:?}", pmc_post_ctl.map(|v| format!("0x{v:08x}")));

    // Open GPU device AFTER ctl — triggers nvidia_open → nv_open_device →
    // nv_start_device → rm_init_adapter.
    eprintln!("\nOpening GPU device (minor 0) — triggers rm_init_adapter...");
    let gpu_fd = std::fs::OpenOptions::new().read(true).write(true).open(gpu_path);
    match &gpu_fd {
        Ok(f) => eprintln!("  GPU open ok (fd={})", f.as_raw_fd()),
        Err(e) => {
            eprintln!("  GPU open FAILED: {e}");
            steps.push(step_json("gpu_open", false, serde_json::json!({"error": e.to_string()})));
        }
    }

    let pmc_post_gpu = read_pmc_enable("0000:49:00.0");
    eprintln!("[Diag] PMC_ENABLE after GPU open: {:?}", pmc_post_gpu.map(|v| format!("0x{v:08x}")));
    if pmc_pre == pmc_post_gpu {
        eprintln!("  ⚠ PMC_ENABLE UNCHANGED — rm_init_adapter likely did NOT run DEVINIT");
    } else {
        eprintln!("  ✓ PMC_ENABLE CHANGED — rm_init_adapter ran DEVINIT");
    }

    // Try RM operations on BOTH fds to see if the GPU fd works differently
    let gpu_raw_fd = gpu_fd.as_ref().map(|f| f.as_raw_fd()).unwrap_or(-1);
    if gpu_raw_fd >= 0 {
        eprintln!("\n[Diag] Trying root alloc on GPU fd ({gpu_raw_fd})...");
        let r_gpu = rm_alloc(gpu_raw_fd, 0, 0, 0xBEEF_0001, class::NV01_ROOT_CLIENT, 0, 0);
        if r_gpu.status == 0 {
            let gpu_root = r_gpu.h_object_new;
            eprintln!("  ✓ Root client on GPU fd: handle=0x{gpu_root:08x}");
            let mut p_ids = [0u32; 32];
            let (_, st) = rm_ctrl(gpu_raw_fd, gpu_root, gpu_root, 0x0000_0214,
                p_ids.as_mut_ptr() as u64, size_of::<[u32; 32]>() as u32);
            let ids: Vec<u32> = p_ids.iter().copied().filter(|&id| id != 0 && id != 0xFFFF_FFFF).collect();
            eprintln!("  GPU_GET_PROBED_IDS on GPU fd: status=0x{st:x} ids={ids:?}");

            // Try device_alloc on GPU fd
            let mut dp2 = rm_abi::Nv0080AllocParams::default();
            dp2.device_id = 0;
            dp2.h_client_share = gpu_root;
            let r2 = rm_alloc(gpu_raw_fd, gpu_root, gpu_root, 0xBEEF_0002, class::NV01_DEVICE_0,
                &dp2 as *const _ as u64, size_of::<rm_abi::Nv0080AllocParams>() as u32);
            eprintln!("  device_alloc on GPU fd(device_id=0): status=0x{:x}", r2.status);
        } else {
            eprintln!("  ✗ Root client on GPU fd FAILED: status=0x{:x}", r_gpu.status);
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 0a: NV_ESC_CARD_INFO — reads nv_pci_probe's device registry
    // sizeof(nv_ioctl_card_info_t) = 72 on x86_64.
    // Offsets: valid@0 pci_info{domain@4 bus@8 slot@9 func@10
    //          vendor@12 device@14} gpu_id@16 irq@20 reg_addr@24
    //          reg_size@32 fb_addr@40 fb_size@48 minor@56 dev_name@60
    // ═══════════════════════════════════════════════════════════════════
    {
        const CARD_INFO_ENTRY: usize = 72;
        const MAX_CARDS: usize = 8;
        let total_size = CARD_INFO_ENTRY * MAX_CARDS;
        let mut ci_buf = vec![0u8; total_size];
        let ci_cmd: u64 = iowr(NV_IOCTL_MAGIC, 0xC8, total_size);
        let rc = unsafe { libc::ioctl(fd, ci_cmd, ci_buf.as_mut_ptr()) };
        let errno = if rc < 0 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) } else { 0 };
        eprintln!("\n[Diag] NV_ESC_CARD_INFO: rc={rc} errno={errno}");
        for i in 0..MAX_CARDS {
            let off = i * CARD_INFO_ENTRY;
            let valid = ci_buf[off];
            if valid == 0 { continue; }
            let domain = u32::from_ne_bytes(ci_buf[off+4..off+8].try_into().unwrap());
            let bus = ci_buf[off+8];
            let slot = ci_buf[off+9];
            let func = ci_buf[off+10];
            let vendor = u16::from_ne_bytes(ci_buf[off+12..off+14].try_into().unwrap());
            let devid = u16::from_ne_bytes(ci_buf[off+14..off+16].try_into().unwrap());
            let gpu_id = u32::from_ne_bytes(ci_buf[off+16..off+20].try_into().unwrap());
            let irq = u16::from_ne_bytes(ci_buf[off+20..off+22].try_into().unwrap());
            let reg_addr = u64::from_ne_bytes(ci_buf[off+24..off+32].try_into().unwrap());
            let reg_size = u64::from_ne_bytes(ci_buf[off+32..off+40].try_into().unwrap());
            let fb_addr = u64::from_ne_bytes(ci_buf[off+40..off+48].try_into().unwrap());
            let fb_size = u64::from_ne_bytes(ci_buf[off+48..off+56].try_into().unwrap());
            let minor = u32::from_ne_bytes(ci_buf[off+56..off+60].try_into().unwrap());
            eprintln!("  card[{i}]: valid={valid} {domain:04x}:{bus:02x}:{slot:02x}.{func} vendor=0x{vendor:04x} dev=0x{devid:04x} gpu_id=0x{gpu_id:x} irq={irq} minor={minor}");
            eprintln!("    regs=0x{reg_addr:x}+0x{reg_size:x} fb=0x{fb_addr:x}+0x{fb_size:x}");
        }
        steps.push(step_json("card_info", rc == 0, serde_json::json!({"rc": rc, "errno": errno})));

        // Check PCI command register for first valid card — verify BusMaster
        for i in 0..MAX_CARDS {
            let off = i * CARD_INFO_ENTRY;
            let valid = ci_buf[off];
            if valid == 0 { continue; }
            let domain = u32::from_ne_bytes(ci_buf[off+4..off+8].try_into().unwrap());
            let bus = ci_buf[off+8];
            let slot = ci_buf[off+9];
            let func = ci_buf[off+10];
            let bdf = format!("{domain:04x}:{bus:02x}:{slot:02x}.{func}");
            let cfg_path = format!("/sys/bus/pci/devices/{bdf}/config");
            match std::fs::read(&cfg_path) {
                Ok(cfg) if cfg.len() >= 6 => {
                    let cmd = u16::from_le_bytes([cfg[4], cfg[5]]);
                    let io_en = cmd & 1;
                    let mem_en = (cmd >> 1) & 1;
                    let bus_master = (cmd >> 2) & 1;
                    eprintln!("[Diag] PCI CMD({bdf}): 0x{cmd:04x} IO={io_en} MEM={mem_en} BusMaster={bus_master}");
                    if bus_master == 0 {
                        eprintln!("  WARNING: BusMaster DISABLED — enabling via sysfs...");
                        let enable_path = format!("/sys/bus/pci/devices/{bdf}/enable");
                        let _ = std::fs::write(&enable_path, "1");
                        // Directly set bus master bit via config write
                        if let Ok(mut f) = std::fs::OpenOptions::new().read(true).write(true).open(&cfg_path) {
                            use std::io::{Read, Seek, Write};
                            let mut cmd_bytes = [0u8; 2];
                            let _ = f.seek(std::io::SeekFrom::Start(4));
                            let _ = f.read_exact(&mut cmd_bytes);
                            let new_cmd = u16::from_le_bytes(cmd_bytes) | 0x0004; // set BusMaster bit
                            let _ = f.seek(std::io::SeekFrom::Start(4));
                            let _ = f.write_all(&new_cmd.to_le_bytes());
                            eprintln!("  PCI CMD now: 0x{new_cmd:04x} (BusMaster forced ON)");
                        }
                    }
                }
                Ok(cfg) => eprintln!("[Diag] PCI config too short ({} bytes)", cfg.len()),
                Err(e) => eprintln!("[Diag] PCI config read failed ({bdf}): {e}"),
            }
            break; // only check first card
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 0: Root client — kernel REWRITES h_object_new
    // ═══════════════════════════════════════════════════════════════════

    // Try NV01_ROOT_CLIENT (0x41) first — nvidia-470 RM may require this
    eprintln!("\n[Phase 0] Allocating root client (class 0x0041 NV01_ROOT_CLIENT)...");
    let root_test = rm_alloc(fd, 0, 0, H_ROOT, class::NV01_ROOT_CLIENT, 0, 0);
    let rm_root = if root_test.status == 0 {
        root_test.h_object_new
    } else {
        eprintln!("  ROOT_CLIENT(0x41) failed, falling back to NV01_ROOT(0x0000)...");
        let r2 = rm_alloc(fd, 0, 0, H_ROOT, class::NV01_ROOT, 0, 0);
        r2.h_object_new
    };
    eprintln!("  Kernel assigned root handle: 0x{rm_root:08x} (we asked for 0x{H_ROOT:08x})");

    let mut gpu_ids = [0u32; 32];
    let (_, ctrl_st) = rm_ctrl(fd, rm_root, rm_root, 0x0000_0201,
        gpu_ids.as_mut_ptr() as u64, size_of::<[u32; 32]>() as u32);
    let attached: Vec<u32> = gpu_ids.iter().copied().filter(|&id| id != 0 && id != 0xFFFF_FFFF).collect();
    eprintln!("[Phase 0] GPU_GET_ATTACHED_IDS: status=0x{ctrl_st:x} ids={attached:?}");

    steps.push(step_json("root_client", root_test.status == 0, serde_json::json!({
        "requested_handle": format!("0x{H_ROOT:08x}"),
        "kernel_assigned": format!("0x{rm_root:08x}"),
        "attached_ids": attached,
    })));

    let gpu_id = attached.first().copied().unwrap_or(0);

    // PMC_ENABLE after root alloc
    let pmc_post_root = read_pmc_enable("0000:49:00.0");
    eprintln!("[Diag] PMC_ENABLE after root alloc: {:?}", pmc_post_root.map(|v| format!("0x{v:08x}")));

    // Pre-attach diagnostics
    {
        let mut pre_probed = [0u32; 32];
        let (_, st) = rm_ctrl(fd, rm_root, rm_root, 0x0000_0214,
            pre_probed.as_mut_ptr() as u64, size_of::<[u32; 32]>() as u32);
        let ids: Vec<u32> = pre_probed.iter().copied().filter(|&id| id != 0 && id != 0xFFFF_FFFF).collect();
        eprintln!("[Diag] PRE-ATTACH GPU_GET_PROBED_IDS: status=0x{st:x} ids={ids:?}");
        let pmc_now = read_pmc_enable("0000:49:00.0");
        eprintln!("[Diag] PMC_ENABLE after probed_ids query: {:?}", pmc_now.map(|v| format!("0x{v:08x}")));
    }

    // GPU_ATTACH_IDS ctrl cmd (0x0280) — manually register GPU with RM's
    // GPU manager, which is what populates the probed table. This is the
    // RM-level attach (vs. the kernel-level NV_ESC_ATTACH_GPUS_TO_FD).
    if gpu_id != 0 {
        let mut attach_ctrl_ids = [gpu_id, 0u32];
        let (_, st) = rm_ctrl(fd, rm_root, rm_root, 0x0000_0280,
            attach_ctrl_ids.as_mut_ptr() as u64, size_of::<[u32; 2]>() as u32);
        eprintln!("[Diag] GPU_ATTACH_IDS(0x{gpu_id:x}): status=0x{st:x}");
        steps.push(step_json("gpu_attach_ids_ctrl", st == 0, serde_json::json!({
            "gpu_id": format!("0x{gpu_id:x}"), "status": format!("0x{st:x}"),
        })));
    }

    // Post-attach-ctrl check
    {
        let mut post_probed = [0u32; 32];
        let (_, st) = rm_ctrl(fd, rm_root, rm_root, 0x0000_0214,
            post_probed.as_mut_ptr() as u64, size_of::<[u32; 32]>() as u32);
        let ids: Vec<u32> = post_probed.iter().copied().filter(|&id| id != 0 && id != 0xFFFF_FFFF).collect();
        eprintln!("[Diag] POST-ATTACH-CTRL GPU_GET_PROBED_IDS: status=0x{st:x} ids={ids:?}");
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 0b: NV_ESC_ATTACH_GPUS_TO_FD (kernel-level GPU attachment)
    // ═══════════════════════════════════════════════════════════════════

    if gpu_id != 0 {
        eprintln!("\n[Phase 0b] NV_ESC_ATTACH_GPUS_TO_FD: gpu_id=0x{gpu_id:x}...");
        let mut attach_buf = [gpu_id];
        let attach_cmd: u64 = iowr(NV_IOCTL_MAGIC, 0xD4, size_of::<[u32; 1]>());
        let rc = unsafe { libc::ioctl(fd, attach_cmd, attach_buf.as_mut_ptr()) };
        let errno = if rc < 0 {
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
        } else {
            0
        };
        eprintln!("  ATTACH_GPUS_TO_FD: rc={rc} errno={errno}");
        let ok = rc == 0;
        steps.push(step_json("attach_gpus_to_fd", ok, serde_json::json!({
            "gpu_id": format!("0x{gpu_id:x}"),
            "rc": rc,
            "errno": errno,
        })));
        if !ok {
            eprintln!("  CRITICAL: ATTACH_GPUS_TO_FD failed — rm_init_adapter will not run!");
            success = false;
        }
    } else {
        eprintln!("\n[Phase 0b] No GPU IDs from GET_ATTACHED_IDS — cannot attach");
        steps.push(step_json("attach_gpus_to_fd", false, serde_json::json!({
            "error": "no gpu_ids available"
        })));
        success = false;
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 1: Core object tree using KERNEL-ASSIGNED handles
    // ═══════════════════════════════════════════════════════════════════

    let root_ok = root_test.rc == 0 && root_test.status == 0;
    if !root_ok { success = false; }

    // Step 2: device alloc — device_id is GPU instance index (0 for first GPU)
    let mut rm_device = 0u32;
    if root_ok && success {
        eprintln!("\n[Phase 1] Step 2: device alloc (parent=0x{rm_root:08x})...");

        // After ATTACH_GPUS_TO_FD, GPU_GET_PROBED_IDS should return our GPU
        let mut probed_ids = [0u32; 32];
        let (_, probed_st) = rm_ctrl(fd, rm_root, rm_root, 0x0000_0214,
            probed_ids.as_mut_ptr() as u64, size_of::<[u32; 32]>() as u32);
        let probed: Vec<u32> = probed_ids.iter().copied().filter(|&id| id != 0 && id != 0xFFFF_FFFF).collect();
        eprintln!("  GPU_GET_PROBED_IDS: status=0x{probed_st:x} ids={probed:?}");

        // Try device_id=0 first (first GPU instance), then gpu_id directly
        let mut dp = rm_abi::Nv0080AllocParams::default();
        dp.device_id = 0;
        dp.h_client_share = rm_root;

        let r = rm_alloc(fd, rm_root, rm_root, H_DEVICE, class::NV01_DEVICE_0,
            &dp as *const _ as u64, size_of::<rm_abi::Nv0080AllocParams>() as u32);
        rm_device = r.h_object_new;
        let mut device_ok = r.rc == 0 && r.status == 0;
        eprintln!("  device_alloc(device_id=0): status=0x{:x} handle=0x{:08x}", r.status, r.h_object_new);

        if !device_ok && gpu_id != 0 {
            eprintln!("  Retrying with device_id=gpu_id (0x{gpu_id:x})...");
            dp.device_id = gpu_id;
            let r2 = rm_alloc(fd, rm_root, rm_root, H_DEVICE, class::NV01_DEVICE_0,
                &dp as *const _ as u64, size_of::<rm_abi::Nv0080AllocParams>() as u32);
            rm_device = r2.h_object_new;
            device_ok = r2.rc == 0 && r2.status == 0;
            eprintln!("  device_alloc(device_id=0x{gpu_id:x}): status=0x{:x} handle=0x{:08x}", r2.status, r2.h_object_new);
        }

        steps.push(step_json("device_alloc", device_ok, serde_json::json!({
            "class": "NV01_DEVICE_0",
            "status": format!("0x{:x}", r.status),
            "kernel_handle": format!("0x{rm_device:08x}"),
            "probed_ids": probed,
        })));

        if !device_ok {
            // Diagnostic: GPU_GET_ID_INFO to understand GPU state
            let mut id_info = [0u32; 6];
            id_info[0] = gpu_id;
            let (_, st) = rm_ctrl(fd, rm_root, rm_root, 0x0000_0202,
                id_info.as_mut_ptr() as u64, 24);
            eprintln!("[Diag] GPU_GET_ID_INFO(0x{gpu_id:x}): status=0x{st:x} devInst={} subDevInst={} gpuInst={}",
                id_info[2], id_info[3], id_info[5]);
            success = false;
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

    // CRITICAL: Quench GPU interrupt generation BEFORE closing nvidia fds.
    // nvidia_close → nv_stop_device skips rm_disable_adapter (NOP'd) but still
    // runs free_irq + pci_disable_msi. A warm GPU with active engines will
    // fire unhandled legacy INTx → interrupt storm → system lockup.
    eprintln!("\n[SAFETY] Quenching GPU interrupts before fd close...");
    quench_gpu_interrupts("0000:49:00.0");

    eprintln!("Dropping nvidia fds...");
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
