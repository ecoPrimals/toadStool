// SPDX-License-Identifier: AGPL-3.0-or-later
//! Container runtime detection (Docker, Podman, containerd).

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

/// Detect container platforms.
#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Sync probe; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();

    if probe::command_exists("docker") {
        platforms.push(PlatformType::Docker);
    }
    if probe::command_exists("podman") {
        platforms.push(PlatformType::Podman);
    }
    if probe::command_exists("ctr") {
        platforms.push(PlatformType::Containerd);
    }

    Ok(platforms)
}
