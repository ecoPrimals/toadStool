// SPDX-License-Identifier: AGPL-3.0-or-later
//! Linux compatibility layer.
//!
//! Uses `uname`-based detection during initialization to verify the platform.
//! On non-Linux platforms, returns `PlatformNotAvailable`.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult};
use toadstool_common::error::SystemError;

use super::CompatibilityLayer;

/// Linux compatibility layer.
///
/// On Linux, uses `uname`-based detection during initialization to verify
/// the platform. On other platforms, returns `PlatformNotAvailable`.
#[derive(Debug)]
pub struct LinuxCompatibilityLayer {
    config: LinuxCompatConfig,
    /// Detected uname output (set during initialize on Linux)
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: LinuxCompatConfig::default(),
            uname_info: None,
        }
    }

    #[must_use]
    pub const fn get_config(&self) -> &LinuxCompatConfig {
        &self.config
    }

    /// Get detected uname info (available after initialize on Linux).
    #[must_use]
    pub fn uname_info(&self) -> Option<&str> {
        self.uname_info.as_deref()
    }

    /// Run uname-based platform detection (Linux only).
    fn detect_platform() -> String {
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("uname")
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
                .unwrap_or_else(|| {
                    std::fs::read_to_string("/proc/version")
                        .ok()
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                })
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
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            if !cfg!(target_os = "linux") {
                return Err(SystemError::NotSupported {
                    feature: "linux_compat_layer".into(),
                    reason: "LinuxCompatibilityLayer requires target_os = linux".into(),
                }
                .into());
            }
            Ok(ExecutionResponse::default())
        })
    }

    fn initialize(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let uname_info = Self::detect_platform();
        if !cfg!(target_os = "linux") {
            return Box::pin(async move {
                Err(SystemError::NotSupported {
                    feature: "linux_compat_layer".into(),
                    reason: "LinuxCompatibilityLayer requires target_os = linux. Current platform is not Linux.".into(),
                }
                .into())
            });
        }
        if uname_info == "not_linux" {
            return Box::pin(async move {
                Err(SystemError::NotSupported {
                    feature: "linux_compat_layer".into(),
                    reason: "Platform detection failed: not running on Linux.".into(),
                }
                .into())
            });
        }
        self.uname_info = Some(uname_info);
        Box::pin(async move { Ok(()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}
