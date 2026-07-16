// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from sysfs_executor.rs (S334).

use super::sysfs_executor::*;
use crate::device_id::DeviceId;
use crate::swap::SwapExecutor;

#[test]
fn bdf_from_pci_device_id() {
    let id = DeviceId::PciBdf("0000:01:00.0".into());
    assert_eq!(SysfsSwapExecutor::bdf_from_id(&id).unwrap(), "0000:01:00.0");
}

#[test]
fn bdf_from_non_pci_errors() {
    let id = DeviceId::UsbPath("1-2".into());
    assert!(SysfsSwapExecutor::bdf_from_id(&id).is_err());
}

#[test]
fn driver_name_mapping() {
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("vfio"),
        "vfio-pci"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("vfio-pci"),
        "vfio-pci"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("nouveau"),
        "nouveau"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("nvidia-open"),
        "nvidia"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("amdgpu"),
        "amdgpu"
    );
    assert_eq!(SysfsSwapExecutor::driver_name_for_target("xe"), "xe");
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("akida"),
        "akida-pcie"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("custom"),
        "custom"
    );
}

#[test]
fn sysfs_swap_error_display() {
    let err = SysfsSwapError::NotPciBdf("usb:1-2".into());
    assert!(err.to_string().contains("PCI BDF"));

    let err = SysfsSwapError::SysfsWrite {
        path: "/sys/foo".into(),
        reason: "permission denied".into(),
    };
    assert!(err.to_string().contains("/sys/foo"));

    let err = SysfsSwapError::BindFailed {
        bdf: "0000:01:00.0".into(),
        driver: "vfio-pci".into(),
    };
    assert!(err.to_string().contains("vfio-pci"));
}

#[test]
fn warm_preserving_swap_detection() {
    assert!(SysfsSwapExecutor::is_warm_preserving_swap(
        "nouveau", "vfio-pci"
    ));
    assert!(SysfsSwapExecutor::is_warm_preserving_swap(
        "nvidia", "vfio-pci"
    ));
    assert!(SysfsSwapExecutor::is_warm_preserving_swap(
        "nvsov", "vfio-pci"
    ));
    assert!(SysfsSwapExecutor::is_warm_preserving_swap(
        "nvsov2", "vfio-pci"
    ));
    assert!(SysfsSwapExecutor::is_warm_preserving_swap(
        "amdgpu", "vfio-pci"
    ));
    assert!(SysfsSwapExecutor::is_warm_preserving_swap("xe", "vfio-pci"));
    assert!(!SysfsSwapExecutor::is_warm_preserving_swap(
        "vfio-pci", "nouveau"
    ));
    assert!(!SysfsSwapExecutor::is_warm_preserving_swap(
        "vfio-pci", "vfio-pci"
    ));
    assert!(!SysfsSwapExecutor::is_warm_preserving_swap(
        "nouveau", "nvidia"
    ));
    assert!(!SysfsSwapExecutor::is_warm_preserving_swap(
        "unbound", "vfio-pci"
    ));
}

#[tokio::test]
async fn release_nonexistent_device_is_noop() {
    let exec = SysfsSwapExecutor;
    let id = DeviceId::PciBdf("ffff:ff:ff.f".into());
    let result = exec.release(&id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn execute_swap_nonexistent_device_errors() {
    let exec = SysfsSwapExecutor;
    let id = DeviceId::PciBdf("ffff:ff:ff.f".into());
    let result = exec.execute_swap(&id, "vfio-pci").await;
    // No driver bound on nonexistent device, so it tries to write
    // driver_override and drivers_probe — which should fail (not hang)
    assert!(result.is_err());
}

#[tokio::test]
async fn execute_swap_unbound_target_succeeds_on_nonexistent() {
    let exec = SysfsSwapExecutor;
    let id = DeviceId::PciBdf("ffff:ff:ff.f".into());
    // "unbound" target skips bind, so the swap is a no-op
    let result = exec.execute_swap(&id, "unbound").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().to, "unbound");
}

#[test]
fn halt_result_records_correct_step() {
    let steps = vec![crate::warm_init::WarmInitStep {
        name: "seeder_bind".into(),
        ok: false,
        detail: Some("guarded drivers_probe failed: timeout".into()),
        duration_ms: 30000,
    }];
    let result = halt_result(
        "0000:02:00.0",
        "nouveau",
        "seeder_bind",
        steps,
        std::time::Instant::now(),
    );
    assert!(!result.success);
    assert_eq!(result.halted_at.as_deref(), Some("seeder_bind"));
    assert_eq!(result.seeder_used, "nouveau");
    assert!(!result.warm_preserved);
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.steps[0].name, "seeder_bind");
    assert!(!result.steps[0].ok);
}

#[test]
fn driver_name_mapping_exhaustive() {
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("vfio"),
        "vfio-pci"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("vfio-pci"),
        "vfio-pci"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("nouveau"),
        "nouveau"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("nvidia"),
        "nvidia"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("nvidia-open"),
        "nvidia"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("nvidia_open"),
        "nvidia"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("amdgpu"),
        "amdgpu"
    );
    assert_eq!(SysfsSwapExecutor::driver_name_for_target("xe"), "xe");
    assert_eq!(SysfsSwapExecutor::driver_name_for_target("i915"), "i915");
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("akida"),
        "akida-pcie"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("akida-pcie"),
        "akida-pcie"
    );
    assert_eq!(
        SysfsSwapExecutor::driver_name_for_target("unknown_driver"),
        "unknown_driver"
    );
}
