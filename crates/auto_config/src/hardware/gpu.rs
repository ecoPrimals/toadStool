// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU detection and vendor support

use serde::{Deserialize, Serialize};
use toadstool_common::pci::vendors;
use toadstool_common::pci_discovery::{PciDevice, PciFilter, discover_pci_devices};
use tracing::debug;

use crate::ToadStoolResult;

use super::HardwareDetector;
use super::pci_names;

/// GPU information and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU model name.
    pub name: String,
    /// Vendor (NVIDIA, AMD, Intel).
    pub vendor: String,
    /// VRAM in GB.
    pub memory_gb: f64,
    /// Driver version string.
    pub driver_version: String,
    /// Compute capability (e.g. CUDA sm_xx).
    pub compute_capability: String,
    /// CUDA support.
    pub supports_cuda: bool,
}

/// Detect every GPU on the PCI bus.
///
/// # Evolved from vendor tools (Aug 16, 2026)
///
/// This used to run `nvidia-smi` and `rocm-smi` and parse their output, then
/// guess at anything they did not report. That was wrong in four ways, all of
/// which were live on biomeGate:
///
/// - **It missed most of the hardware.** `nvidia-smi` reports only devices
///   bound to the proprietary driver. Of four installed NVIDIA GPUs it saw
///   one, silently omitting an unbound Titan V and both `vfio-pci` Tesla K80
///   dies — precisely the sovereign configuration this project targets.
/// - **It invented an Intel GPU.** The Intel path asserted a 2 GB integrated
///   device whenever `/dev/dri` existed, which it does on any machine with
///   any DRM driver. This host has no Intel graphics and was reporting one.
/// - **It read capability off marketing strings.** Compute capability came
///   from substring-matching names like `"RTX 40"` and `"4090"`, so every GPU
///   here — Titan V, K80, RTX 5060 — fell through to `"Unknown"`.
/// - **It needed the tools installed** to detect hardware that the kernel had
///   already fully enumerated in sysfs.
///
/// Everything now comes from sysfs, which the kernel maintains for every
/// device regardless of bound driver, or none. Devices are selected by PCI
/// class, so an accelerator from an unrecognised vendor still enumerates.
pub async fn detect_gpus(_detector: &HardwareDetector) -> ToadStoolResult<Vec<GpuInfo>> {
    let gpus = detect_gpus_from_sysfs();
    debug!("Detected {} GPU(s) from sysfs", gpus.len());
    Ok(gpus)
}

/// PCI base class 0x03 (display) or 0x12 (processing accelerator).
///
/// 0x12 matters: datacentre parts with no display engine live there, and are
/// missed entirely by the `grep VGA` idiom.
fn is_gpu_class(class_code: u32) -> bool {
    matches!((class_code >> 16) as u8, 0x03 | 0x12)
}

/// Enumerate accelerators from sysfs and describe each one.
fn detect_gpus_from_sysfs() -> Vec<GpuInfo> {
    let filter = PciFilter::default().with_class(is_gpu_class);
    discover_pci_devices(&filter)
        .iter()
        .map(gpu_info_from_pci)
        .collect()
}

/// Build a [`GpuInfo`] from a discovered PCI device, measuring what sysfs
/// exposes and declining to guess at the rest.
fn gpu_info_from_pci(device: &PciDevice) -> GpuInfo {
    let vendor = vendor_label(device.vendor_id);
    GpuInfo {
        name: device_model_name(device),
        memory_gb: detect_vram_gb(device).unwrap_or(0.0),
        driver_version: driver_version(device).unwrap_or_else(|| "unknown".to_string()),
        compute_capability: "unknown".to_string(),
        supports_cuda: device.vendor_id == vendors::NVIDIA_VENDOR_ID,
        vendor,
    }
}

/// Best available model name, preferring the most specific source.
///
/// `pci.ids` is comprehensive but only as current as the installed package —
/// the 2025 copy on this host has no entry for the RTX 5060 (`10de:2d05`) and
/// falls back to the numeric ID. Where a kernel module publishes the model
/// itself, that is both exact and always current, so it wins.
fn device_model_name(device: &PciDevice) -> String {
    kernel_reported_model(device)
        .unwrap_or_else(|| pci_names::describe(device.vendor_id, device.device_id))
}

/// Model name as published by the bound kernel module.
///
/// The NVIDIA module exposes `/proc/driver/nvidia/gpus/<bdf>/information`,
/// whose `Model:` line carries the marketing name. This is procfs written by
/// the kernel, not a `nvidia-smi` invocation: no vendor userspace tooling need
/// be installed, and reading it cannot fail in the ways spawning a process can.
///
/// Returns `None` for any device whose driver publishes nothing, which is the
/// common case and not an error.
fn kernel_reported_model(device: &PciDevice) -> Option<String> {
    let path = format!("/proc/driver/nvidia/gpus/{}/information", device.bdf);
    let info = std::fs::read_to_string(path).ok()?;

    info.lines()
        .find_map(|line| line.strip_prefix("Model:"))
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(ToString::to_string)
}

/// Vendor label, using the same names the rest of the tree matches on.
///
/// Unrecognised vendors get their numeric ID rather than being dropped or
/// coerced into a known vendor — a GPU from a vendor that did not exist when
/// this was written is still a GPU.
fn vendor_label(vendor_id: u16) -> String {
    match vendor_id {
        vendors::NVIDIA_VENDOR_ID => "NVIDIA".to_string(),
        vendors::AMD_VENDOR_ID => "AMD".to_string(),
        vendors::INTEL_VENDOR_ID => "Intel".to_string(),
        other => pci_names::vendor_name(other)
            .map_or_else(|| format!("PCI vendor {other:04x}"), ToString::to_string),
    }
}

/// Driver version from `/sys/bus/pci/devices/<bdf>/driver/module/version`.
///
/// This is where the kernel publishes the version of whichever module is
/// bound, so it works for `nvidia`, `amdgpu`, `nouveau`, and `i915` alike —
/// the same string `nvidia-smi` reports, without `nvidia-smi`. Returns `None`
/// for an unbound device or a built-in module, both of which genuinely have
/// no version to report.
fn driver_version(device: &PciDevice) -> Option<String> {
    let version = std::fs::read_to_string(device.sysfs_path.join("driver/module/version")).ok()?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_string())
}

/// VRAM in GB, where the kernel actually exposes it.
///
/// # Why this is often `None`
///
/// Total VRAM is not in PCI config space, so it is only available when the
/// bound driver publishes it. `amdgpu` does, via `mem_info_vram_total`. The
/// proprietary NVIDIA driver does not, and an unbound or `vfio-pci` device
/// has no driver to ask.
///
/// The BAR1 aperture is deliberately **not** used as a substitute, tempting as
/// it looks. It measures how much VRAM is host-visible, not how much exists:
/// on this machine a 12 GB K80 die presents a 16 GiB BAR while an unbound
/// Titan V presents 256 MiB. Reporting either as capacity would be confidently
/// wrong, which is worse than reporting nothing.
///
/// Reading it from the device itself is possible but needs BAR0 access, which
/// belongs to cylinder's privileged path rather than hardware detection.
fn detect_vram_gb(device: &PciDevice) -> Option<f64> {
    let raw = std::fs::read_to_string(device.sysfs_path.join("mem_info_vram_total")).ok()?;
    let bytes: u64 = raw.trim().parse().ok()?;
    (bytes > 0).then(|| bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// Memory term used when VRAM cannot be measured.
///
/// Mid-scale: enough that a discrete GPU with unmeasurable VRAM is not ranked
/// beneath integrated graphics, low enough not to claim it is a large card.
const NEUTRAL_MEMORY_SCORE: f64 = 25.0;

/// VRAM assumed when ranking a device whose memory cannot be measured.
/// Corresponds to [`NEUTRAL_MEMORY_SCORE`] on the same scale.
const NEUTRAL_VRAM_GB: f64 = 12.0;

/// Calculate GPU performance score
#[must_use]
pub fn calculate_gpu_score(gpu_info: &[GpuInfo]) -> f64 {
    if gpu_info.is_empty() {
        return 20.0; // Integrated graphics assumption
    }

    // Rank by measured VRAM, but treat unmeasurable as the neutral midpoint so
    // a discrete GPU with no reporting driver can still be selected as best.
    let effective_vram = |g: &GpuInfo| {
        if g.memory_gb > 0.0 {
            g.memory_gb
        } else {
            NEUTRAL_VRAM_GB
        }
    };
    let Some(best_gpu) = gpu_info.iter().max_by(|a, b| {
        effective_vram(a)
            .partial_cmp(&effective_vram(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    }) else {
        return 20.0; // Fallback to integrated graphics score
    };

    // VRAM is only reported when the kernel publishes it, so 0.0 means
    // "not measurable here", not "no memory". Scoring it as zero would rank a
    // Titan V below an integrated GPU purely because no driver is bound to it.
    // Score the memory term as neutral when unknown, and let the vendor and
    // compute terms carry the estimate.
    let memory_score = if best_gpu.memory_gb > 0.0 {
        (best_gpu.memory_gb / 24.0 * 50.0).min(50.0)
    } else {
        NEUTRAL_MEMORY_SCORE
    };
    let vendor_score = match best_gpu.vendor.as_str() {
        "NVIDIA" => 40.0,
        "AMD" => 35.0,
        "Intel" => 20.0,
        _ => 15.0,
    };
    let compute_score = if best_gpu.supports_cuda { 10.0 } else { 5.0 };

    memory_score + vendor_score + compute_score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_info_serialization() {
        let gpu = GpuInfo {
            name: "NVIDIA GeForce RTX 4090".to_string(),
            vendor: "NVIDIA".to_string(),
            memory_gb: 24.0,
            driver_version: "535.0".to_string(),
            compute_capability: "8.9".to_string(),
            supports_cuda: true,
        };

        let json = serde_json::to_string(&gpu).unwrap();
        let deserialized: GpuInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, gpu.name);
        assert_eq!(deserialized.vendor, gpu.vendor);
        assert!((deserialized.memory_gb - gpu.memory_gb).abs() < f64::EPSILON);
    }

    /// Class selection must catch display controllers, 3D controllers, and
    /// the 0x12 processing-accelerator class that datacentre parts use.
    #[test]
    fn gpu_classes_include_headless_accelerators() {
        assert!(is_gpu_class(0x03_00_00), "VGA controller");
        assert!(is_gpu_class(0x03_02_00), "3D controller (Tesla)");
        assert!(is_gpu_class(0x12_00_00), "processing accelerator");
        assert!(!is_gpu_class(0x02_00_00), "ethernet");
        assert!(!is_gpu_class(0x06_04_00), "PCI bridge");
    }

    /// An unrecognised vendor must still be reported as itself rather than
    /// dropped or coerced into a known vendor.
    #[test]
    fn unknown_vendors_are_labelled_not_discarded() {
        assert_eq!(vendor_label(vendors::NVIDIA_VENDOR_ID), "NVIDIA");
        assert_eq!(vendor_label(vendors::AMD_VENDOR_ID), "AMD");
        assert_eq!(vendor_label(vendors::INTEL_VENDOR_ID), "Intel");

        let exotic = vendor_label(0xBEEF);
        assert!(!exotic.is_empty());
        assert_ne!(exotic, "NVIDIA");
    }

    /// Detection must not invent hardware. The previous implementation
    /// asserted a 2 GB Intel iGPU whenever /dev/dri existed; every GPU
    /// reported now corresponds to a device the kernel enumerated.
    #[test]
    fn detection_reports_only_real_devices() {
        for gpu in detect_gpus_from_sysfs() {
            assert!(!gpu.name.is_empty(), "every device must be nameable");
            assert!(!gpu.vendor.is_empty());
            assert!(
                gpu.memory_gb >= 0.0,
                "VRAM is measured or zero, never negative"
            );
        }
    }

    /// Must be total on hosts with no GPUs and no sysfs (CI, containers).
    #[test]
    fn detection_is_infallible() {
        let _ = detect_gpus_from_sysfs();
    }

    #[test]
    fn test_calculate_gpu_score_empty() {
        let score = calculate_gpu_score(&[]);
        assert!((score - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_gpu_score_nvidia() {
        let gpus = vec![GpuInfo {
            name: "NVIDIA GeForce RTX 4090".to_string(),
            vendor: "NVIDIA".to_string(),
            memory_gb: 24.0,
            driver_version: "535.0".to_string(),
            compute_capability: "8.9".to_string(),
            supports_cuda: true,
        }];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 90.0);
    }

    #[test]
    fn test_calculate_gpu_score_amd() {
        let gpus = vec![GpuInfo {
            name: "AMD Radeon RX 7900 XTX".to_string(),
            vendor: "AMD".to_string(),
            memory_gb: 24.0,
            driver_version: "Unknown".to_string(),
            compute_capability: "RDNA".to_string(),
            supports_cuda: false,
        }];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 80.0);
    }

    #[test]
    fn test_calculate_gpu_score_intel() {
        let gpus = vec![GpuInfo {
            name: "Intel Integrated Graphics".to_string(),
            vendor: "Intel".to_string(),
            memory_gb: 2.0,
            driver_version: "Unknown".to_string(),
            compute_capability: "Gen9+".to_string(),
            supports_cuda: false,
        }];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 20.0);
        assert!(score < 50.0);
    }

    #[test]
    fn test_calculate_gpu_score_unknown_vendor() {
        let gpus = vec![GpuInfo {
            name: "Unknown GPU".to_string(),
            vendor: "Other".to_string(),
            memory_gb: 8.0,
            driver_version: "Unknown".to_string(),
            compute_capability: "Unknown".to_string(),
            supports_cuda: false,
        }];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 15.0);
    }

    #[test]
    fn test_calculate_gpu_score_picks_best_by_memory() {
        let gpus = vec![
            GpuInfo {
                name: "NVIDIA GTX 1060".to_string(),
                vendor: "NVIDIA".to_string(),
                memory_gb: 6.0,
                driver_version: "535.0".to_string(),
                compute_capability: "6.1".to_string(),
                supports_cuda: true,
            },
            GpuInfo {
                name: "NVIDIA GeForce RTX 4090".to_string(),
                vendor: "NVIDIA".to_string(),
                memory_gb: 24.0,
                driver_version: "535.0".to_string(),
                compute_capability: "8.9".to_string(),
                supports_cuda: true,
            },
        ];
        let score = calculate_gpu_score(&gpus);
        assert!(score >= 90.0);
    }
}

#[cfg(test)]
mod live_detection {
    use super::*;

    /// What native detection reports on this machine. Not an assertion — CI
    /// has no GPUs — but the comparison against the vendor tools it replaces.
    #[test]
    fn show_detected_gpus() {
        for g in detect_gpus_from_sysfs() {
            eprintln!(
                "  {:<10} {:<38} vram={:>5.1}GB driver={:<8} cuda={}",
                g.vendor, g.name, g.memory_gb, g.driver_version, g.supports_cuda
            );
        }
    }
}
