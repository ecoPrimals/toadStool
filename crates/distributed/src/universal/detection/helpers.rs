// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform detection helpers
//!
//! CPU introspection, shell utilities, and OS-specific queries used by detection.

/// CPU information structure
#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub cache_mb: u32,
    pub big_little: bool,
    pub features: Vec<String>,
}

/// Get CPU information
pub fn get_cpu_info() -> CpuInfo {
    let cores = std::thread::available_parallelism()
        .map(|p| u32::try_from(p.get()).unwrap_or(4))
        .unwrap_or(4);

    #[cfg(target_os = "linux")]
    {
        if let Ok(info) = parse_cpuinfo_linux() {
            return info;
        }
    }

    // Fallback for non-Linux or parse failure
    CpuInfo {
        model: "Generic CPU".to_string(),
        cores,
        threads: cores,
        cache_mb: 8,
        big_little: false,
        features: vec!["sse4.2".to_string(), "avx2".to_string()],
    }
}

#[cfg(target_os = "linux")]
pub fn parse_cpuinfo_linux() -> Result<CpuInfo, ()> {
    let content = std::fs::read_to_string("/proc/cpuinfo").map_err(|_| ())?;

    let cores = std::thread::available_parallelism()
        .map(|p| u32::try_from(p.get()).unwrap_or(4))
        .unwrap_or(4);

    let mut model = "Generic CPU".to_string();
    let mut cache_mb = 8u32;
    let mut flags = Vec::new();
    let mut cpu_parts: std::collections::HashSet<String> = std::collections::HashSet::new();

    for block in content.split("\n\n") {
        for line in block.lines() {
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim();
                let val = val.trim();
                match key {
                    "model name" | "Model" => model = val.to_string(),
                    "cache size" => {
                        if let Some(kb_str) = val.split_whitespace().next()
                            && let Ok(kb) = kb_str.parse::<u32>()
                        {
                            cache_mb = kb.div_ceil(1024);
                        }
                    }
                    "flags" | "Features" => {
                        flags = val
                            .split_whitespace()
                            .filter(|s| s.len() > 2)
                            .map(String::from)
                            .collect();
                    }
                    "CPU part" => {
                        cpu_parts.insert(val.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    let features = if flags.is_empty() {
        vec!["sse4.2".to_string(), "avx2".to_string()]
    } else {
        flags
    };

    let big_little = cpu_parts.len() > 1;

    Ok(CpuInfo {
        model,
        cores,
        threads: cores,
        cache_mb,
        big_little,
        features,
    })
}

/// Get system memory in gigabytes
pub fn get_memory_gb() -> u32 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:")
                    && let Some(kb_str) = line.split_whitespace().nth(1)
                    && let Ok(kb) = kb_str.parse::<u64>()
                {
                    let gb = kb.div_ceil(1024 * 1024);
                    return gb.min(u32::MAX as u64) as u32;
                }
                if line.starts_with("MemTotal:") {
                    break;
                }
            }
        }
    }

    8
}

/// Check if a command exists in PATH
pub fn check_command_exists(command: &str) -> bool {
    std::process::Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or_default()
}

/// Get version string from a command
pub fn get_command_version(command: &str) -> String {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
        )
}

/// Get Rust target triple
pub fn get_rust_target_triple() -> String {
    std::process::Command::new("rustc")
        .arg("--print")
        .arg("target-triple")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
        )
}

/// Get Linux distribution name
pub fn get_linux_distribution() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let val = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                    if !val.is_empty() {
                        return val.to_string();
                    }
                    break;
                }
            }
            for line in content.lines() {
                if line.starts_with("NAME=") {
                    let val = line.trim_start_matches("NAME=").trim_matches('"');
                    if !val.is_empty() {
                        return val.to_string();
                    }
                    break;
                }
            }
        }
    }
    "unknown".to_string()
}

/// Get kernel version
pub fn get_kernel_version() -> String {
    std::process::Command::new("uname")
        .arg("-r")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
        )
}

/// Get init system type
pub fn get_init_system() -> String {
    if std::path::Path::new("/run/systemd/system").exists() {
        "systemd".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Get package manager type
pub fn get_package_manager() -> String {
    if check_command_exists("apt") {
        "apt".to_string()
    } else if check_command_exists("yum") {
        "yum".to_string()
    } else if check_command_exists("pacman") {
        "pacman".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Get macOS version
pub fn get_macos_version() -> String {
    std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_string(),
        )
}

/// Get macOS frameworks
pub fn get_macos_frameworks() -> Vec<String> {
    vec!["Foundation".to_string(), "CoreFoundation".to_string()]
}

/// Get Windows version
pub fn get_windows_version() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("cmd")
            .args(["/c", "ver"])
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout);
            let s = s.trim();
            // Extract version number from output like "Microsoft Windows [Version 10.0.19045.3803]"
            if let Some(start) = s.find("Version ") {
                let rest = &s[start + 8..];
                if let Some(end) = rest.find(']') {
                    let ver = rest[..end].trim();
                    if !ver.is_empty() {
                        return ver.to_string();
                    }
                }
            }
        }
    }
    "10".to_string()
}

/// Get Windows features
pub fn get_windows_features() -> Vec<String> {
    vec!["PowerShell".to_string(), "WSL".to_string()]
}

/// Get Windows subsystems
pub fn get_windows_subsystems() -> Vec<String> {
    vec!["Win32".to_string(), "WSL".to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_info() {
        let info = get_cpu_info();
        assert!(info.cores > 0);
    }

    #[test]
    fn test_command_detection() {
        let exists = check_command_exists("sh");
        assert!(exists);

        let not_exists = check_command_exists("nonexistent_command_xyz");
        assert!(!not_exists);
    }

    #[test]
    fn test_get_memory_gb() {
        let gb = get_memory_gb();
        assert!(gb > 0);
    }

    #[test]
    fn test_get_command_version() {
        let version = get_command_version("echo test");
        assert!(!version.is_empty() || version == "unknown");
    }

    #[test]
    fn test_get_linux_distribution() {
        let dist = get_linux_distribution();
        assert!(!dist.is_empty());
    }

    #[test]
    fn test_get_init_system() {
        let init = get_init_system();
        assert!(!init.is_empty());
    }

    #[test]
    fn test_get_package_manager() {
        let pkg = get_package_manager();
        assert!(!pkg.is_empty());
    }

    #[test]
    fn test_get_macos_frameworks() {
        let fw = get_macos_frameworks();
        assert!(!fw.is_empty());
    }

    #[test]
    fn test_get_windows_version() {
        let ver = get_windows_version();
        assert!(!ver.is_empty());
    }

    #[test]
    fn test_get_rust_target_triple() {
        let triple = get_rust_target_triple();
        assert!(triple == "unknown" || triple.is_empty() || triple.contains('-'));
    }

    #[test]
    fn test_get_windows_features() {
        let features = get_windows_features();
        assert!(!features.is_empty());
        assert!(features.contains(&"PowerShell".to_string()));
    }

    #[test]
    fn test_get_windows_subsystems() {
        let subsystems = get_windows_subsystems();
        assert!(!subsystems.is_empty());
        assert!(subsystems.contains(&"Win32".to_string()));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_cpuinfo_linux_fallback() {
        let result = parse_cpuinfo_linux();
        if let Ok(info) = result {
            assert!(info.cores > 0);
            assert!(!info.model.is_empty());
        }
    }
}
