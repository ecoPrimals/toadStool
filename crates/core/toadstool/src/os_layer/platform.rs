use serde::{Deserialize, Serialize};

/// Platform information for the current system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Operating system name
    pub os: String,
    /// Architecture
    pub arch: String,
    /// OS version
    pub version: String,
    /// Kernel version
    pub kernel: String,
    /// Available features
    pub features: Vec<String>,
}

impl PlatformInfo {
    /// Detect current platform information
    pub fn detect() -> Self {
        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();
        let version = "unknown".to_string();
        let kernel = "unknown".to_string();

        let features = vec![
            "universal".to_string(),
            "cross-platform".to_string(),
            "container-aware".to_string(),
            "resource-monitoring".to_string(),
        ];

        Self {
            os,
            arch,
            version,
            kernel,
            features,
        }
    }
}
