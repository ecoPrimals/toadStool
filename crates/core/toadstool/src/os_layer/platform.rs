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
    #[must_use]
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

    /// Check if a specific feature is available
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// Get a summary string of the platform
    pub fn summary(&self) -> String {
        format!("{} {} ({})", self.os, self.arch, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detect() {
        let info = PlatformInfo::detect();

        // OS should be detected
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());

        // Should have standard features
        assert!(info.features.contains(&"universal".to_string()));
        assert!(info.features.contains(&"cross-platform".to_string()));
    }

    #[test]
    fn test_platform_serialization() {
        let info = PlatformInfo {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            version: "5.10.0".to_string(),
            kernel: "5.10.0-generic".to_string(),
            features: vec!["test".to_string()],
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize");
        let deserialized: PlatformInfo =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.os, "linux");
        assert_eq!(deserialized.arch, "x86_64");
    }

    #[test]
    fn test_has_feature() {
        let info = PlatformInfo::detect();

        assert!(info.has_feature("universal"));
        assert!(info.has_feature("cross-platform"));
        assert!(!info.has_feature("nonexistent-feature"));
    }

    #[test]
    fn test_summary() {
        let info = PlatformInfo {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            version: "5.10.0".to_string(),
            kernel: "5.10.0-generic".to_string(),
            features: vec![],
        };

        let summary = info.summary();
        assert!(summary.contains("linux"));
        assert!(summary.contains("x86_64"));
        assert!(summary.contains("5.10.0"));
    }

    #[test]
    fn test_platform_clone() {
        let info = PlatformInfo::detect();
        let cloned = info.clone();

        assert_eq!(info.os, cloned.os);
        assert_eq!(info.arch, cloned.arch);
    }

    #[test]
    fn test_default_features() {
        let info = PlatformInfo::detect();

        assert_eq!(info.features.len(), 4);
        assert!(info.has_feature("universal"));
        assert!(info.has_feature("cross-platform"));
        assert!(info.has_feature("container-aware"));
        assert!(info.has_feature("resource-monitoring"));
    }

    #[test]
    fn test_platform_info_fields() {
        let info = PlatformInfo {
            os: "windows".to_string(),
            arch: "aarch64".to_string(),
            version: "10.0".to_string(),
            kernel: "NT 10.0".to_string(),
            features: vec!["feature1".to_string(), "feature2".to_string()],
        };

        assert_eq!(info.os, "windows");
        assert_eq!(info.arch, "aarch64");
        assert_eq!(info.version, "10.0");
        assert_eq!(info.kernel, "NT 10.0");
        assert_eq!(info.features.len(), 2);
    }
}
