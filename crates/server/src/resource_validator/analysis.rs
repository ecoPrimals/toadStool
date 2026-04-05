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
