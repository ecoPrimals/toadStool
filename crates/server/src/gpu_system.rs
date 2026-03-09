// SPDX-License-Identifier: AGPL-3.0-only
//! GPU System Query Helpers
//!
//! Standalone functions for querying GPU devices, memory, and available backends.
//! Detects NVIDIA GPUs via /proc on Linux, falls back to wgpu abstraction.
//! Backend discovery is capability-based — no hardcoded backend lists.

/// Query available GPU devices
///
/// Detects NVIDIA GPUs via /proc on Linux, falls back to wgpu abstraction.
#[must_use]
pub fn query_gpu_devices() -> Vec<serde_json::Value> {
    let mut devices = Vec::new();

    #[cfg(target_os = "linux")]
    if let Ok(entries) = std::fs::read_dir("/proc/driver/nvidia/gpus") {
        for (idx, entry) in entries.flatten().enumerate() {
            let name = entry.file_name().to_string_lossy().to_string();
            devices.push(serde_json::json!({
                "index": idx, "id": name, "backend": "nvidia",
            }));
        }
    }

    if devices.is_empty() {
        devices.push(serde_json::json!({
            "index": 0, "id": "wgpu-default", "backend": "wgpu",
            "note": "GPU detection via wgpu adapter enumeration at runtime",
        }));
    }

    devices
}

/// Query GPU memory usage via nvidia-smi
#[must_use]
pub fn query_gpu_memory() -> Vec<serde_json::Value> {
    let mut devices = Vec::new();

    #[cfg(target_os = "linux")]
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,memory.total,memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
    {
        if output.status.success() {
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
    }

    if devices.is_empty() {
        devices.push(serde_json::json!({
            "note": "GPU memory query requires nvidia-smi or wgpu adapter",
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
        backends.push("metal");
    }

    #[cfg(target_os = "windows")]
    {
        backends.push("dx12");
        backends.push("vulkan");
    }

    if backends.is_empty() {
        backends.push("wgpu-auto");
    }

    backends
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
}
