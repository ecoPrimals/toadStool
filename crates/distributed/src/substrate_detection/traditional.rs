//! Traditional OS platform detection (Linux, Windows, macOS).

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

/// Detect traditional operating system platforms.
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
