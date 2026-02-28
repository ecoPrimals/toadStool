//! Experimental platform detection placeholder.

use super::types::PlatformType;
use toadstool::ToadStoolResult;

/// Detect experimental platforms.
/// Placeholder for future hardware/software-specific detection.
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    Ok(Vec::new())
}
