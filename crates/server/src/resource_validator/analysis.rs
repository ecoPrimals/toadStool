// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compares estimated requirements to discovered capabilities (gaps and warnings).

use crate::resource_estimator::ResourceEstimate;

use super::types::{ResourceGap, SystemCapabilities};

/// Identify resource gaps
///
/// Compares estimated requirements against system capabilities and
/// identifies what's missing.
pub(crate) fn identify_gaps(
    estimate: &ResourceEstimate,
    capabilities: &SystemCapabilities,
) -> Vec<ResourceGap> {
    let mut gaps = Vec::new();

    // Check CPU
    if estimate.cpu_cores > capabilities.available_cpu_cores {
        gaps.push(ResourceGap {
            resource_type: "cpu_cores".to_string(),
            required: u64::from(estimate.cpu_cores),
            available: u64::from(capabilities.available_cpu_cores),
            shortage: u64::from(estimate.cpu_cores - capabilities.available_cpu_cores),
            suggestion: format!(
                "Need {} more CPU cores. Consider reducing parallelism or waiting for resources.",
                estimate.cpu_cores - capabilities.available_cpu_cores
            ),
        });
    }

    // Check memory
    if estimate.memory_bytes > capabilities.available_memory_bytes {
        gaps.push(ResourceGap {
            resource_type: "memory".to_string(),
            required: estimate.memory_bytes,
            available: capabilities.available_memory_bytes,
            shortage: estimate.memory_bytes - capabilities.available_memory_bytes,
            suggestion: format!(
                "Need {} GB more memory. Consider streaming data or reducing batch size.",
                (estimate.memory_bytes - capabilities.available_memory_bytes)
                    / (1024 * 1024 * 1024)
            ),
        });
    }

    // Check GPU memory
    if estimate.gpu_memory_bytes > 0
        && estimate.gpu_memory_bytes > capabilities.available_gpu_memory_bytes
    {
        gaps.push(ResourceGap {
            resource_type: "gpu_memory".to_string(),
            required: estimate.gpu_memory_bytes,
            available: capabilities.available_gpu_memory_bytes,
            shortage: estimate.gpu_memory_bytes - capabilities.available_gpu_memory_bytes,
            suggestion: if capabilities.gpu_count == 0 {
                "No GPU detected. Consider using CPU fallback or acquiring GPU resources."
                    .to_string()
            } else {
                format!(
                    "Need {} GB more GPU memory. Consider model quantization or sharding.",
                    (estimate.gpu_memory_bytes - capabilities.available_gpu_memory_bytes)
                        / (1024 * 1024 * 1024)
                )
            },
        });
    }

    // Check storage
    if estimate.storage_bytes > capabilities.available_storage_bytes {
        gaps.push(ResourceGap {
            resource_type: "storage".to_string(),
            required: estimate.storage_bytes,
            available: capabilities.available_storage_bytes,
            shortage: estimate.storage_bytes - capabilities.available_storage_bytes,
            suggestion: format!(
                "Need {} GB more storage. Consider cleaning up or using remote storage.",
                (estimate.storage_bytes - capabilities.available_storage_bytes)
                    / (1024 * 1024 * 1024)
            ),
        });
    }

    gaps
}

/// Generate warnings for tight resources
///
/// Even if resources are technically available, warn if they're close to limits.
pub(crate) fn generate_warnings(
    estimate: &ResourceEstimate,
    capabilities: &SystemCapabilities,
) -> Vec<String> {
    let mut warnings = Vec::new();

    // Warn if CPU usage is > 70%
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )]
    let cpu_usage = estimate.cpu_cores as f32 / capabilities.available_cpu_cores as f32;
    if cpu_usage > 0.7 && cpu_usage <= 1.0 {
        warnings.push(format!(
            "High CPU usage: {:.0}% of available cores. Performance may be impacted.",
            cpu_usage * 100.0
        ));
    }

    // Warn if memory usage is > 70%
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )]
    let memory_usage = estimate.memory_bytes as f32 / capabilities.available_memory_bytes as f32;
    if memory_usage > 0.7 && memory_usage <= 1.0 {
        warnings.push(format!(
            "High memory usage: {:.0}% of available memory. Risk of swapping.",
            memory_usage * 100.0
        ));
    }

    // Warn if GPU memory usage is > 70%
    if capabilities.total_gpu_memory_bytes > 0 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "precision loss acceptable for this conversion"
        )]
        let gpu_usage =
            estimate.gpu_memory_bytes as f32 / capabilities.available_gpu_memory_bytes as f32;
        if gpu_usage > 0.7 && gpu_usage <= 1.0 {
            warnings.push(format!(
                "High GPU memory usage: {:.0}% of available GPU memory. May cause OOM.",
                gpu_usage * 100.0
            ));
        }
    }

    // Warn if storage usage is > 80%
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )]
    let storage_usage = estimate.storage_bytes as f32 / capabilities.available_storage_bytes as f32;
    if storage_usage > 0.8 && storage_usage <= 1.0 {
        warnings.push(format!(
            "High storage usage: {:.0}% of available storage. Consider cleanup.",
            storage_usage * 100.0
        ));
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estimate(cpu: u32, mem: u64, gpu: u64, storage: u64) -> ResourceEstimate {
        ResourceEstimate {
            graph_id: "test".to_string(),
            cpu_cores: cpu,
            memory_bytes: mem,
            gpu_memory_bytes: gpu,
            storage_bytes: storage,
            network_bandwidth_mbps: 0,
            estimated_duration: std::time::Duration::from_secs(1),
            max_parallelism: 1,
            critical_path_length: 1,
            node_estimates: std::collections::HashMap::new(),
            warnings: Vec::new(),
        }
    }

    fn caps(cpu: u32, mem: u64, gpu: u64, gpu_total: u64, storage: u64) -> SystemCapabilities {
        SystemCapabilities {
            total_cpu_cores: cpu,
            available_cpu_cores: cpu,
            total_memory_bytes: mem,
            available_memory_bytes: mem,
            total_gpu_memory_bytes: gpu_total,
            available_gpu_memory_bytes: gpu,
            total_storage_bytes: storage,
            available_storage_bytes: storage,
            network_bandwidth_mbps: 1000,
            gpu_count: usize::from(gpu_total > 0),
            gpu_types: Vec::new(),
        }
    }

    #[test]
    fn no_gaps_when_capacity_sufficient() {
        let gaps = identify_gaps(&estimate(4, 8_000, 0, 1_000), &caps(8, 16_000, 0, 0, 2_000));
        assert!(gaps.is_empty());
    }

    #[test]
    fn cpu_gap_detected() {
        let gaps = identify_gaps(&estimate(8, 1_000, 0, 100), &caps(4, 2_000, 0, 0, 200));
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].resource_type, "cpu_cores");
        assert_eq!(gaps[0].shortage, 4);
    }

    #[test]
    fn memory_gap_detected() {
        let gb = 1024 * 1024 * 1024;
        let gaps = identify_gaps(&estimate(1, 16 * gb, 0, 100), &caps(2, 8 * gb, 0, 0, 200));
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].resource_type, "memory");
    }

    #[test]
    fn gpu_gap_with_no_gpu_suggests_cpu_fallback() {
        let gb = 1024 * 1024 * 1024;
        let gaps = identify_gaps(&estimate(1, 1_000, 4 * gb, 100), &caps(2, 2_000, 0, 0, 200));
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].resource_type, "gpu_memory");
        assert!(gaps[0].suggestion.contains("No GPU"));
    }

    #[test]
    fn gpu_gap_with_insufficient_gpu_suggests_quantization() {
        let gb = 1024 * 1024 * 1024;
        let gaps = identify_gaps(
            &estimate(1, 1_000, 8 * gb, 100),
            &caps(2, 2_000, 4 * gb, 8 * gb, 200),
        );
        assert_eq!(gaps.len(), 1);
        assert!(gaps[0].suggestion.contains("quantization"));
    }

    #[test]
    fn storage_gap_detected() {
        let gb = 1024 * 1024 * 1024;
        let gaps = identify_gaps(&estimate(1, 100, 0, 50 * gb), &caps(2, 200, 0, 0, 10 * gb));
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].resource_type, "storage");
    }

    #[test]
    fn multiple_gaps() {
        let gb = 1024 * 1024 * 1024;
        let gaps = identify_gaps(
            &estimate(8, 16 * gb, 0, 50 * gb),
            &caps(4, 8 * gb, 0, 0, 10 * gb),
        );
        assert_eq!(gaps.len(), 3);
    }

    #[test]
    fn no_warnings_when_usage_low() {
        let warnings =
            generate_warnings(&estimate(2, 4_000, 0, 500), &caps(10, 16_000, 0, 0, 2_000));
        assert!(warnings.is_empty());
    }

    #[test]
    fn cpu_warning_at_75_percent() {
        let warnings = generate_warnings(&estimate(3, 100, 0, 100), &caps(4, 1_000, 0, 0, 1_000));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("CPU"));
    }

    #[test]
    fn memory_warning_at_80_percent() {
        let warnings = generate_warnings(&estimate(1, 800, 0, 100), &caps(10, 1_000, 0, 0, 1_000));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("memory"));
    }

    #[test]
    fn storage_warning_at_85_percent() {
        let warnings = generate_warnings(&estimate(1, 100, 0, 850), &caps(10, 1_000, 0, 0, 1_000));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("storage"));
    }

    #[test]
    fn no_gpu_warning_when_no_gpu() {
        let warnings =
            generate_warnings(&estimate(1, 100, 500, 100), &caps(10, 1_000, 0, 0, 1_000));
        assert!(warnings.is_empty());
    }
}
