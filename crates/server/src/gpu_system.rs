// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU System Query Helpers
//!
//! Standalone functions for querying GPU devices, memory, and available backends.
//! Detects NVIDIA GPUs via /proc on Linux, falls back to wgpu adapter enumeration
//! on all platforms. Backend discovery is capability-based.

/// Query available GPU devices.
///
/// Linux: detects NVIDIA GPUs via `/proc/driver/nvidia/gpus`.
/// All platforms: falls back to wgpu adapter enumeration for cross-platform coverage.
#[must_use]
pub fn query_gpu_devices() -> Vec<serde_json::Value> {
    let mut devices = Vec::new();

    #[cfg(target_os = "linux")]
    {
        devices.extend(discover_via_sysfs());
    }

    #[cfg(feature = "gpu-discovery")]
    if devices.is_empty() {
        discover_via_wgpu(&mut devices);
    }

    #[cfg(not(feature = "gpu-discovery"))]
    if devices.is_empty() {
        devices.push(serde_json::json!({
            "index": 0, "id": "none", "backend": "cpu",
            "note": "No GPU detected; gpu-discovery feature not enabled",
        }));
    }

    devices
}

/// Enumerate accelerators from sysfs, whatever driver is bound.
///
/// This replaced a listing of `/proc/driver/nvidia/gpus`, which is populated
/// only by the proprietary NVIDIA module. That found no AMD or Intel GPU at
/// all, and among NVIDIA cards found only those bound to that one driver — on
/// biomeGate, one of four, missing an unbound Titan V and both `vfio-pci`
/// Tesla K80 dies. It also reported the directory name as the device `id` and
/// nothing else, so callers got a PCI address with no model, vendor, or
/// indication of whether the device was answering.
///
/// Liveness is included because it is the one thing a caller cannot recover
/// on its own: a wedged GPU is enumerated, bound, and completely silent.
#[cfg(target_os = "linux")]
fn discover_via_sysfs() -> Vec<serde_json::Value> {
    use toadstool_cylinder::vfio::pci_discovery::{Liveness, scan_accelerators};

    scan_accelerators()
        .into_iter()
        .enumerate()
        .map(|(idx, accel)| {
            serde_json::json!({
                "index": idx,
                "id": accel.bdf(),
                "vendor_id": format!("{:#06x}", accel.device.vendor_id),
                "device_id": format!("{:#06x}", accel.device.device_id),
                "class_code": format!("{:#08x}", accel.device.class_code),
                "backend": accel.device.driver.as_deref().unwrap_or("unbound"),
                "responding": accel.liveness == Liveness::Responding,
            })
        })
        .collect()
}

/// Enumerate GPU adapters via wgpu (cross-platform: Vulkan, DX12, Metal).
#[cfg(feature = "gpu-discovery")]
fn discover_via_wgpu(devices: &mut Vec<serde_json::Value>) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapters =
        futures::executor::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    for (idx, adapter) in adapters.iter().enumerate() {
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu {
            continue;
        }
        devices.push(serde_json::json!({
            "index": idx,
            "id": format!("{:#06x}:{:#06x}", info.vendor, info.device),
            "name": info.name,
            "backend": format!("{:?}", info.backend),
            "device_type": format!("{:?}", info.device_type),
            "driver": if info.driver.is_empty() { None } else { Some(&info.driver) },
        }));
    }
}

/// Query GPU memory usage via `nvidia-smi` (cross-platform — works on Linux and Windows).
#[must_use]
pub fn query_gpu_memory() -> Vec<serde_json::Value> {
    let mut devices = Vec::new();

    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,memory.total,memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
            if parts.len() >= 4 {
                devices.push(serde_json::json!({
                    "index": parts[0], "total_mb": parts[1],
                    "used_mb": parts[2], "free_mb": parts[3],
                }));
            }
        }
    }

    if devices.is_empty() {
        devices.push(serde_json::json!({
            "note": "GPU memory query requires nvidia-smi (available on Linux and Windows with NVIDIA drivers)",
        }));
    }

    devices
}

/// Discover available compute backends at runtime.
///
/// Probes the host for GPU API availability rather than returning a
/// hardcoded list. Capability-based: only reports backends that are
/// actually present on this system.
#[must_use]
pub fn query_available_backends() -> Vec<&'static str> {
    let mut backends = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/proc/driver/nvidia").exists()
            || std::fs::read_to_string("/proc/modules").is_ok_and(|m| m.contains("nvidia"))
        {
            backends.push("vulkan");
        }

        if std::path::Path::new("/dev/dri").exists() && !backends.contains(&"vulkan") {
            backends.push("vulkan");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if std::path::Path::new("/System/Library/Frameworks/Metal.framework").exists() {
            backends.push("metal");
        }
    }

    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new(r"C:\Windows\System32\d3d12.dll").exists() {
            backends.push("dx12");
        }
        if std::path::Path::new(r"C:\Windows\System32\vulkan-1.dll").exists() {
            backends.push("vulkan");
        }
    }

    if backends.is_empty() {
        backends.push("wgpu-auto");
    }

    backends
}

/// Query SPIR-V codegen safety information for discovered GPUs.
///
/// Returns driver classification, poisoning risk, and transcendental
/// safety per detected GPU so springs can make precision routing
/// decisions without local probing (which risks device poisoning).
///
/// Root cause: naga SPIR-V codegen (not NVVM) — renamed per hotSpring v0.6.30.
/// Absorbed from hotSpring v0.6.26 requirement: expose
/// `nvvm_transcendental_risk` in runtime discovery.
#[must_use]
pub fn query_spirv_codegen_safety() -> serde_json::Value {
    let sysmon_gpus = toadstool_sysmon::discover_gpus();
    let mut entries = Vec::new();

    for gpu in &sysmon_gpus {
        let driver = gpu.driver.as_str();
        let is_nvk = driver.contains("nvk") || driver.contains("nouveau");
        let is_radv = driver.contains("radv");
        let is_nvidia_prop = driver.contains("nvidia") && !is_nvk;

        let poisoning_risk = if is_nvk || is_radv {
            "none"
        } else if is_nvidia_prop {
            "transcendental_only"
        } else {
            "unknown"
        };

        let transcendentals_safe = is_nvk || is_radv;

        let safe_tiers = if is_nvk || is_radv {
            serde_json::json!(["F32", "F64", "F64Precise", "DF64"])
        } else if is_nvidia_prop {
            serde_json::json!(["F32", "F64"])
        } else {
            serde_json::json!(["F32"])
        };

        entries.push(serde_json::json!({
            "card_index": gpu.card_index,
            "pci_slot": gpu.pci_slot,
            "driver": driver,
            "nvvm_poisoning_risk": poisoning_risk,
            "nvvm_transcendental_risk": !transcendentals_safe && is_nvidia_prop,
            "f64_transcendentals_safe": transcendentals_safe,
            "df64_transcendentals_safe": transcendentals_safe,
            "safe_tiers": safe_tiers,
        }));
    }

    if entries.is_empty() {
        return serde_json::json!({
            "note": "No GPUs detected for NVVM safety classification",
            "devices": [],
        });
    }

    serde_json::json!({
        "devices": entries,
    })
}

/// Query firmware inventory for detected NVIDIA GPUs.
#[must_use]
pub fn query_firmware_inventory() -> serde_json::Value {
    #[cfg(not(target_os = "linux"))]
    {
        return serde_json::json!({
            "note": "Firmware inventory is only available on Linux",
            "devices": [],
        });
    }

    #[cfg(target_os = "linux")]
    {
        let sysmon_gpus = toadstool_sysmon::discover_gpus();
        let mut entries = Vec::new();

        for gpu in &sysmon_gpus {
            let driver = gpu.driver.as_str();
            let is_nvidia =
                driver.contains("nvidia") || driver.contains("nvk") || driver.contains("nouveau");

            if is_nvidia {
                let chip = infer_chip_codename(&gpu.pci_slot);
                let inv = nvpmu::FirmwareInventory::probe(&chip);
                entries.push(serde_json::json!({
                    "card_index": gpu.card_index,
                    "pci_slot": gpu.pci_slot,
                    "chip": chip,
                    "compute_viable": inv.compute_viable(),
                    "compute_blockers": inv.compute_blockers(),
                    "needs_software_pmu": inv.needs_software_pmu(),
                    "firmware": serde_json::to_value(&inv).unwrap_or_default(),
                }));
            } else {
                entries.push(serde_json::json!({
                    "card_index": gpu.card_index,
                    "pci_slot": gpu.pci_slot,
                    "note": "Firmware inventory only supported for NVIDIA GPUs",
                }));
            }
        }

        serde_json::json!({ "devices": entries })
    }
}

/// Infer chip codename from PCI device ID via sysfs.
#[cfg(target_os = "linux")]
fn infer_chip_codename(pci_slot: &str) -> String {
    let device_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(pci_slot, "device");
    let device_id = std::fs::read_to_string(&device_path)
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();

    match device_id.as_str() {
        // Volta
        s if s.starts_with("0x1d") => "gv100".to_string(),
        // Turing
        s if s.starts_with("0x1e") || s.starts_with("0x1f") => "tu102".to_string(),
        // Ampere
        s if s.starts_with("0x22") || s.starts_with("0x20") => "ga102".to_string(),
        // Ada Lovelace
        s if s.starts_with("0x26") || s.starts_with("0x27") => "ad102".to_string(),
        _ => format!("unknown-{}", device_id.trim_start_matches("0x")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_gpu_devices_returns_at_least_one() {
        let devices = query_gpu_devices();
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_query_gpu_memory_returns_at_least_one() {
        let memory = query_gpu_memory();
        assert!(!memory.is_empty());
    }

    #[test]
    fn test_query_available_backends_returns_at_least_one() {
        let backends = query_available_backends();
        assert!(!backends.is_empty());
    }

    #[test]
    fn test_query_spirv_codegen_safety_returns_valid_json() {
        let safety = query_spirv_codegen_safety();
        assert!(safety.get("devices").is_some() || safety.get("note").is_some());
    }

    #[test]
    fn test_query_firmware_inventory_returns_valid_json() {
        let inv = query_firmware_inventory();
        assert!(inv.get("devices").is_some());
    }
}

#[cfg(all(test, target_os = "linux"))]
mod sysfs_discovery_tests {
    use super::*;

    /// Every device reported must carry the fields callers rely on, and the
    /// enumeration must be total on hosts with no GPUs.
    #[test]
    fn discovery_reports_complete_records() {
        for d in discover_via_sysfs() {
            assert!(d.get("id").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty()));
            assert!(d.get("vendor_id").is_some());
            assert!(d.get("backend").is_some());
            assert!(
                d.get("responding").and_then(serde_json::Value::as_bool).is_some(),
                "liveness must always be stated, never omitted"
            );
        }
    }

    /// Shows what the server reports on this machine.
    #[test]
    fn show_discovered() {
        for d in discover_via_sysfs() {
            eprintln!("  {d}");
        }
    }
}
