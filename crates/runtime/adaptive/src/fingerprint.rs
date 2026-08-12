// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU fingerprinting for hardware identification
//!
//! Uniquely identifies GPU hardware for cache lookup and optimization.
//! Uses capability-based discovery (no hardcoding!).

use crate::error::AdaptiveError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// GPU vendor identification
///
/// Detected at runtime via wgpu adapter info.
/// No hardcoded assumptions about vendor-specific behavior!
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum GpuVendor {
    /// AMD GPUs
    AMD,
    /// NVIDIA GPUs
    NVIDIA,
    /// Intel GPUs
    Intel,
    /// Apple Silicon
    Apple,
    /// Qualcomm GPUs
    Qualcomm,
    /// ARM Mali GPUs
    ARM,
    /// Software/CPU fallback
    Software,
    /// Unknown vendor
    Unknown,
}

impl fmt::Display for GpuVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AMD => write!(f, "AMD"),
            Self::NVIDIA => write!(f, "NVIDIA"),
            Self::Intel => write!(f, "Intel"),
            Self::Apple => write!(f, "Apple"),
            Self::Qualcomm => write!(f, "Qualcomm"),
            Self::ARM => write!(f, "ARM"),
            Self::Software => write!(f, "Software"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// GPU fingerprint for unique hardware identification
///
/// Identifies GPU hardware characteristics for cache lookup.
/// Does NOT hardcode vendor-specific optimizations - all configs learned at runtime!
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct GpuFingerprint {
    /// GPU vendor (AMD, NVIDIA, Intel, etc.)
    pub vendor: GpuVendor,
    /// Architecture name (RDNA2, Ampere, Xe, etc.)
    pub architecture: String,
    /// Model class (`high_end`, `mid_range`, mobile, etc.)
    pub model_class: String,
    /// Driver version (for cache invalidation)
    pub driver_version: String,
    /// Backend being used (Vulkan, Metal, DX12, etc.)
    pub backend: String,
    /// Approximate memory size in GB (rounded)
    pub memory_size_gb: u64,
}

impl GpuFingerprint {
    /// Discover GPU hardware at runtime via wgpu adapter probing.
    ///
    /// Requires the `gpu-discovery` feature (enabled by default).
    /// Without that feature, returns an error immediately.
    ///
    /// # Errors
    ///
    /// Returns error if GPU discovery fails or the `gpu-discovery` feature is disabled.
    #[cfg(feature = "gpu-discovery")]
    #[cfg_attr(target_env = "musl", allow(unreachable_code))]
    pub async fn discover() -> Result<Self, AdaptiveError> {
        #[cfg(target_env = "musl")]
        {
            return Err(AdaptiveError::Other(
                "GPU discovery skipped on musl (Vulkan dlopen incompatible with static linking)"
                    .to_string(),
            ));
        }
        let instance = std::panic::catch_unwind(|| {
            wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            })
        })
        .map_err(|_| {
            AdaptiveError::Other(
                "No wgpu backend available for this platform".to_string(),
            )
        })?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|e| AdaptiveError::Other(format!("Failed to find GPU adapter: {e}")))?;

        let info = adapter.get_info();

        let vendor = Self::detect_vendor(info.vendor);
        let architecture = Self::extract_architecture(&info.name, vendor);
        let model_class = Self::classify_model(&info.name, vendor);
        let driver_version = format!("{} ({})", info.driver, info.driver_info);
        let backend = format!("{:?}", info.backend);
        let limits = adapter.limits();
        let memory_size_gb = (limits.max_buffer_size / 1_000_000_000).max(1);

        Ok(Self {
            vendor,
            architecture,
            model_class,
            driver_version,
            backend,
            memory_size_gb,
        })
    }

    /// Fallback when `gpu-discovery` feature is disabled — returns software fingerprint.
    #[cfg(not(feature = "gpu-discovery"))]
    pub async fn discover() -> Result<Self, AdaptiveError> {
        Ok(Self {
            vendor: GpuVendor::Software,
            architecture: "cpu-fallback".to_string(),
            model_class: "software".to_string(),
            driver_version: "none".to_string(),
            backend: "cpu".to_string(),
            memory_size_gb: 0,
        })
    }

    /// Detect vendor from PCI vendor ID
    const fn detect_vendor(vendor_id: u32) -> GpuVendor {
        match vendor_id {
            0x1002 => GpuVendor::AMD,
            0x10DE => GpuVendor::NVIDIA,
            0x8086 => GpuVendor::Intel,
            0x106B => GpuVendor::Apple,
            0x5143 => GpuVendor::Qualcomm,
            0x13B5 => GpuVendor::ARM,
            0x0000 => GpuVendor::Software,
            _ => GpuVendor::Unknown,
        }
    }

    /// Extract architecture from device name
    ///
    /// Best-effort parsing - not used for optimization decisions!
    /// All optimizations learned at runtime.
    fn extract_architecture(name: &str, vendor: GpuVendor) -> String {
        let name_lower = name.to_lowercase();

        match vendor {
            GpuVendor::AMD => {
                if name_lower.contains("rdna3") || name_lower.contains("rx 7") {
                    "RDNA3".to_string()
                } else if name_lower.contains("rdna2") || name_lower.contains("rx 6") {
                    "RDNA2".to_string()
                } else if name_lower.contains("rdna") || name_lower.contains("rx 5") {
                    "RDNA".to_string()
                } else {
                    "GCN".to_string()
                }
            }
            GpuVendor::NVIDIA => {
                if name_lower.contains("40") || name_lower.contains("ada") {
                    "Ada Lovelace".to_string()
                } else if name_lower.contains("30") || name_lower.contains("ampere") {
                    "Ampere".to_string()
                } else if name_lower.contains("20") || name_lower.contains("turing") {
                    "Turing".to_string()
                } else {
                    "Unknown".to_string()
                }
            }
            GpuVendor::Intel => {
                if name_lower.contains("arc") {
                    "Arc".to_string()
                } else if name_lower.contains("xe") {
                    "Xe".to_string()
                } else {
                    "UHD".to_string()
                }
            }
            GpuVendor::Apple => {
                if name_lower.contains("m3") {
                    "M3".to_string()
                } else if name_lower.contains("m2") {
                    "M2".to_string()
                } else if name_lower.contains("m1") {
                    "M1".to_string()
                } else {
                    "Apple Silicon".to_string()
                }
            }
            _ => "Unknown".to_string(),
        }
    }

    /// Classify model tier (for cache grouping)
    fn classify_model(name: &str, _vendor: GpuVendor) -> String {
        let name_lower = name.to_lowercase();

        // High-end indicators
        if name_lower.contains("rtx 4090")
            || name_lower.contains("rtx 3090")
            || name_lower.contains("rx 7900")
            || name_lower.contains("rx 6950")
            || name_lower.contains("m1 max")
            || name_lower.contains("m1 ultra")
        {
            return "high_end".to_string();
        }

        // Mid-range indicators
        if name_lower.contains("rtx 3060")
            || name_lower.contains("rx 6700")
            || name_lower.contains("m1 pro")
        {
            return "mid_range".to_string();
        }

        // Mobile indicators
        if name_lower.contains("mobile") || name_lower.contains("laptop") {
            return "mobile".to_string();
        }

        // Integrated indicators
        if name_lower.contains("integrated")
            || name_lower.contains("uhd")
            || name_lower.contains("iris")
        {
            return "integrated".to_string();
        }

        "unknown".to_string()
    }

    /// Generate cache key for storage
    ///
    /// Creates unique key for storing optimization configs.
    #[must_use]
    pub fn cache_key(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.vendor,
            self.architecture.replace(' ', "_"),
            self.model_class,
            self.backend
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_detection() {
        assert_eq!(GpuFingerprint::detect_vendor(0x1002), GpuVendor::AMD);
        assert_eq!(GpuFingerprint::detect_vendor(0x10DE), GpuVendor::NVIDIA);
        assert_eq!(GpuFingerprint::detect_vendor(0x8086), GpuVendor::Intel);
    }

    #[test]
    fn test_architecture_extraction() {
        assert_eq!(
            GpuFingerprint::extract_architecture("AMD Radeon RX 6950 XT", GpuVendor::AMD),
            "RDNA2"
        );
        assert_eq!(
            GpuFingerprint::extract_architecture("NVIDIA GeForce RTX 3090", GpuVendor::NVIDIA),
            "Ampere"
        );
    }

    #[test]
    fn test_model_classification() {
        assert_eq!(
            GpuFingerprint::classify_model("NVIDIA GeForce RTX 3090", GpuVendor::NVIDIA),
            "high_end"
        );
        assert_eq!(
            GpuFingerprint::classify_model("AMD Radeon RX 6950 XT", GpuVendor::AMD),
            "high_end"
        );
    }

    #[tokio::test]
    async fn test_gpu_discovery() {
        // Should not panic - graceful if no GPU
        let result = GpuFingerprint::discover().await;

        if let Ok(fingerprint) = result {
            assert!(!fingerprint.architecture.is_empty());
            assert!(fingerprint.memory_size_gb > 0);
        } else {
            eprintln!("Note: No GPU available for testing");
        }
    }
}
