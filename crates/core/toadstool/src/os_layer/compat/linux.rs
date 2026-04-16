// SPDX-License-Identifier: AGPL-3.0-or-later
//! Linux compatibility layer.
//!
//! Uses `uname`-based detection during initialization to verify the platform.
//! On non-Linux platforms, returns `PlatformNotAvailable`.

use serde::{Deserialize, Serialize};
use std::future::Future;

use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult};
#[cfg(target_os = "linux")]
use toadstool_common::constants::platform_paths::procfs;
use toadstool_common::error::SystemError;

use super::CompatibilityLayer;

/// Normalize `uname -a` output or `/proc/version` text to a short kernel release string.
fn parse_kernel_version(uname_or_proc_version: &str) -> String {
    let s = uname_or_proc_version.trim();
    if s.is_empty() {
        return "unknown".to_string();
    }
    if let Some(idx) = s.find("Linux version ") {
        let rest = s[idx + "Linux version ".len()..].trim_start();
        if let Some(ver) = rest.split_whitespace().next() {
            if !ver.is_empty() {
                return ver.to_string();
            }
        }
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() >= 3 && parts[0] == "Linux" {
        return parts[2].to_string();
    }
    "unknown".to_string()
}

/// Linux compatibility layer.
///
/// On Linux, uses `uname`-based detection during initialization to verify
/// the platform. On other platforms, returns `PlatformNotAvailable`.
#[derive(Debug)]
pub struct LinuxCompatibilityLayer {
    config: LinuxCompatConfig,
    /// Detected kernel release string (normalized from `uname -a` or `/proc/version` on initialize)
    uname_info: Option<String>,
}

/// Configuration for Linux compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxCompatConfig {
    /// Enable namespace isolation
    pub namespace_isolation: bool,
    /// Enable cgroup resource control
    pub cgroup_control: bool,
    /// Enable seccomp filtering
    pub seccomp_filtering: bool,
    /// Enable capabilities management
    pub capabilities_management: bool,
}

impl Default for LinuxCompatConfig {
    fn default() -> Self {
        Self {
            namespace_isolation: true,
            cgroup_control: true,
            seccomp_filtering: true,
            capabilities_management: true,
        }
    }
}

impl Default for LinuxCompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxCompatibilityLayer {
    /// Creates a new Linux compatibility layer with default config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: LinuxCompatConfig::default(),
            uname_info: None,
        }
    }

    /// Returns the Linux compatibility config.
    #[must_use]
    pub const fn get_config(&self) -> &LinuxCompatConfig {
        &self.config
    }

    /// Get detected kernel release (available after initialize on Linux).
    #[must_use]
    pub fn uname_info(&self) -> Option<&str> {
        self.uname_info.as_deref()
    }

    /// Run uname-based platform detection (Linux only).
    fn detect_platform() -> String {
        #[cfg(target_os = "linux")]
        {
            let raw = std::process::Command::new("uname")
                .args(["-a"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    std::fs::read_to_string(procfs::VERSION)
                        .ok()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or_default();
            parse_kernel_version(&raw)
        }
        #[cfg(not(target_os = "linux"))]
        {
            "not_linux".to_string()
        }
    }
}

impl CompatibilityLayer for LinuxCompatibilityLayer {
    fn name(&self) -> &'static str {
        "linux"
    }

    fn features(&self) -> Vec<String> {
        vec!["namespaces".to_string(), "cgroups".to_string()]
    }

    fn can_handle(&self, _request: &ExecutionRequest) -> bool {
        cfg!(target_os = "linux")
    }

    fn execute_with_compatibility(
        &self,
        _request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async move {
            if !cfg!(target_os = "linux") {
                return Err(SystemError::NotSupported {
                    feature: "linux_compat_layer".into(),
                    reason: "LinuxCompatibilityLayer requires target_os = linux".into(),
                }
                .into());
            }
            Ok(ExecutionResponse::default())
        }
    }

    fn initialize(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async {
            let uname_info = Self::detect_platform();
            if !cfg!(target_os = "linux") {
                return Err(SystemError::NotSupported {
                    feature: "linux_compat_layer".into(),
                    reason: "LinuxCompatibilityLayer requires target_os = linux. Current platform is not Linux.".into(),
                }
                .into());
            }
            if uname_info == "not_linux" {
                return Err(SystemError::NotSupported {
                    feature: "linux_compat_layer".into(),
                    reason: "Platform detection failed: not running on Linux.".into(),
                }
                .into());
            }
            self.uname_info = Some(uname_info);
            Ok(())
        }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_kernel_version;

    #[test]
    fn parse_kernel_version_from_uname_a() {
        let uname = "Linux workstation 6.14.0-23-generic #23~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Mon Sep 22 10:57:38 UTC 2 x86_64 x86_64 x86_64 GNU/Linux";
        assert_eq!(parse_kernel_version(uname), "6.14.0-23-generic");
    }

    #[test]
    fn parse_kernel_version_from_proc_version() {
        let proc = "Linux version 6.14.0-23-generic (buildd@lcy02-amd64-029) (x86_64-linux-gnu-gcc-13 (Ubuntu 13.3.0-6ubuntu2~24.04) 13.3.0, GNU ld (GNU Binutils for Ubuntu) 2.42) #23~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Mon Sep 22 10:57:38 UTC 2025\n";
        assert_eq!(parse_kernel_version(proc), "6.14.0-23-generic");
    }

    #[test]
    fn parse_kernel_version_garbage_and_empty() {
        assert_eq!(parse_kernel_version(""), "unknown");
        assert_eq!(parse_kernel_version("   \n"), "unknown");
        assert_eq!(
            parse_kernel_version("this is not a kernel string"),
            "unknown"
        );
    }
}
