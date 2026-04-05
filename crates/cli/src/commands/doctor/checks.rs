// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use toadstool_common::constants::ecosystem::well_known;
use toadstool_common::constants::primal_identity::PRIMAL_NAME;

use super::types::{ConfigReport, EcosystemReport, HardwareReport, PrimalStatus};

pub(crate) async fn check_hardware_health() -> HardwareReport {
    let mut issues = vec![];

    let cpu_cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);

    let mut cpu_features = vec![];
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            cpu_features.push("AVX2".to_string());
        }
        if std::arch::is_x86_feature_detected!("avx512f") {
            cpu_features.push("AVX-512".to_string());
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        cpu_features.push("NEON".to_string());
    }

    let gpu_detected = check_gpu_available().await;
    let gpu_info = if gpu_detected {
        Some("GPU adapter available via wgpu".to_string())
    } else {
        issues.push("No GPU detected - compute will use CPU fallback".to_string());
        None
    };

    let npu_detected = Path::new("/dev/akida0").exists() || {
        let pci_devices = Path::new("/sys/bus/pci/devices");
        let mut found = false;
        if let Ok(mut entries) = tokio::fs::read_dir(pci_devices).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let vendor_path = entry.path().join("vendor");
                if let Ok(v) = tokio::fs::read_to_string(&vendor_path).await {
                    if v.trim() == "0x1e7c" {
                        found = true;
                        break;
                    }
                }
            }
        }
        found
    };

    let npu_info = if npu_detected {
        Some("Akida NPU detected".to_string())
    } else {
        None
    };

    let memory_total_mb = toadstool_sysmon::memory_info()
        .map(|m| m.total / (1024 * 1024))
        .unwrap_or(0);

    HardwareReport {
        cpu_cores,
        cpu_features,
        gpu_detected,
        gpu_info,
        npu_detected,
        npu_info,
        memory_total_mb,
        issues,
    }
}

#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)]
async fn check_gpu_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new("/usr/share/vulkan/icd.d").exists() || std::env::var("VK_ICD_FILENAMES").is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

#[allow(deprecated)]
pub(crate) async fn check_ecosystem_health() -> EcosystemReport {
    let mut issues = vec![];

    let biomeos_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let biomeos_dir_exists = biomeos_dir.exists();

    if !biomeos_dir_exists {
        issues.push(format!(
            "biomeOS directory does not exist: {}",
            biomeos_dir.display()
        ));
    }

    let mut sockets_found = vec![];
    let mut discovered_primal_names = HashSet::new();

    if biomeos_dir_exists {
        if let Ok(mut entries) = tokio::fs::read_dir(&biomeos_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().map(|e| e == "sock").unwrap_or(false)
                    || path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.ends_with(".sock") || !n.contains('.'))
                        .unwrap_or(false)
                {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        sockets_found.push(name.to_string());
                        let primal_name = name.strip_suffix(".sock").unwrap_or(name).to_string();
                        if !primal_name.is_empty() {
                            discovered_primal_names.insert(primal_name);
                        }
                    }
                }
            }
        }
    }

    let well_known_primals = [
        PRIMAL_NAME,
        well_known::SONGBIRD,
        well_known::BEARDOG,
        well_known::NESTGATE,
    ];
    for name in &well_known_primals {
        discovered_primal_names.insert((*name).to_string());
    }

    let mut primals_reachable = vec![];
    for primal in discovered_primal_names {
        let socket_path = biomeos_dir.join(format!("{primal}.sock"));
        let socket_exists = socket_path.exists();
        let reachable = if socket_exists {
            tokio::net::UnixStream::connect(&socket_path).await.is_ok()
        } else {
            false
        };

        primals_reachable.push(PrimalStatus {
            name: primal,
            socket_exists,
            reachable,
        });
    }

    primals_reachable.sort_by(|a, b| a.name.cmp(&b.name));

    EcosystemReport {
        biomeos_dir_exists,
        biomeos_dir: biomeos_dir.display().to_string(),
        sockets_found,
        primals_reachable,
        issues,
    }
}

pub(crate) async fn check_config_health() -> ConfigReport {
    let issues = vec![];

    let home = std::env::var("HOME").ok();
    let config_paths: Vec<PathBuf> = vec![
        PathBuf::from("toadstool.toml"),
        PathBuf::from(".toadstool/config.toml"),
    ];

    let mut all_paths = config_paths;
    if let Some(ref home_dir) = home {
        all_paths.push(PathBuf::from(home_dir).join(".config/toadstool/config.toml"));
    }

    let mut config_file_exists = false;
    let mut config_file_path = None;

    for path in all_paths {
        if path.exists() {
            config_file_exists = true;
            config_file_path = Some(path.display().to_string());
            break;
        }
    }

    let env_vars = [
        "TOADSTOOL_BIND_HOST",
        "TOADSTOOL_API_PORT",
        "TOADSTOOL_COORDINATION_ENDPOINT",
        "TOADSTOOL_SECURITY_ENDPOINT",
        "TOADSTOOL_STORAGE_ENDPOINT",
        "XDG_RUNTIME_DIR",
    ];

    let env_vars_set: Vec<String> = env_vars
        .iter()
        .filter(|v| std::env::var(v).is_ok())
        .map(|s| (*s).to_string())
        .collect();

    ConfigReport {
        config_file_exists,
        config_file_path,
        env_vars_set,
        issues,
    }
}
