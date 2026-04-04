// SPDX-License-Identifier: AGPL-3.0-only
//! GPU platform detection (NVIDIA CUDA, AMD ROCm).

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

/// Detect GPU platforms.
#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Sync probe; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();

    if probe::command_exists("nvidia-smi") {
        platforms.push(PlatformType::GPU {
            vendor: "NVIDIA".to_string(),
            framework: "CUDA".to_string(),
        });
    }
    if probe::command_exists("rocm-smi") {
        platforms.push(PlatformType::GPU {
            vendor: "AMD".to_string(),
            framework: "ROCm".to_string(),
        });
    }

    Ok(platforms)
}
