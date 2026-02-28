//! GPU platform detection (NVIDIA CUDA, AMD ROCm).

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

/// Detect GPU platforms.
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
