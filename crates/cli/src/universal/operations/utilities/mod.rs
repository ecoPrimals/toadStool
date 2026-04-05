// SPDX-License-Identifier: AGPL-3.0-only
//! Utility Operations
//!
//! Extension trait for utility helper methods.

use crate::Result;
use crate::universal::types::{GpuInfo, HardwareInfo};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use toadstool_distributed::substrate_detection::PlatformType;

mod hardware;
mod platform_id;
mod platform_metadata;

#[cfg(test)]
mod tests;

/// Utility operations trait
pub trait UtilityOps {
    /// Get platform ID from platform type
    fn get_platform_id(&self, platform: &PlatformType) -> String;

    /// Get platform metadata (`Arc<str>` values = zero-copy clone)
    fn get_platform_metadata(&self, platform: &PlatformType) -> HashMap<String, Arc<str>>;

    /// Get system hardware information
    fn get_system_hardware_info(&self) -> impl Future<Output = Result<HardwareInfo>> + Send;

    /// Detect GPU information
    fn detect_gpu_info(&self) -> impl Future<Output = Result<GpuInfo>> + Send;
}

/// Implementation of utility operations
impl UtilityOps for crate::universal::UniversalComputeManager {
    fn get_platform_id(&self, platform: &PlatformType) -> String {
        platform_id::from_platform(platform)
    }

    fn get_platform_metadata(&self, platform: &PlatformType) -> HashMap<String, Arc<str>> {
        platform_metadata::from_platform(platform)
    }

    async fn get_system_hardware_info(&self) -> Result<HardwareInfo> {
        hardware::system_hardware_info().await
    }

    async fn detect_gpu_info(&self) -> Result<GpuInfo> {
        hardware::detect_gpu_info().await
    }
}
