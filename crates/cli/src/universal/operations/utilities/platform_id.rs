// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stable string identifiers for [`PlatformType`] values.

use toadstool_distributed::substrate_detection::PlatformType;

/// Build the canonical platform ID string for storage and comparison.
pub(crate) fn from_platform(platform: &PlatformType) -> String {
    match platform {
        PlatformType::Linux {
            distribution,
            architecture,
        } => {
            format!(
                "linux_{}_{}",
                distribution.to_lowercase().replace(' ', "_"),
                architecture
            )
        }
        PlatformType::MacOS {
            version,
            architecture,
        } => {
            format!("macos_{}_{}", version.replace('.', "_"), architecture)
        }
        PlatformType::Windows {
            version,
            architecture,
        } => {
            format!("windows_{}_{}", version.replace('.', "_"), architecture)
        }
        // **Zero-Copy Optimization** (Nov 28, 2025): String::from is more efficient for literals
        PlatformType::Docker => String::from("docker"),
        PlatformType::Podman => String::from("podman"),
        PlatformType::Containerd => String::from("containerd"),
        PlatformType::WebAssembly { runtime } => format!("wasm_{}", runtime.to_lowercase()),
        PlatformType::Language { name, .. } => format!("lang_{}", name.to_lowercase()),
        PlatformType::GPU { vendor, framework } => {
            format!("gpu_{}_{}", vendor.to_lowercase(), framework.to_lowercase())
        }
        PlatformType::Other { os, architecture } => format!("other_{os}_{architecture}"),
        PlatformType::EdgeDevice {
            device_type,
            architecture,
        } => {
            format!("edge_{device_type}_{architecture}")
        }
        PlatformType::MCUDevelopment { platform, tool } => format!("mcu_{platform}_{tool}"),
        PlatformType::BiologicalComputing {
            platform,
            simulation,
        } => {
            format!("bio_{platform}_{simulation}")
        }
        PlatformType::Quantum {
            framework,
            simulator,
        } => {
            format!("quantum_{framework}_{simulator}")
        }
        PlatformType::NeuromorphicComputing { platform, hardware } => {
            format!("neuro_{platform}_{hardware}")
        }
    }
}
