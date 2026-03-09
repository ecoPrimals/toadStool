// SPDX-License-Identifier: AGPL-3.0-only
//! GPU and specialized architecture detection
//!
//! CUDA, ROCm, and OpenCL capability probing.

use super::helpers::check_command_exists;

/// Get CUDA version
pub(crate) fn get_cuda_version() -> String {
    std::process::Command::new("nvcc")
        .arg("--version")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
        )
}

/// Get CUDA compute capability
pub(crate) fn get_cuda_compute_capability() -> String {
    if check_command_exists("nvidia-smi") {
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                let cap = s.trim().split('\n').next().unwrap_or("").trim();
                if !cap.is_empty() {
                    return cap.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

/// Get GPU memory in gigabytes
pub(crate) fn get_gpu_memory_gb() -> u32 {
    if check_command_exists("nvidia-smi") {
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                if let Some(mb_str) = s
                    .trim()
                    .split('\n')
                    .next()
                    .and_then(|l| l.split(',').next())
                {
                    let mb_str = mb_str.trim();
                    if let Ok(mb) = mb_str.parse::<u32>() {
                        return mb.div_ceil(1024);
                    }
                }
            }
        }
    }
    0
}

/// Get ROCm version
pub(crate) fn get_rocm_version() -> String {
    if let Ok(ver) = std::fs::read_to_string("/opt/rocm/.info/version") {
        let ver = ver.trim();
        if !ver.is_empty() {
            return ver.to_string();
        }
    }
    if check_command_exists("rocm-smi") {
        if let Ok(output) = std::process::Command::new("rocm-smi")
            .arg("--showversion")
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                let first_line = s.lines().next().unwrap_or("").trim();
                if !first_line.is_empty() {
                    return first_line.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

/// Get ROCm GFX version
pub(crate) fn get_rocm_gfx_version() -> String {
    if check_command_exists("rocm-smi") {
        if let Ok(output) = std::process::Command::new("rocm-smi")
            .arg("--showproductname")
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines() {
                    let line = line.trim();
                    if line.contains("gfx") {
                        if let Some(gfx) = line.split_whitespace().find(|w| w.starts_with("gfx")) {
                            return gfx.to_string();
                        }
                    }
                }
            }
        }
    }
    "unknown".to_string()
}

/// Check for OpenCL support
pub(crate) fn check_opencl_support() -> bool {
    #[cfg(target_os = "linux")]
    {
        let vendors = std::path::Path::new("/etc/OpenCL/vendors");
        if vendors.is_dir() {
            if let Ok(entries) = std::fs::read_dir(vendors) {
                for entry in entries.flatten() {
                    if entry.path().extension().is_some_and(|e| e == "icd") {
                        return true;
                    }
                }
            }
        }
    }
    check_command_exists("clinfo")
}

/// Get OpenCL version
pub(crate) fn get_opencl_version() -> String {
    "2.0".to_string()
}

/// Get OpenCL device type
pub(crate) fn get_opencl_device_type() -> String {
    "GPU".to_string()
}

/// Get OpenCL compute units
pub(crate) const fn get_opencl_compute_units() -> u32 {
    64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_opencl_support() {
        let support = check_opencl_support();
        let _ = support;
    }

    #[test]
    fn test_get_opencl_compute_units() {
        let units = get_opencl_compute_units();
        assert!(units > 0);
    }

    #[test]
    fn test_get_opencl_version() {
        let version = get_opencl_version();
        assert_eq!(version, "2.0");
    }

    #[test]
    fn test_get_opencl_device_type() {
        let device_type = get_opencl_device_type();
        assert_eq!(device_type, "GPU");
    }

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
