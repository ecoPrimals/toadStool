// SPDX-License-Identifier: AGPL-3.0-or-later
//! Traditional OS platform detection (Linux, Windows, macOS).

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

/// Detect traditional operating system platforms.
#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Sync probe; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match os {
        "linux" => {
            if let Ok(distro) = probe::detect_linux_distribution() {
                platforms.push(PlatformType::Linux {
                    distribution: distro,
                    architecture: arch.to_string(),
                });
            }
        }
        "windows" => {
            platforms.push(PlatformType::Windows {
                version: "Unknown".to_string(),
                architecture: arch.to_string(),
            });
        }
        "macos" => {
            platforms.push(PlatformType::MacOS {
                version: "Unknown".to_string(),
                architecture: arch.to_string(),
            });
        }
        _ => {
            platforms.push(PlatformType::Other {
                os: os.to_string(),
                architecture: arch.to_string(),
            });
        }
    }

    Ok(platforms)
}
