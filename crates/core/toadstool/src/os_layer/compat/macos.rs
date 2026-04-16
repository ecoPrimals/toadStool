// SPDX-License-Identifier: AGPL-3.0-or-later
//! macOS compatibility layer.
//!
//! On macOS, initializes successfully. On other platforms, returns
//! `PlatformNotAvailable` from initialize.

use serde::{Deserialize, Serialize};
use std::future::Future;

use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult};
use toadstool_common::error::SystemError;

use super::CompatibilityLayer;

/// macOS compatibility layer.
///
/// On macOS, initializes successfully. On other platforms, returns
/// `PlatformNotAvailable` from initialize.
#[derive(Debug)]
pub struct MacOSCompatibilityLayer {
    config: MacOSCompatConfig,
}

/// Configuration for macOS compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacOSCompatConfig {
    /// Enable sandbox profiles
    pub sandbox_profiles: bool,
    /// Enable System Integrity Protection
    pub sip_integration: bool,
    /// Enable Transparency, Consent & Control
    pub tcc_integration: bool,
    /// Enable code signing verification
    pub code_signing: bool,
}

impl Default for MacOSCompatConfig {
    fn default() -> Self {
        Self {
            sandbox_profiles: true,
            sip_integration: true,
            tcc_integration: true,
            code_signing: true,
        }
    }
}

impl Default for MacOSCompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOSCompatibilityLayer {
    /// Creates a new macOS compatibility layer with default config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: MacOSCompatConfig::default(),
        }
    }

    /// Returns the macOS compatibility config.
    #[must_use]
    pub const fn get_config(&self) -> &MacOSCompatConfig {
        &self.config
    }
}

impl CompatibilityLayer for MacOSCompatibilityLayer {
    fn name(&self) -> &'static str {
        "macos"
    }

    fn features(&self) -> Vec<String> {
        vec!["sandbox_profiles".to_string(), "sip".to_string()]
    }

    fn can_handle(&self, _request: &ExecutionRequest) -> bool {
        cfg!(target_os = "macos")
    }

    fn execute_with_compatibility(
        &self,
        _request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async move {
            if !cfg!(target_os = "macos") {
                return Err(SystemError::NotSupported {
                    feature: "macos_compat_layer".into(),
                    reason: "MacOSCompatibilityLayer requires target_os = macos".into(),
                }
                .into());
            }
            Ok(ExecutionResponse::default())
        }
    }

    fn initialize(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async {
            if !cfg!(target_os = "macos") {
                return Err(SystemError::NotSupported {
                    feature: "macos_compat_layer".into(),
                    reason: "MacOSCompatibilityLayer requires target_os = macos. Current platform is not macOS.".into(),
                }
                .into());
            }
            Ok(())
        }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
}
