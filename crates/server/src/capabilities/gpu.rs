// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU device detection via platform-specific self-knowledge
//!
//! Detects GPUs via:
//! - Linux: `/sys/class/drm` for all GPUs, `/proc/driver/nvidia` for NVIDIA details
//! - macOS: System Profiler for Apple Silicon / discrete GPUs

use super::GpuDevice;
use tracing::info;

/// Parse `/proc/driver/nvidia/gpus/.../information` and return the model name.
#[cfg(any(test, target_os = "linux"))]
fn parse_nvidia_information(contents: &str) -> Option<String> {
    let mut model = None;
    for line in contents.lines() {
        if line.starts_with("Model:") {
            let name = line.trim_start_matches("Model:").trim();
            if !name.is_empty() {
                model = Some(name.to_string());
            }
        }
    }
    model
}

#[cfg(any(test, target_os = "linux"))]
fn parse_pci_hex_field(s: &str) -> Option<u32> {
    let s = s.trim();
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u32::from_str_radix(s, 16).ok()
}

/// Parse DRM `uevent` text and return PCI vendor and device IDs from `PCI_ID=`.
#[cfg(any(test, target_os = "linux"))]
fn parse_drm_uevent(contents: &str) -> Option<(u32, u32)> {
    let mut out = None;
    for line in contents.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("PCI_ID=") else {
            continue;
        };
        let rest = rest.trim();
        let fields: Vec<&str> = rest.split(':').collect();
        if fields.len() != 2 {
            continue;
        }
        if let (Some(vid), Some(did)) = (
            parse_pci_hex_field(fields[0]),
            parse_pci_hex_field(fields[1]),
        ) {
            out = Some((vid, did));
        }
    }
    out
}

/// Best-effort human-readable label from PCI vendor/device IDs (no PCI ID database).
#[cfg(any(test, target_os = "linux"))]
fn infer_gpu_model_from_ids(vendor_id: u32, device_id: u32) -> String {
    match vendor_id {
        0x10de => format!("NVIDIA GPU (0x{device_id:04X})"),
        0x1002 => format!("AMD GPU (0x{device_id:04X})"),
        0x8086 => format!("Intel GPU (0x{device_id:04X})"),
        _ => format!("Unknown GPU (0x{vendor_id:04X}:0x{device_id:04X})"),
    }
}

/// Query GPU devices (self-knowledge)
///
/// Vendor-agnostic, graceful degradation if no GPUs found.
pub(super) fn query_gpu_devices() -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    let mut device_id = 0;

    #[cfg(target_os = "linux")]
    {
        detect_nvidia_gpus(&mut devices, &mut device_id);
        detect_drm_gpus(&mut devices, &mut device_id);
    }

    #[cfg(target_os = "macos")]
    {
        detect_macos_gpus(&mut devices, &mut device_id);
    }

    if !devices.is_empty() {
        info!("🎮 Detected {} GPU(s) via self-knowledge", devices.len());
        for device in &devices {
            info!(
                "   - {}: {} ({} MB)",
                device.vendor,
                device.name,
                device.memory_bytes / (1024 * 1024)
            );
        }
    }

    devices
}

#[cfg(target_os = "linux")]
fn detect_nvidia_gpus(devices: &mut Vec<GpuDevice>, device_id: &mut usize) {
    if let Ok(entries) = std::fs::read_dir("/proc/driver/nvidia/gpus") {
        for entry in entries.flatten() {
            let gpu_path = entry.path();
            let pci_id = entry.file_name().to_string_lossy().to_string();

            let info_path = gpu_path.join("information");
            let mut name = format!("NVIDIA GPU {device_id}");
            let mut memory_bytes = 0u64;

            if let Ok(info) = std::fs::read_to_string(&info_path)
                && let Some(parsed) = parse_nvidia_information(&info)
            {
                name = parsed;
            }

            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args([
                    "--query-gpu=memory.total",
                    "--format=csv,noheader,nounits",
                    "-i",
                    &device_id.to_string(),
                ])
                .output()
                && output.status.success()
                && let Ok(mem_str) = String::from_utf8(output.stdout)
                && let Ok(mem_mb) = mem_str.trim().parse::<u64>()
            {
                memory_bytes = mem_mb * 1024 * 1024;
            }

            let render_node = find_render_node_for_pci(&pci_id);
            let driver = detect_nvidia_driver();

            devices.push(GpuDevice {
                device_id: *device_id,
                name,
                vendor: "nvidia".to_string(),
                memory_bytes,
                compute_capability: Some(pci_id),
                render_node,
                driver,
                arch: None,
            });
            *device_id += 1;
        }
    }
}

#[cfg(target_os = "linux")]
fn detect_drm_gpus(devices: &mut Vec<GpuDevice>, device_id: &mut usize) {
    if let Ok(entries) = std::fs::read_dir(toadstool_cylinder::linux_paths::sysfs_join(&["class", "drm"])) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("card") || name.contains("render") {
                continue;
            }

            let card_path = entry.path();
            let device_path = card_path.join("device");

            let vendor_path = device_path.join("vendor");
            if let Ok(vendor_id) = std::fs::read_to_string(&vendor_path) {
                let vendor_id = vendor_id.trim();

                if vendor_id == "0x10de" {
                    continue;
                }

                let vendor = match vendor_id {
                    "0x1002" => "amd",
                    "0x8086" => "intel",
                    _ => continue,
                };

                let mut gpu_name = format!("{} GPU {}", vendor.to_uppercase(), device_id);

                let uevent_path = device_path.join("uevent");
                if let Ok(uevent) = std::fs::read_to_string(&uevent_path)
                    && let Some((vid, did)) = parse_drm_uevent(&uevent)
                {
                    gpu_name = infer_gpu_model_from_ids(vid, did);
                }

                let mut memory_bytes = 0u64;
                let mem_path = device_path.join("mem_info_vram_total");
                if let Ok(mem_str) = std::fs::read_to_string(&mem_path)
                    && let Ok(mem) = mem_str.trim().parse::<u64>()
                {
                    memory_bytes = mem;
                }

                let render_node = find_render_node_sibling(&card_path);
                let driver = read_driver_name(&device_path);
                let arch = infer_gpu_arch(vendor, &device_path);

                devices.push(GpuDevice {
                    device_id: *device_id,
                    name: gpu_name,
                    vendor: vendor.to_string(),
                    memory_bytes,
                    compute_capability: None,
                    render_node,
                    driver,
                    arch,
                });
                *device_id += 1;
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn detect_macos_gpus(devices: &mut Vec<GpuDevice>, device_id: &mut usize) {
    if let Ok(output) = std::process::Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output()
    {
        if output.status.success() {
            if let Ok(json_str) = String::from_utf8(output.stdout) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(displays) =
                        json.get("SPDisplaysDataType").and_then(|d| d.as_array())
                    {
                        for display in displays {
                            let name = display
                                .get("sppci_model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown GPU")
                                .to_string();

                            let vendor = if name.contains("Apple")
                                || name.contains("M1")
                                || name.contains("M2")
                                || name.contains("M3")
                            {
                                "apple"
                            } else if name.contains("AMD") || name.contains("Radeon") {
                                "amd"
                            } else if name.contains("Intel") {
                                "intel"
                            } else if name.contains("NVIDIA") {
                                "nvidia"
                            } else {
                                "unknown"
                            };

                            let memory_bytes = display
                                .get("sppci_vram")
                                .and_then(|v| v.as_str())
                                .and_then(|s| {
                                    let parts: Vec<&str> = s.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        let num: u64 = parts[0].parse().ok()?;
                                        let unit = parts[1].to_uppercase();
                                        match unit.as_str() {
                                            "GB" => Some(num * 1024 * 1024 * 1024),
                                            "MB" => Some(num * 1024 * 1024),
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(0);

                            devices.push(GpuDevice {
                                device_id: *device_id,
                                name,
                                vendor: vendor.to_string(),
                                memory_bytes,
                                compute_capability: None,
                                render_node: None,
                                driver: Some("metal".to_string()),
                                arch: None,
                            });
                            *device_id += 1;
                        }
                    }
                }
            }
        }
    }
}

/// Find the render node (e.g. `/dev/dri/renderD128`) for a card directory.
#[cfg(target_os = "linux")]
fn find_render_node_sibling(card_path: &std::path::Path) -> Option<String> {
    let parent = card_path.parent()?;
    let card_device = card_path.join("device").canonicalize().ok()?;
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("renderD") {
                let render_device = entry.path().join("device").canonicalize().ok();
                if render_device.as_ref() == Some(&card_device) {
                    return Some(format!("/dev/dri/{name}"));
                }
            }
        }
    }
    None
}

/// Find the render node for an NVIDIA GPU given its PCI address.
#[cfg(target_os = "linux")]
fn find_render_node_for_pci(pci_id: &str) -> Option<String> {
    let drm_dir = toadstool_cylinder::linux_paths::sysfs_join(&["class", "drm"]);
    let drm_path = std::path::Path::new(&drm_dir);
    if let Ok(entries) = std::fs::read_dir(drm_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("renderD")
                && let Ok(target) = std::fs::read_link(entry.path().join("device"))
                && target.to_string_lossy().contains(pci_id)
            {
                return Some(format!("/dev/dri/{name}"));
            }
        }
    }
    None
}

/// Read the kernel driver name from the `driver` symlink in sysfs.
#[cfg(target_os = "linux")]
fn read_driver_name(device_path: &std::path::Path) -> Option<String> {
    let driver_link = device_path.join("driver");
    std::fs::read_link(driver_link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
}

/// Detect NVIDIA driver type from loaded kernel modules.
#[cfg(target_os = "linux")]
fn detect_nvidia_driver() -> Option<String> {
    if std::path::Path::new(&toadstool_cylinder::linux_paths::sysfs_module_path("nvidia")).exists() {
        Some("nvidia".to_string())
    } else if std::path::Path::new(&toadstool_cylinder::linux_paths::sysfs_module_path("nouveau")).exists() {
        Some("nouveau".to_string())
    } else {
        None
    }
}

/// Infer GPU micro-architecture from PCI revision and vendor.
#[cfg(target_os = "linux")]
fn infer_gpu_arch(vendor: &str, device_path: &std::path::Path) -> Option<String> {
    let revision_path = device_path.join("revision");
    let _revision = std::fs::read_to_string(revision_path).ok();
    match vendor {
        "amd" => {
            let uevent = std::fs::read_to_string(device_path.join("uevent")).ok()?;
            let (_, device_id) = parse_drm_uevent(&uevent)?;
            Some(amd_arch_from_device_id(device_id).to_string())
        }
        _ => None,
    }
}

/// Map AMD PCI device ID ranges to architecture names.
#[cfg(target_os = "linux")]
fn amd_arch_from_device_id(device_id: u32) -> &'static str {
    match device_id {
        0x73A0..=0x73FF => "rdna3",
        0x7300..=0x739F => "rdna2",
        0x6900..=0x69FF => "rdna1",
        0x6860..=0x68FF => "vega",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::{infer_gpu_model_from_ids, parse_drm_uevent, parse_nvidia_information};

    #[test]
    fn parse_nvidia_information_real_format() {
        let text = "Model: GeForce RTX 4090\nBus Type: PCIe\n";
        assert_eq!(
            parse_nvidia_information(text).as_deref(),
            Some("GeForce RTX 4090")
        );
    }

    #[test]
    fn parse_nvidia_information_missing_model_line() {
        let text = "Bus: PCI 0000:01:00.0\n";
        assert!(parse_nvidia_information(text).is_none());
    }

    #[test]
    fn parse_nvidia_information_empty() {
        assert!(parse_nvidia_information("").is_none());
    }

    #[test]
    fn parse_drm_uevent_valid_pci_id() {
        let text = "DRIVER=amdgpu\nPCI_ID=1002:73FF\n";
        assert_eq!(parse_drm_uevent(text), Some((0x1002, 0x73FF)));
    }

    #[test]
    fn parse_drm_uevent_valid_pci_id_with_0x_prefix() {
        let text = "PCI_ID=0x8086:0x9A49\n";
        assert_eq!(parse_drm_uevent(text), Some((0x8086, 0x9A49)));
    }

    #[test]
    fn parse_drm_uevent_missing_pci_id() {
        let text = "DRIVER=i915\nMODALIAS=pci:v00008086d00009A49...\n";
        assert!(parse_drm_uevent(text).is_none());
    }

    #[test]
    fn parse_drm_uevent_malformed() {
        assert!(parse_drm_uevent("PCI_ID=1002\n").is_none());
        assert!(parse_drm_uevent("PCI_ID=1002:ZZ:01\n").is_none());
        assert!(parse_drm_uevent("PCI_ID=nothex:nothex\n").is_none());
    }

    #[test]
    fn infer_gpu_model_from_ids_nvidia() {
        let s = infer_gpu_model_from_ids(0x10de, 0x2204);
        assert!(s.contains("NVIDIA"));
        assert!(s.contains("2204"));
    }

    #[test]
    fn infer_gpu_model_from_ids_amd() {
        let s = infer_gpu_model_from_ids(0x1002, 0x73FF);
        assert!(s.contains("AMD"));
    }

    #[test]
    fn infer_gpu_model_from_ids_intel() {
        let s = infer_gpu_model_from_ids(0x8086, 0x9A49);
        assert!(s.contains("Intel"));
    }
}
