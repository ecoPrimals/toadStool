// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// Result of probing nvidia's live FECS/GPCCS state for runtime services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeServicesProbe {
    /// PCI BDF probed.
    pub bdf: String,
    /// Currently bound driver.
    pub driver: String,
    /// Whether nvidia module is loaded.
    pub nvidia_loaded: bool,
    /// FECS firmware state (from /proc/driver/nvidia/ or BAR0 probe).
    pub fecs_state: String,
    /// TPC station liveness (from nvidia's perspective).
    pub tpc_alive: bool,
    /// Number of nvidia GPU channels established (from /proc/driver/nvidia/gpus/).
    pub nvidia_channels: u32,
}

/// Probe nvidia's live state for runtime services dispatch.
///
/// When nvidia is loaded as a runtime service, toadStool needs to discover
/// what nvidia has established: FECS context, TPC stations, channel state.
pub fn probe_runtime_services(bdf: &str) -> RuntimeServicesProbe {
    let driver_path = crate::linux_paths::sysfs_pci_device_file(bdf, "driver");
    let driver = std::fs::read_link(&driver_path)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unbound".to_owned());

    let nvidia_loaded = std::path::Path::new("/proc/driver/nvidia/version").exists();

    let gpu_dir = format!("/proc/driver/nvidia/gpus/{bdf}");
    let nvidia_channels = if std::path::Path::new(&gpu_dir).is_dir() {
        std::fs::read_to_string(format!("{gpu_dir}/information"))
            .ok()
            .and_then(|info| {
                for line in info.lines() {
                    if line.contains("GPU UUID") || line.contains("Bus Location") {
                        return Some(1);
                    }
                }
                None
            })
            .unwrap_or(0)
    } else {
        0
    };

    let fecs_state = if nvidia_loaded && driver.contains("nvidia") {
        "running (nvidia owns FECS context)".to_owned()
    } else if driver == "vfio-pci" {
        "unknown (vfio-pci bound, no FECS probe without BAR0)".to_owned()
    } else {
        format!("unknown (driver={driver})")
    };

    let tpc_alive = nvidia_loaded && driver.contains("nvidia");

    RuntimeServicesProbe {
        bdf: bdf.to_owned(),
        driver,
        nvidia_loaded,
        fecs_state,
        tpc_alive,
        nvidia_channels,
    }
}
