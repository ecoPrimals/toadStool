// SPDX-License-Identifier: AGPL-3.0-or-later
//! Windows compatibility layer.
//!
//! On Windows, initializes successfully. On other platforms, returns
//! `PlatformNotAvailable` from initialize.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult};
use toadstool_common::error::SystemError;

use super::CompatibilityLayer;

/// Windows compatibility layer.
///
/// On Windows, initializes successfully. On other platforms, returns
/// `PlatformNotAvailable` from initialize.
#[derive(Debug)]
pub struct WindowsCompatibilityLayer {
    config: WindowsCompatConfig,
}

/// Configuration for Windows compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsCompatConfig {
    /// Enable job object control
    pub job_object_control: bool,
    /// Enable token restriction
    pub token_restriction: bool,
    /// Enable `AppContainer` isolation
    pub app_container_isolation: bool,
    /// Enable integrity levels
    pub integrity_levels: bool,
}

impl Default for WindowsCompatConfig {
    fn default() -> Self {
        Self {
            job_object_control: true,
            token_restriction: true,
            app_container_isolation: true,
            integrity_levels: true,
        }
    }
}

impl Default for WindowsCompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsCompatibilityLayer {
    /// Creates a new Windows compatibility layer with default config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: WindowsCompatConfig::default(),
        }
    }

    /// Returns the Windows compatibility config.
    #[must_use]
    pub const fn get_config(&self) -> &WindowsCompatConfig {
        &self.config
    }
}

impl CompatibilityLayer for WindowsCompatibilityLayer {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn features(&self) -> Vec<String> {
        vec!["job_objects".to_string(), "tokens".to_string()]
    }

    fn can_handle(&self, _request: &ExecutionRequest) -> bool {
        cfg!(target_os = "windows")
    }

    fn execute_with_compatibility(
        &self,
        _request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            if !cfg!(target_os = "windows") {
                return Err(SystemError::NotSupported {
                    feature: "windows_compat_layer".into(),
                    reason: "WindowsCompatibilityLayer requires target_os = windows".into(),
                }
                .into());
            }
            Ok(ExecutionResponse::default())
        })
    }

    fn initialize(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        if !cfg!(target_os = "windows") {
            return Box::pin(async move {
                Err(SystemError::NotSupported {
                    feature: "windows_compat_layer".into(),
                    reason: "WindowsCompatibilityLayer requires target_os = windows. Current platform is not Windows.".into(),
                }
                .into())
            });
        }
        Box::pin(async move { Ok(()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}
