// SPDX-License-Identifier: AGPL-3.0-or-later
// SAFETY: /dev/kmsg seek/read require BorrowedFd::borrow_raw on an owned fd.
#![allow(unsafe_code)]
//! Kernel oops sentinel — diesel engine crash forensics.
//!
//! Monitors `/dev/kmsg` (kernel log ring buffer) in real-time for signs of
//! kernel distress: oops, panic, BUG, RIP, or GPU-related faults. When a
//! crash signature is detected, the sentinel immediately:
//!
//! 1. Snapshots all GPU-related state it can safely read
//! 2. Writes a crash triage report to persistent storage
//! 3. Attempts emergency interrupt quench on any active handoff GPU
//!
//! This runs as an OS thread (not tokio) to survive scheduler stalls during
//! kernel corruption. It uses raw `/dev/kmsg` reads which are non-blocking
//! and survive most kernel failures short of a triple fault.

use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{error, info, warn};

fn crash_report_dir() -> String {
    toadstool_cylinder::linux_paths::data_subdir("crash-reports")
}

static SENTINEL_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Patterns in kernel log lines that indicate the kernel is crashing.
///
/// IMPORTANT: these are substring matches against raw kmsg lines. Every
/// normal kernel message from the nvsov module (e.g. "nvsov: module license
/// 'NVIDIA' taints kernel") would match a bare "nvsov" pattern — triggering
/// emergency quench during RM init and corrupting GPU state. Only include
/// patterns that unambiguously indicate a kernel crash.
const CRASH_PATTERNS: &[&str] = &[
    "Oops:",
    "BUG:",
    "general protection fault",
    "RIP:",
    "Call Trace:",
    "Kernel panic",
    "unable to handle page request",
    "unable to handle kernel NULL pointer",
    "stack-protector:",
];

/// Patterns that are GPU-related but not necessarily fatal — logged as warnings.
/// irq_domain_remove and msi_device_data_release are kernel WARNINGs from
/// stale MSI state cleanup, not crashes. nvsov matches normal module messages.
const GPU_WARN_PATTERNS: &[&str] = &[
    "NVRM:",
    "nvidia:",
    "nvsov",
    "vfio-pci",
    "iommu fault",
    "AER:",
    "PCIe Bus Error",
    "irq_domain_remove",
    "msi_device_data_release",
];

/// Classify a kernel log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    /// Kernel is crashing — save everything now
    Critical,
    /// GPU-related warning — log but don't panic
    GpuWarn,
    /// Not interesting
    Normal,
}

fn classify_line(line: &str) -> Severity {
    for pat in CRASH_PATTERNS {
        if line.contains(pat) {
            return Severity::Critical;
        }
    }
    for pat in GPU_WARN_PATTERNS {
        if line.contains(pat) {
            return Severity::GpuWarn;
        }
    }
    Severity::Normal
}

/// Capture a crash triage report — gather as much state as possible before
/// the system goes down.
fn save_crash_report(trigger_line: &str, recent_lines: &[String]) {
    use std::fmt::Write;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let crash_dir = crash_report_dir();
    let _ = std::fs::create_dir_all(&crash_dir);

    let report_path = format!("{}/crash-{}.txt", crash_dir, timestamp);
    let mut report = String::with_capacity(8192);

    report.push_str("=== DIESEL ENGINE CRASH REPORT ===\n");
    let _ = writeln!(report, "Timestamp: {timestamp}");
    let _ = writeln!(report, "Trigger:   {trigger_line}\n");

    report.push_str("=== KERNEL LOG CONTEXT (last 50 lines) ===\n");
    for line in recent_lines.iter().rev().take(50).rev() {
        report.push_str(line);
        report.push('\n');
    }
    report.push('\n');

    report.push_str("=== MODULE STATE ===\n");
    for name in &["nvsov", "nvidia", "nvidia_uvm", "nvidia_modeset", "nvidia_drm", "vfio_pci", "no_bus_reset"] {
        if let Some(snap) = toadstool_cylinder::vfio::guarded_sysfs::module_snapshot(name) {
            let _ = writeln!(report, "{name}: {snap}");
        }
    }
    report.push('\n');

    report.push_str("=== GPU REGISTER SNAPSHOT (best-effort) ===\n");
    let gpu_bdfs = discover_gpu_bdfs();
    for bdf in &gpu_bdfs {
        let _ = writeln!(report, "--- {bdf} ---");
        let resource_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "resource0");
        match std::fs::metadata(&resource_path) {
            Ok(_) => {
                match read_gpu_registers_safe(bdf) {
                    Some(regs) => {
                        for (name, val) in &regs {
                            let _ = writeln!(report, "  {name}: 0x{val:08x}");
                        }
                    }
                    None => report.push_str("  (BAR0 read failed — GPU may be owned by vfio-pci or dead)\n"),
                }
            }
            Err(_) => report.push_str("  (resource0 not accessible)\n"),
        }
    }
    report.push('\n');

    report.push_str("=== PCI CONFIG (best-effort) ===\n");
    for bdf in &gpu_bdfs {
        let config_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "config");
        if let Ok(data) = std::fs::read(&config_path)
            && data.len() >= 8 {
                let cmd = u16::from_le_bytes([data[4], data[5]]);
                let status = u16::from_le_bytes([data[6], data[7]]);
                let _ = writeln!(report, "{bdf}: CMD=0x{cmd:04x} STATUS=0x{status:04x}");
            }
    }

    // Write the report
    match std::fs::write(&report_path, &report) {
        Ok(()) => error!(path = report_path.as_str(), "SENTINEL: crash report saved"),
        Err(e) => error!(error = %e, "SENTINEL: failed to save crash report"),
    }

    // Also attempt emergency quench if a handoff is active
    if super::catalyst_watchdog::is_active() {
        warn!("SENTINEL: kernel crash detected during active handoff — triggering emergency quench");
        super::catalyst_watchdog::force_emergency_quench();
    }
}

/// Discover all NVIDIA GPU BDFs on this machine via sysfs scan.
/// Best-effort: returns empty vec if sysfs is unavailable.
fn discover_gpu_bdfs() -> Vec<String> {
    let filter = toadstool_common::pci_discovery::PciFilter {
        vendor_id: Some(0x10de),
        class_match: Some(Box::new(|c| {
            let masked = c & 0x00FF_FF00;
            masked == 0x0003_0000 || masked == 0x0003_0200
        })),
        ..Default::default()
    };
    toadstool_common::pci_discovery::discover_pci_devices(&filter)
        .into_iter()
        .map(|d| d.bdf)
        .collect()
}

/// Best-effort BAR0 register read — returns None if anything goes wrong.
fn read_gpu_registers_safe(bdf: &str) -> Option<Vec<(&'static str, u32)>> {
    const BAR0_SIZE: usize = 0x100_0000;
    toadstool_cylinder::vfio::sysfs_bar0::read_registers_best_effort(
        bdf,
        BAR0_SIZE,
        &[
            ("BOOT0", 0x000),
            ("INTR_0", 0x100),
            ("INTR_EN_0", 0x140),
            ("PMC_ENABLE", 0x200),
            ("FECS_CPUCTL", 0x40_9100),
        ],
    )
}

/// Start the kernel sentinel background thread. Call once at daemon startup.
pub fn start_sentinel_thread() {
    if SENTINEL_ACTIVE.swap(true, Ordering::SeqCst) {
        return; // already running
    }

    std::thread::Builder::new()
        .name("kernel-sentinel".into())
        .spawn(move || {
            // Wrap entire thread body in catch_unwind to detect silent panics
            let result = std::panic::catch_unwind(|| {
            use rustix::fd::BorrowedFd;
            use std::os::unix::io::AsRawFd;

            info!("kernel sentinel: thread spawned, opening /dev/kmsg");

            // /dev/kmsg is a special device: each read() returns exactly one
            // complete log record. We must use raw read() — BufReader breaks it.
            let kmsg_fd = match std::fs::OpenOptions::new().read(true).open("/dev/kmsg") {
                Ok(f) => f,
                Err(e) => {
                    error!(error = %e, "SENTINEL: cannot open /dev/kmsg — sentinel disabled");
                    SENTINEL_ACTIVE.store(false, Ordering::SeqCst);
                    return;
                }
            };

            // Seek to end so we only see new messages
            let raw_fd = kmsg_fd.as_raw_fd();
            // SAFETY: we own kmsg_fd and it outlives this borrow
            let borrowed = unsafe { BorrowedFd::borrow_raw(raw_fd) };
            let _ = rustix::fs::seek(borrowed, rustix::fs::SeekFrom::End(0));

            info!("kernel sentinel thread started — monitoring /dev/kmsg");

            let mut buf = vec![0u8; 8192];
            let mut recent_lines: Vec<String> = Vec::with_capacity(64);
            let mut critical_count = 0u32;
            let mut report_saved = false;

            loop {
                // SAFETY: we own kmsg_fd and it outlives this borrow
                let borrowed = unsafe { BorrowedFd::borrow_raw(raw_fd) };
                // Blocking read — returns one complete kmsg record per call
                let n = match rustix::io::read(borrowed, &mut buf) {
                    Ok(n) if n > 0 => n,
                    Err(e) if e == rustix::io::Errno::PIPE => {
                        // Ring buffer wrapped — records were lost, keep going
                        continue;
                    }
                    Err(e) if e == rustix::io::Errno::INVAL => {
                        // Buffer too small for record (unlikely at 8K)
                        continue;
                    }
                    _ => {
                        // Other error or EOF — brief sleep to avoid spin
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                };

                let line = String::from_utf8_lossy(&buf[..n]);
                let line = line.trim_end();

                // /dev/kmsg format: "priority,sequence,timestamp,-;message"
                let msg = line.split_once(';').map_or(line, |x| x.1);

                // Keep a rolling buffer of recent lines
                if recent_lines.len() >= 64 {
                    recent_lines.remove(0);
                }
                recent_lines.push(msg.to_string());

                match classify_line(msg) {
                    Severity::Critical => {
                        critical_count += 1;
                        error!(line = msg, count = critical_count, "SENTINEL: kernel crash signature detected");

                        if !report_saved {
                            save_crash_report(msg, &recent_lines);
                            report_saved = true;
                        }

                        if critical_count >= 20 {
                            error!("SENTINEL: 20+ crash signatures — system is likely dead. Stopping sentinel.");
                            break;
                        }
                    }
                    Severity::GpuWarn => {
                        warn!(line = msg, "SENTINEL: GPU-related kernel warning");
                    }
                    Severity::Normal => {}
                }
            }

            }); // end catch_unwind closure

            if let Err(e) = result {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                error!(panic = msg.as_str(), "SENTINEL: thread panicked");
            }

            SENTINEL_ACTIVE.store(false, Ordering::SeqCst);
            info!("kernel sentinel thread exited");
        })
        .expect("failed to spawn kernel sentinel thread");
}
