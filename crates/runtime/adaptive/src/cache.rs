// SPDX-License-Identifier: AGPL-3.0-or-later
//! Optimization cache for persistent configuration storage
//!
//! Stores learned optimal configurations across runs.
//! Platform-agnostic storage location (XDG dirs on Linux, etc.).

use crate::error::AdaptiveError;
use crate::fingerprint::GpuFingerprint;
use crate::types::{OpType, SizeClass};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Workgroup configuration for specific operation + size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkgroupConfig {
    /// Optimal workgroup size
    pub workgroup_size: usize,
    /// Average performance in microseconds
    pub performance_us: f64,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Number of measurements taken
    pub sample_count: usize,
    /// Last validation time
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_validated: SystemTime,
}

impl WorkgroupConfig {
    /// Create new workgroup config
    #[must_use]
    pub fn new(workgroup_size: usize, performance_us: f64) -> Self {
        Self {
            workgroup_size,
            performance_us,
            confidence: 0.8, // Initial confidence
            sample_count: 1,
            last_validated: SystemTime::now(),
        }
    }

    /// Update with new measurement
    pub fn update(&mut self, performance_us: f64) {
        // Running average
        let alpha: f64 = 0.3;
        self.performance_us = (1.0 - alpha).mul_add(self.performance_us, alpha * performance_us);

        self.sample_count += 1;
        self.confidence = (self.confidence + 0.05).min(1.0);
        self.last_validated = SystemTime::now();
    }
}

/// Operation profile containing size-specific configs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationProfile {
    /// Operation type
    pub op_type: OpType,
    /// Configurations per size class
    pub size_configs: HashMap<SizeClass, WorkgroupConfig>,
}

impl OperationProfile {
    /// Create new operation profile
    #[must_use]
    pub fn new(op_type: OpType) -> Self {
        Self {
            op_type,
            size_configs: HashMap::new(),
        }
    }

    /// Add configuration for size class
    pub fn add_config(&mut self, size_class: SizeClass, config: WorkgroupConfig) {
        self.size_configs.insert(size_class, config);
    }

    /// Get configuration for size class
    #[must_use]
    pub fn get_config(&self, size_class: SizeClass) -> Option<&WorkgroupConfig> {
        self.size_configs.get(&size_class)
    }
}

/// Optimization cache storing learned configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationCache {
    /// Cache format version
    pub version: u32,
    /// GPU fingerprint this cache is for
    pub gpu_fingerprint: GpuFingerprint,
    /// Creation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
    /// Last update timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_updated: SystemTime,
    /// Profiles per operation
    pub profiles: HashMap<OpType, OperationProfile>,
}

impl OptimizationCache {
    /// Cache format version
    const VERSION: u32 = 1;

    /// Load cache from disk or create new
    ///
    /// Platform-agnostic cache location using dirs crate.
    ///
    /// # Errors
    ///
    /// Returns error if file I/O fails.
    pub fn load_or_create(gpu: &GpuFingerprint) -> Result<Self, AdaptiveError> {
        let cache_path = Self::cache_path(gpu)?;

        if cache_path.exists() {
            // Load existing cache
            let contents = fs::read_to_string(&cache_path)
                .map_err(|e| AdaptiveError::Other(format!("Failed to read cache file: {e}")))?;

            let mut cache: Self = serde_json::from_str(&contents)
                .map_err(|e| AdaptiveError::Other(format!("Failed to parse cache file: {e}")))?;

            // Verify cache is for this GPU
            if cache.gpu_fingerprint.cache_key() != gpu.cache_key() {
                tracing::warn!("Cache GPU mismatch - creating new cache");
                return Ok(Self::new(gpu.clone()));
            }

            // Check if cache is stale (driver changed)
            if cache.gpu_fingerprint.driver_version != gpu.driver_version {
                tracing::info!("Driver version changed - invalidating cache");
                cache.invalidate_stale();
            }

            cache.last_updated = SystemTime::now();
            Ok(cache)
        } else {
            // Create new cache
            Ok(Self::new(gpu.clone()))
        }
    }

    /// Create new empty cache
    pub(crate) fn new(gpu_fingerprint: GpuFingerprint) -> Self {
        let now = SystemTime::now();
        Self {
            version: Self::VERSION,
            gpu_fingerprint,
            created_at: now,
            last_updated: now,
            profiles: HashMap::new(),
        }
    }

    /// Save cache to disk
    ///
    /// # Errors
    ///
    /// Returns error if file I/O fails.
    pub fn save(&self) -> Result<(), AdaptiveError> {
        let cache_path = Self::cache_path(&self.gpu_fingerprint)?;

        // Ensure directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AdaptiveError::Other(format!("Failed to create cache directory: {e}"))
            })?;
        }

        // Serialize to JSON
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| AdaptiveError::Other(format!("Failed to serialize cache: {e}")))?;

        // Write atomically (write to temp, then rename)
        let temp_path = cache_path.with_extension("tmp");
        fs::write(&temp_path, contents)
            .map_err(|e| AdaptiveError::Other(format!("Failed to write cache file: {e}")))?;
        fs::rename(temp_path, cache_path)
            .map_err(|e| AdaptiveError::Other(format!("Failed to rename cache file: {e}")))?;

        Ok(())
    }

    /// Get cache file path
    ///
    /// Platform-specific location using Pure Rust etcetera:
    /// - Linux: ~/.cache/toadstool-gpu/
    /// - macOS: ~/Library/Caches/toadstool-gpu/
    /// - Windows: %LOCALAPPDATA%\toadstool-gpu\
    fn cache_path(gpu: &GpuFingerprint) -> Result<PathBuf, AdaptiveError> {
        use etcetera::{BaseStrategy, choose_base_strategy};
        const GPU_CACHE_NAMESPACE: &str = "toadstool-gpu";

        let strategy = choose_base_strategy().map_err(|e| {
            AdaptiveError::Other(format!("Failed to determine base directory strategy: {e}"))
        })?;
        let cache_dir = strategy.cache_dir();

        let gpu_cache = cache_dir.join(GPU_CACHE_NAMESPACE);
        let filename = format!("optimization_{}.json", gpu.cache_key());

        Ok(gpu_cache.join(filename))
    }

    /// Get optimal configuration for operation + size
    #[must_use]
    pub fn get_optimal(&self, op_type: OpType, size: usize) -> Option<&WorkgroupConfig> {
        let size_class = SizeClass::from_size(size);
        self.profiles.get(&op_type)?.get_config(size_class)
    }

    /// Add operation profile
    pub fn add_profile(&mut self, profile: OperationProfile) {
        self.profiles.insert(profile.op_type, profile);
        self.last_updated = SystemTime::now();
    }

    /// Update with new measurement
    pub fn update_measurement(
        &mut self,
        op_type: OpType,
        size: usize,
        workgroup: usize,
        performance_us: f64,
    ) {
        let size_class = SizeClass::from_size(size);

        let profile = self
            .profiles
            .entry(op_type)
            .or_insert_with(|| OperationProfile::new(op_type));

        if let Some(config) = profile.size_configs.get_mut(&size_class) {
            if config.workgroup_size == workgroup {
                // Update existing config
                config.update(performance_us);
            } else {
                // Replace with better config
                *config = WorkgroupConfig::new(workgroup, performance_us);
            }
        } else {
            // Add new config
            profile.add_config(size_class, WorkgroupConfig::new(workgroup, performance_us));
        }

        self.last_updated = SystemTime::now();
    }

    /// Invalidate stale entries
    ///
    /// Removes low-confidence or old entries.
    pub fn invalidate_stale(&mut self) {
        for profile in self.profiles.values_mut() {
            profile.size_configs.retain(|_, config| {
                config.confidence > 0.5 // Keep only high-confidence entries
            });
        }

        // Remove empty profiles
        self.profiles
            .retain(|_, profile| !profile.size_configs.is_empty());

        self.last_updated = SystemTime::now();
    }

    /// Check if cache is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Clear all cached configurations
    pub fn clear(&mut self) {
        self.profiles.clear();
        self.last_updated = SystemTime::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::GpuVendor;

    fn mock_fingerprint() -> GpuFingerprint {
        GpuFingerprint {
            vendor: GpuVendor::NVIDIA,
            architecture: "Ampere".to_string(),
            model_class: "high_end".to_string(),
            driver_version: "1.0".to_string(),
            backend: "Vulkan".to_string(),
            memory_size_gb: 24,
        }
    }

    #[test]
    fn test_workgroup_config_update() {
        let mut config = WorkgroupConfig::new(128, 1000.0);
        assert_eq!(config.sample_count, 1);
        assert_eq!(config.workgroup_size, 128);

        config.update(900.0);
        assert_eq!(config.sample_count, 2);
        assert!(config.performance_us < 1000.0);
        assert!(config.confidence > 0.8);
    }

    #[test]
    fn test_operation_profile() {
        let mut profile = OperationProfile::new(OpType::MatMul);
        let config = WorkgroupConfig::new(128, 1000.0);

        profile.add_config(SizeClass::Medium, config);
        assert!(profile.get_config(SizeClass::Medium).is_some());
        assert!(profile.get_config(SizeClass::Large).is_none());
    }

    #[test]
    fn test_optimization_cache() {
        let gpu = mock_fingerprint();
        let mut cache = OptimizationCache::new(gpu);

        assert!(cache.is_empty());

        // Add measurement
        cache.update_measurement(OpType::MatMul, 100_000, 128, 1000.0);
        assert!(!cache.is_empty());

        // Retrieve config
        let config = cache.get_optimal(OpType::MatMul, 100_000);
        assert!(config.is_some());
        assert_eq!(config.unwrap().workgroup_size, 128);
    }

    #[test]
    fn test_cache_invalidation() {
        let gpu = mock_fingerprint();
        let mut cache = OptimizationCache::new(gpu);

        // Add low-confidence config
        cache.update_measurement(OpType::MatMul, 100_000, 128, 1000.0);
        if let Some(profile) = cache.profiles.get_mut(&OpType::MatMul)
            && let Some(config) = profile.size_configs.get_mut(&SizeClass::Small)
        {
            config.confidence = 0.3; // Set low confidence
        }

        cache.invalidate_stale();

        // Low-confidence entries should be removed
        // (May still have medium size class)
        let has_stale = cache.profiles.values().any(|profile| {
            profile
                .size_configs
                .values()
                .any(|config| config.confidence < 0.5)
        });
        assert!(!has_stale);
    }
}
