// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from kernel_sentinel.rs (S333).

use super::kernel_sentinel::*;

#[test]
fn classify_critical_oops() {
    assert_eq!(
        classify_line("Oops: 0002 [#1] SMP NOPTI"),
        Severity::Critical
    );
}

#[test]
fn classify_critical_bug() {
    assert_eq!(
        classify_line("BUG: unable to handle page request at ffffffff"),
        Severity::Critical
    );
}

#[test]
fn classify_critical_rip() {
    assert_eq!(
        classify_line("RIP: 0010:nv_rm_isr+0x2a/0x40 [nvidia]"),
        Severity::Critical
    );
}

#[test]
fn classify_critical_call_trace() {
    assert_eq!(classify_line("Call Trace:"), Severity::Critical);
}

#[test]
fn classify_critical_kernel_panic() {
    assert_eq!(
        classify_line("Kernel panic - not syncing: Fatal exception in interrupt"),
        Severity::Critical
    );
}

#[test]
fn classify_critical_null_pointer() {
    assert_eq!(
        classify_line("unable to handle kernel NULL pointer dereference at 0000000000000008"),
        Severity::Critical
    );
}

#[test]
fn classify_critical_gpf() {
    assert_eq!(
        classify_line("general protection fault, probably for non-canonical address"),
        Severity::Critical
    );
}

#[test]
fn classify_critical_stack_protector() {
    assert_eq!(
        classify_line("stack-protector: Kernel stack is corrupted in: nv_api_call_kernel_isr"),
        Severity::Critical
    );
}

#[test]
fn classify_gpu_warn_nvrm() {
    assert_eq!(
        classify_line("NVRM: Xid (PCI:0000:01:00): 79, pid=1234"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_gpu_warn_nvidia_module() {
    assert_eq!(
        classify_line("nvidia: loading out-of-tree module taints kernel"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_gpu_warn_nvsov() {
    assert_eq!(
        classify_line("nvsov: module license 'NVIDIA' taints kernel"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_gpu_warn_vfio_pci() {
    assert_eq!(
        classify_line("vfio-pci 0000:01:00.0: enabling device"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_gpu_warn_aer() {
    assert_eq!(
        classify_line("AER: Uncorrected (Non-Fatal) error received"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_gpu_warn_pcie_bus_error() {
    assert_eq!(
        classify_line("PCIe Bus Error: severity=Uncorrected (Non-Fatal)"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_gpu_warn_iommu_fault() {
    assert_eq!(
        classify_line("iommu fault: domain 0 addr 0xdead0000"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_gpu_warn_irq_domain() {
    assert_eq!(
        classify_line("irq_domain_remove: mapping still active"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_gpu_warn_msi_release() {
    assert_eq!(
        classify_line("msi_device_data_release called with active descriptors"),
        Severity::GpuWarn
    );
}

#[test]
fn classify_normal_harmless() {
    assert_eq!(
        classify_line("usb 1-2: new high-speed USB device"),
        Severity::Normal
    );
    assert_eq!(classify_line("wlan0: associated"), Severity::Normal);
    assert_eq!(classify_line("EXT4-fs (sda1): mounted"), Severity::Normal);
}

#[test]
fn classify_empty_line() {
    assert_eq!(classify_line(""), Severity::Normal);
}

#[test]
fn classify_critical_takes_priority_over_gpu_warn() {
    assert_eq!(classify_line("BUG: NVRM: corrupted"), Severity::Critical);
}

#[test]
fn parse_kmsg_standard_format() {
    assert_eq!(
        parse_kmsg_message("6,1234,5678901,-;usb 1-2: new device"),
        "usb 1-2: new device"
    );
}

#[test]
fn parse_kmsg_no_semicolon() {
    assert_eq!(
        parse_kmsg_message("plain text without separator"),
        "plain text without separator"
    );
}

#[test]
fn parse_kmsg_empty() {
    assert_eq!(parse_kmsg_message(""), "");
}

#[test]
fn parse_kmsg_multiple_semicolons() {
    assert_eq!(
        parse_kmsg_message("6,1234,5678901,-;message;with;semicolons"),
        "message;with;semicolons"
    );
}

#[test]
fn crash_report_dir_returns_nonempty() {
    let dir = crash_report_dir();
    assert!(!dir.is_empty());
    assert!(dir.contains("crash-reports"));
}

#[test]
fn severity_equality() {
    assert_eq!(Severity::Critical, Severity::Critical);
    assert_eq!(Severity::GpuWarn, Severity::GpuWarn);
    assert_eq!(Severity::Normal, Severity::Normal);
    assert_ne!(Severity::Critical, Severity::GpuWarn);
    assert_ne!(Severity::Critical, Severity::Normal);
    assert_ne!(Severity::GpuWarn, Severity::Normal);
}

#[test]
fn all_crash_patterns_are_detected() {
    for pat in CRASH_PATTERNS {
        let line = format!("some prefix {pat} some suffix");
        assert_eq!(
            classify_line(&line),
            Severity::Critical,
            "CRASH_PATTERNS entry '{pat}' was not detected"
        );
    }
}

#[test]
fn all_gpu_warn_patterns_are_detected() {
    for pat in GPU_WARN_PATTERNS {
        let line = format!("prefix {pat} suffix");
        assert_eq!(
            classify_line(&line),
            Severity::GpuWarn,
            "GPU_WARN_PATTERNS entry '{pat}' was not detected"
        );
    }
}
