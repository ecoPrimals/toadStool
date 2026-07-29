// SPDX-License-Identifier: AGPL-3.0-or-later
//! RM trigger — Full RM compute channel client for catalyst pipeline.
//!
//! Opens nvidiactl + nvidia0, allocates the complete RM object tree needed for
//! a GR compute channel: root → device → subdevice → VA space → memory objects
//! → TSG → context share → GPFIFO channel → compute engine → BIND → SCHEDULE
//! → work submit token. This establishes FECS ctx-switch state that survives
//! the catalyst warm swap (Exp 229).
//!
//! Usage: rm_trigger `<major>` [--channel]
//!
//! Without --channel: legacy mode (root/device/subdevice/GR_GET_INFO only).
//! With --channel: full RM compute channel creation (Exp 229).
//!
//! Outputs structured JSON on stdout, diagnostics on stderr.

// SAFETY: Raw NVIDIA RM ioctls require unsafe ioctl() calls.
#![allow(unsafe_code)]
#![allow(
    clippy::borrow_as_ptr,
    clippy::needless_pass_by_value,
    clippy::field_reassign_with_default,
    clippy::similar_names,
    clippy::if_not_else,
    clippy::cast_ptr_alignment,
    clippy::map_unwrap_or,
    clippy::items_after_statements,
    clippy::ref_as_ptr,
    unused_assignments
)]

#[cfg(target_os = "linux")]
mod channel_tree;
#[cfg(target_os = "linux")]
mod rm_ioctl;
#[cfg(target_os = "linux")]
mod rm_object_tree;

#[cfg(target_os = "linux")]
use std::os::fd::{AsFd, AsRawFd};
#[cfg(target_os = "linux")]
use std::process::ExitCode;

#[cfg(target_os = "linux")]
use rustix::ioctl::Opcode;

#[cfg(target_os = "linux")]
use toadstool_cylinder::bin_helpers::Bar0;
#[cfg(target_os = "linux")]
use toadstool_cylinder::nv::rm_abi::NvChannelAllocParams;

#[cfg(target_os = "linux")]
use rm_ioctl::{NV_IOCTL_MAGIC, Nvos21Parameters, RM_ALLOC_OP, RM_CTRL_OP, RmRawIoctl, iowr};
#[cfg(target_os = "linux")]
use rm_object_tree::{
    H_CHANNEL, H_COMPUTE, H_CTX_SHARE, H_DEVICE, H_MEM_ERR_NOTIFIER, H_MEM_GPFIFO, H_MEM_USERD,
    H_ROOT, H_SUBDEVICE, H_TSG, H_VASPACE, RootClientResult, alloc_compute_channel,
    alloc_core_tree, alloc_root_client, diag_gpu_fd_root_alloc, gpu_attach_ids_ctrl,
    post_attach_diagnostics, pre_attach_diagnostics,
};

#[cfg(target_os = "linux")]
fn step_json(name: &str, ok: bool, detail: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"step": name, "ok": ok, "detail": detail})
}

#[cfg(target_os = "linux")]
fn ne_bytes<const N: usize>(slice: &[u8], field: &str) -> Result<[u8; N], String> {
    slice
        .try_into()
        .map_err(|_| format!("card_info {field}: expected {N} bytes, got {}", slice.len()))
}

#[cfg(target_os = "linux")]
fn print_result(result: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(result).unwrap_or_default()
    );
}

#[cfg(target_os = "linux")]
fn cleanup(ctl_path: &str, gpu_path: &str) {
    let _ = std::fs::remove_file(ctl_path);
    let _ = std::fs::remove_file(gpu_path);
}

/// Read PMC_ENABLE (offset 0x200) from BAR0 via sysfs resource0 mmap
#[cfg(target_os = "linux")]
fn read_pmc_enable(bdf: &str) -> Option<u32> {
    const BAR0_SIZE: usize = 0x1000;
    let path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "resource0");
    let f = std::fs::OpenOptions::new().read(true).open(&path).ok()?;
    // SAFETY: f is a valid sysfs BAR0 resource0; BAR0_SIZE covers PMC registers.
    let bar0 = unsafe { Bar0::map_with_prot(f.as_fd(), BAR0_SIZE, false) }.ok()?;
    Some(bar0.r32(0x200))
}

// Volta (GV100) uses SET/CLEAR register pairs for interrupt enable:
//   0x140 — NV_PMC_INTR_EN(0): READ-ONLY, shows current enable mask
//   0x160 — NV_PMC_INTR_EN_SET(0): WRITE-ONLY, writing 1 bits enables
//   0x180 — NV_PMC_INTR_EN_CLEAR(0): WRITE-ONLY, writing 1 bits disables
// Writing to 0x140 is a NO-OP (lockup #5 confirmed: 0x7fffffff → 0x7fffffff).
#[cfg(target_os = "linux")]
const NV_PMC_INTR_EN_0: usize = 0x140;
#[cfg(target_os = "linux")]
const NV_PMC_INTR_EN_CLEAR_0: usize = 0x180;
// Also clear the top-level interrupt pending register to ACK any in-flight IRQs
#[cfg(target_os = "linux")]
const NV_PMC_INTR_0: usize = 0x100;

/// Quench all GPU interrupt generation BEFORE nvidia close tears down MSI/IRQ.
#[cfg(target_os = "linux")]
fn quench_gpu_interrupts(bdf: &str) {
    let bar0_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "resource0");
    if let Ok(f) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&bar0_path)
    {
        const BAR0_SIZE: usize = 0x1000;
        // SAFETY: f is a valid sysfs BAR0 resource0; BAR0_SIZE covers PMC registers.
        match unsafe { Bar0::map(f.as_fd(), BAR0_SIZE) } {
            Ok(bar0) => {
                let old_en = bar0.r32(NV_PMC_INTR_EN_0 as u32);
                bar0.w32(NV_PMC_INTR_EN_CLEAR_0 as u32, 0xFFFF_FFFF);
                let new_en = bar0.r32(NV_PMC_INTR_EN_0 as u32);
                let pending = bar0.r32(NV_PMC_INTR_0 as u32);
                eprintln!(
                    "[QUENCH] INTR_EN: 0x{old_en:08x} → 0x{new_en:08x} (wrote 0xFFFFFFFF to CLEAR@0x180) pending=0x{pending:08x}"
                );
                if new_en != 0 {
                    eprintln!(
                        "[QUENCH] WARNING: INTR_EN not zero after CLEAR — GPU may still generate interrupts!"
                    );
                }
            }
            Err(_) => {
                eprintln!("[QUENCH] BAR0 mmap failed — cannot disable GPU interrupts at source!");
            }
        }
    } else {
        eprintln!("[QUENCH] Cannot open {bar0_path} for write — interrupt quench skipped");
    }
}

/// Disable MSI at PCI config level — standalone version for rm_trigger binary.
#[cfg(target_os = "linux")]
fn disable_pci_msi_config(bdf: &str) {
    use std::io::{Read, Seek, Write};
    let cfg_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "config");
    let mut f = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&cfg_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[MSI] Cannot open config for {bdf}: {e}");
            return;
        }
    };

    let mut hdr = [0u8; 0x40];
    if f.seek(std::io::SeekFrom::Start(0)).is_err() || f.read_exact(&mut hdr).is_err() {
        eprintln!("[MSI] Cannot read PCI header for {bdf}");
        return;
    }

    let status = u16::from_le_bytes([hdr[0x06], hdr[0x07]]);
    if status & 0x0010 == 0 {
        eprintln!("[MSI] No capabilities list for {bdf}");
        return;
    }

    let mut cap_ptr = hdr[0x34] & 0xFC;
    let mut visited = 0u32;

    while cap_ptr != 0 && visited < 48 {
        visited += 1;
        let cap_offset = u64::from(cap_ptr);
        let mut cap_hdr = [0u8; 4];
        if f.seek(std::io::SeekFrom::Start(cap_offset)).is_err()
            || f.read_exact(&mut cap_hdr).is_err()
        {
            break;
        }
        let cap_id = cap_hdr[0];
        let next_ptr = cap_hdr[1] & 0xFC;

        if cap_id == 0x05 {
            let msg_ctrl = u16::from_le_bytes([cap_hdr[2], cap_hdr[3]]);
            if msg_ctrl & 0x0001 != 0 {
                let new_ctrl = msg_ctrl & !0x0001;
                let ctrl_offset = cap_offset + 2;
                if f.seek(std::io::SeekFrom::Start(ctrl_offset)).is_ok() {
                    let _ = f.write_all(&new_ctrl.to_le_bytes());
                }
                eprintln!(
                    "[MSI] MSI disabled: ctrl 0x{msg_ctrl:04x} → 0x{new_ctrl:04x} at cap offset 0x{cap_offset:02x}"
                );
            } else {
                eprintln!("[MSI] MSI already disabled at cap offset 0x{cap_offset:02x}");
            }
        } else if cap_id == 0x11 {
            let msg_ctrl = u16::from_le_bytes([cap_hdr[2], cap_hdr[3]]);
            let new_ctrl = (msg_ctrl & !0x8000) | 0x4000;
            if new_ctrl != msg_ctrl {
                let ctrl_offset = cap_offset + 2;
                if f.seek(std::io::SeekFrom::Start(ctrl_offset)).is_ok() {
                    let _ = f.write_all(&new_ctrl.to_le_bytes());
                }
            }
            eprintln!(
                "[MSI] MSI-X disabled+masked: ctrl 0x{msg_ctrl:04x} → 0x{new_ctrl:04x} at cap offset 0x{cap_offset:02x}"
            );
        }

        cap_ptr = next_ptr;
    }

    let mut cmd_bytes = [0u8; 2];
    if f.seek(std::io::SeekFrom::Start(4)).is_ok() && f.read_exact(&mut cmd_bytes).is_ok() {
        let old_cmd = u16::from_le_bytes(cmd_bytes);
        let new_cmd = old_cmd | 0x0400;
        let _ = f.seek(std::io::SeekFrom::Start(4));
        let _ = f.write_all(&new_cmd.to_le_bytes());
        eprintln!("[MSI] INTx disabled: cmd 0x{old_cmd:04x} → 0x{new_cmd:04x}");
    }
}

#[cfg(target_os = "linux")]
fn run_card_info(fd: &impl AsFd) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    const CARD_INFO_ENTRY: usize = 72;
    const MAX_CARDS: usize = 8;
    const CARD_INFO_OP: Opcode = iowr(NV_IOCTL_MAGIC, 0xC8, CARD_INFO_ENTRY * MAX_CARDS);
    let mut ci_buf = vec![0u8; CARD_INFO_ENTRY * MAX_CARDS];
    // SAFETY: ci_buf is correctly sized for nv_ioctl_card_info; fd is valid.
    let ioctl = RmRawIoctl::<{ CARD_INFO_OP }> {
        ptr: ci_buf.as_mut_ptr(),
    };
    let (rc, errno) = match unsafe { rustix::ioctl::ioctl(fd, ioctl) } {
        Ok(v) => (v, 0),
        Err(e) => (-1, e.raw_os_error()),
    };
    eprintln!("\n[Diag] NV_ESC_CARD_INFO: rc={rc} errno={errno}");
    for i in 0..MAX_CARDS {
        let off = i * CARD_INFO_ENTRY;
        let valid = ci_buf[off];
        if valid == 0 {
            continue;
        }
        let domain = u32::from_ne_bytes(ne_bytes(&ci_buf[off + 4..off + 8], "domain")?);
        let bus = ci_buf[off + 8];
        let slot = ci_buf[off + 9];
        let func = ci_buf[off + 10];
        let vendor = u16::from_ne_bytes(ne_bytes(&ci_buf[off + 12..off + 14], "vendor")?);
        let devid = u16::from_ne_bytes(ne_bytes(&ci_buf[off + 14..off + 16], "devid")?);
        let gpu_id = u32::from_ne_bytes(ne_bytes(&ci_buf[off + 16..off + 20], "gpu_id")?);
        let irq = u16::from_ne_bytes(ne_bytes(&ci_buf[off + 20..off + 22], "irq")?);
        let reg_addr = u64::from_ne_bytes(ne_bytes(&ci_buf[off + 24..off + 32], "reg_addr")?);
        let reg_size = u64::from_ne_bytes(ne_bytes(&ci_buf[off + 32..off + 40], "reg_size")?);
        let fb_addr = u64::from_ne_bytes(ne_bytes(&ci_buf[off + 40..off + 48], "fb_addr")?);
        let fb_size = u64::from_ne_bytes(ne_bytes(&ci_buf[off + 48..off + 56], "fb_size")?);
        let minor = u32::from_ne_bytes(ne_bytes(&ci_buf[off + 56..off + 60], "minor")?);
        eprintln!(
            "  card[{i}]: valid={valid} {domain:04x}:{bus:02x}:{slot:02x}.{func} vendor=0x{vendor:04x} dev=0x{devid:04x} gpu_id=0x{gpu_id:x} irq={irq} minor={minor}"
        );
        eprintln!("    regs=0x{reg_addr:x}+0x{reg_size:x} fb=0x{fb_addr:x}+0x{fb_size:x}");
    }

    for i in 0..MAX_CARDS {
        let off = i * CARD_INFO_ENTRY;
        let valid = ci_buf[off];
        if valid == 0 {
            continue;
        }
        let domain = u32::from_ne_bytes(ne_bytes(&ci_buf[off + 4..off + 8], "domain")?);
        let bus = ci_buf[off + 8];
        let slot = ci_buf[off + 9];
        let func = ci_buf[off + 10];
        let bdf = format!("{domain:04x}:{bus:02x}:{slot:02x}.{func}");
        let cfg_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(&bdf, "config");
        match std::fs::read(&cfg_path) {
            Ok(cfg) if cfg.len() >= 6 => {
                let cmd = u16::from_le_bytes([cfg[4], cfg[5]]);
                let io_en = cmd & 1;
                let mem_en = (cmd >> 1) & 1;
                let bus_master = (cmd >> 2) & 1;
                eprintln!(
                    "[Diag] PCI CMD({bdf}): 0x{cmd:04x} IO={io_en} MEM={mem_en} BusMaster={bus_master}"
                );
                if bus_master == 0 {
                    eprintln!("  WARNING: BusMaster DISABLED — enabling via sysfs...");
                    let enable_path =
                        toadstool_cylinder::linux_paths::sysfs_pci_device_file(&bdf, "enable");
                    let _ = std::fs::write(&enable_path, "1");
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&cfg_path)
                    {
                        use std::io::{Read, Seek, Write};
                        let mut cmd_bytes = [0u8; 2];
                        let _ = f.seek(std::io::SeekFrom::Start(4));
                        let _ = f.read_exact(&mut cmd_bytes);
                        let new_cmd = u16::from_le_bytes(cmd_bytes) | 0x0004;
                        let _ = f.seek(std::io::SeekFrom::Start(4));
                        let _ = f.write_all(&new_cmd.to_le_bytes());
                        eprintln!("  PCI CMD now: 0x{new_cmd:04x} (BusMaster forced ON)");
                    }
                }
            }
            Ok(cfg) => eprintln!("[Diag] PCI config too short ({} bytes)", cfg.len()),
            Err(e) => eprintln!("[Diag] PCI config read failed ({bdf}): {e}"),
        }
        break;
    }

    Ok(step_json(
        "card_info",
        rc == 0,
        serde_json::json!({"rc": rc, "errno": errno}),
    ))
}

#[cfg(target_os = "linux")]
fn run_attach_gpus_to_fd(fd: &impl AsFd, gpu_id: u32) -> serde_json::Value {
    eprintln!("\n[Phase 0b] NV_ESC_ATTACH_GPUS_TO_FD: gpu_id=0x{gpu_id:x}...");
    let mut attach_buf = [gpu_id];
    const ATTACH_OP: Opcode = iowr(NV_IOCTL_MAGIC, 0xD4, size_of::<[u32; 1]>());
    // SAFETY: attach_buf is [u32; 1] matching kernel ABI; fd is valid.
    let ioctl = RmRawIoctl::<{ ATTACH_OP }> {
        ptr: attach_buf.as_mut_ptr().cast(),
    };
    let (rc, errno) = match unsafe { rustix::ioctl::ioctl(fd, ioctl) } {
        Ok(v) => (v, 0),
        Err(e) => (-1, e.raw_os_error()),
    };
    eprintln!("  ATTACH_GPUS_TO_FD: rc={rc} errno={errno}");
    step_json(
        "attach_gpus_to_fd",
        rc == 0,
        serde_json::json!({
            "gpu_id": format!("0x{gpu_id:x}"),
            "rc": rc,
            "errno": errno,
        }),
    )
}

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: {} <chardev_major> [--channel] [--bdf 0000:XX:YY.Z]",
            args[0]
        );
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

    let bdf = args
        .windows(2)
        .find(|w| w[0] == "--bdf")
        .map(|w| w[1].as_str())
        .unwrap_or_else(|| {
            eprintln!("rm_trigger: --bdf <BDF> is required (e.g. --bdf 0000:49:00.0)");
            std::process::exit(1);
        });

    eprintln!("rm_trigger: major={major}, channel_mode={channel_mode}, bdf={bdf}");
    eprintln!(
        "sizeof(Nvos21Parameters) = {}",
        size_of::<Nvos21Parameters>()
    );
    eprintln!(
        "sizeof(NvChannelAllocParams) = {}",
        size_of::<NvChannelAllocParams>()
    );
    eprintln!("RM_ALLOC_OP = 0x{RM_ALLOC_OP:x}");
    eprintln!("RM_CTRL_OP  = 0x{RM_CTRL_OP:x}");

    let ctl_path = "/dev/toadstool-rm-nvidiactl";
    let gpu_path = "/dev/toadstool-rm-nvidia0";

    let _ = std::fs::remove_file(ctl_path);
    let _ = std::fs::remove_file(gpu_path);

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
    let mut channel_id: Option<u32> = None;
    let mut work_submit_token: Option<u32> = None;

    let pmc_pre = read_pmc_enable(bdf);
    eprintln!(
        "[Diag] PMC_ENABLE before opens: {:?}",
        pmc_pre.map(|v| format!("0x{v:08x}"))
    );

    eprintln!("\nOpening nvidiactl (minor 255) [CTL-first]...");
    let ctl_file = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(ctl_path)
    {
        Ok(f) => {
            eprintln!("  ctl open ok (fd={})", f.as_raw_fd());
            f
        }
        Err(e) => {
            eprintln!("  ctl open failed: {e}");
            steps.push(step_json(
                "open_ctl",
                false,
                serde_json::json!({"error": e.to_string()}),
            ));
            cleanup(ctl_path, gpu_path);
            print_result(&serde_json::json!({"success": false, "major": major, "steps": steps}));
            return ExitCode::from(1);
        }
    };
    let fd = &ctl_file;

    let pmc_post_ctl = read_pmc_enable(bdf);
    eprintln!(
        "[Diag] PMC_ENABLE after CTL open: {:?}",
        pmc_post_ctl.map(|v| format!("0x{v:08x}"))
    );

    eprintln!("\nOpening GPU device (minor 0) — triggers rm_init_adapter...");
    let gpu_fd = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(gpu_path);
    match &gpu_fd {
        Ok(f) => eprintln!("  GPU open ok (fd={})", f.as_raw_fd()),
        Err(e) => {
            eprintln!("  GPU open FAILED: {e}");
            steps.push(step_json(
                "gpu_open",
                false,
                serde_json::json!({"error": e.to_string()}),
            ));
        }
    }

    let pmc_post_gpu = read_pmc_enable(bdf);
    eprintln!(
        "[Diag] PMC_ENABLE after GPU open: {:?}",
        pmc_post_gpu.map(|v| format!("0x{v:08x}"))
    );
    if pmc_pre == pmc_post_gpu {
        eprintln!("  ⚠ PMC_ENABLE UNCHANGED — rm_init_adapter likely did NOT run DEVINIT");
    } else {
        eprintln!("  ✓ PMC_ENABLE CHANGED — rm_init_adapter ran DEVINIT");
    }

    if let Ok(ref gpu_file) = gpu_fd {
        diag_gpu_fd_root_alloc(gpu_file);
    }

    match run_card_info(fd) {
        Ok(step) => steps.push(step),
        Err(e) => {
            eprintln!("[Diag] NV_ESC_CARD_INFO parse failed: {e}");
            steps.push(step_json(
                "card_info",
                false,
                serde_json::json!({"error": e.to_string()}),
            ));
            success = false;
        }
    }

    let root_result = alloc_root_client(fd);
    let RootClientResult {
        root_test,
        rm_root,
        gpu_id,
        step,
        ..
    } = root_result;
    steps.push(step);

    let pmc_post_root = read_pmc_enable(bdf);
    eprintln!(
        "[Diag] PMC_ENABLE after root alloc: {:?}",
        pmc_post_root.map(|v| format!("0x{v:08x}"))
    );

    pre_attach_diagnostics(fd, rm_root);
    let pmc_now = read_pmc_enable(bdf);
    eprintln!(
        "[Diag] PMC_ENABLE after probed_ids query: {:?}",
        pmc_now.map(|v| format!("0x{v:08x}"))
    );

    if gpu_id != 0 {
        steps.push(gpu_attach_ids_ctrl(fd, rm_root, gpu_id));
    }

    post_attach_diagnostics(fd, rm_root);

    if gpu_id != 0 {
        let attach_step = run_attach_gpus_to_fd(fd, gpu_id);
        if !attach_step["ok"].as_bool().unwrap_or(false) {
            eprintln!("  CRITICAL: ATTACH_GPUS_TO_FD failed — rm_init_adapter will not run!");
            success = false;
        }
        steps.push(attach_step);
    } else {
        eprintln!("\n[Phase 0b] No GPU IDs from GET_ATTACHED_IDS — cannot attach");
        steps.push(step_json(
            "attach_gpus_to_fd",
            false,
            serde_json::json!({
                "error": "no gpu_ids available"
            }),
        ));
        success = false;
    }

    let root_ok = root_test.rc == 0 && root_test.status == 0;
    if !root_ok {
        success = false;
    }

    let (core_handles, core_steps) = alloc_core_tree(fd, rm_root, gpu_id, root_ok, &mut success);
    steps.extend(core_steps);

    if root_ok && channel_mode {
        let (channel_result, channel_steps) = alloc_compute_channel(
            fd,
            rm_root,
            core_handles.rm_device,
            core_handles.rm_subdevice,
            &mut success,
        );
        channel_id = channel_result.channel_id;
        work_submit_token = channel_result.work_submit_token;
        steps.extend(channel_steps);
    }

    let hold_secs = if channel_mode { 3 } else { 5 };
    eprintln!("\nHolding fds open for {hold_secs}s...");
    std::thread::sleep(std::time::Duration::from_secs(hold_secs));

    eprintln!("\n[SAFETY] Quenching GPU interrupts before fd close...");
    quench_gpu_interrupts(bdf);
    disable_pci_msi_config(bdf);

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

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This tool requires Linux");
    std::process::exit(1);
}
