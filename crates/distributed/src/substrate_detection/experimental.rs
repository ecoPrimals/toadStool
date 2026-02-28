//! Experimental platform detection placeholder.

use super::types::PlatformType;
use toadstool::ToadStoolResult;

/// Detect experimental platforms.
/// Placeholder for future hardware/software-specific detection.
#[allow(clippy::unused_async)] // Placeholder; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    Ok(Vec::new())
}
