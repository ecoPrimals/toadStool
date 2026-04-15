// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU and specialized architecture detection
//!
//! CUDA and ROCm capability probing (OpenCL detection removed — S198; use barraCuda/coralReef via IPC).

use super::helpers::check_command_exists;

/// Get CUDA version
pub fn get_cuda_version() -> String {
    std::process::Command::new("nvcc")
        .arg("--version")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
        )
}

/// Get CUDA compute capability
pub fn get_cuda_compute_capability() -> String {
    if check_command_exists("nvidia-smi")
        && let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
            .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        let cap = s.trim().split('\n').next().unwrap_or("").trim();
        if !cap.is_empty() {
            return cap.to_string();
        }
    }
    "unknown".to_string()
}

/// Get GPU memory in gigabytes
pub fn get_gpu_memory_gb() -> u32 {
    if check_command_exists("nvidia-smi")
        && let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
            .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        if let Some(mb_str) = s
            .trim()
            .split('\n')
            .next()
            .and_then(|l| l.split(',').next())
            && let Ok(mb) = mb_str.trim().parse::<u32>()
        {
            return mb.div_ceil(1024);
        }
    }
    0
}

/// Get ROCm version
pub fn get_rocm_version() -> String {
    if let Ok(ver) = std::fs::read_to_string("/opt/rocm/.info/version") {
        let ver = ver.trim();
        if !ver.is_empty() {
            return ver.to_string();
        }
    }
    if check_command_exists("rocm-smi")
        && let Ok(output) = std::process::Command::new("rocm-smi")
            .arg("--showversion")
            .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        let first_line = s.lines().next().unwrap_or("").trim();
        if !first_line.is_empty() {
            return first_line.to_string();
        }
    }
    "unknown".to_string()
}

/// Get ROCm GFX version
pub fn get_rocm_gfx_version() -> String {
    if check_command_exists("rocm-smi")
        && let Ok(output) = std::process::Command::new("rocm-smi")
            .arg("--showproductname")
            .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        for line in s.lines() {
            let line = line.trim();
            if line.contains("gfx")
                && let Some(gfx) = line.split_whitespace().find(|w| w.starts_with("gfx"))
            {
                return gfx.to_string();
            }
        }
    }
    "unknown".to_string()
}

/// Check for OpenCL support (**stub:** always `false` — not runtime probing; OpenCL is not an in-tree backend; use barraCuda/coralReef via IPC).
///
/// Kept for API compatibility; substrate detection no longer consults OpenCL (S198).
#[deprecated(
    since = "0.1.0",
    note = "OpenCL removed S198; use barraCuda/coralReef via IPC"
)]
#[allow(
    dead_code,
    reason = "Legacy OpenCL API stubs retained for compatibility (S198)"
)]
pub fn check_opencl_support() -> bool {
    false
}

/// Get OpenCL version (**stub:** fixed string — not supported in-tree; S198).
#[deprecated(
    since = "0.1.0",
    note = "OpenCL removed S198; use barraCuda/coralReef via IPC"
)]
#[allow(
    dead_code,
    reason = "Legacy OpenCL API stubs retained for compatibility (S198)"
)]
pub fn get_opencl_version() -> String {
    "not supported (S198: use barraCuda/coralReef via IPC)".to_string()
}

/// Get OpenCL device type (**stub:** always `"none"` — S198).
#[deprecated(
    since = "0.1.0",
    note = "OpenCL removed S198; use barraCuda/coralReef via IPC"
)]
#[allow(
    dead_code,
    reason = "Legacy OpenCL API stubs retained for compatibility (S198)"
)]
pub fn get_opencl_device_type() -> String {
    "none".to_string()
}

/// Get OpenCL compute units (**stub:** always `0` — S198).
#[deprecated(
    since = "0.1.0",
    note = "OpenCL removed S198; use barraCuda/coralReef via IPC"
)]
#[allow(
    dead_code,
    reason = "Legacy OpenCL API stubs retained for compatibility (S198)"
)]
pub const fn get_opencl_compute_units() -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cuda_version() {
        let version = get_cuda_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_get_cuda_compute_capability() {
        let cap = get_cuda_compute_capability();
        assert!(!cap.is_empty());
    }

    #[test]
    fn test_get_gpu_memory_gb() {
        let gb = get_gpu_memory_gb();
        let _ = gb;
    }

    #[test]
    fn test_get_rocm_version() {
        let version = get_rocm_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_get_rocm_gfx_version() {
        let gfx = get_rocm_gfx_version();
        assert!(!gfx.is_empty());
    }
}
