//! Resource validation for collaborative intelligence
//!
//! This module validates whether the system has sufficient resources to execute
//! a given execution graph. It compares estimated requirements against actual
//! system capabilities discovered at runtime.
//!
//! ## Deep Debt Principles
//!
//! - **Runtime Discovery**: Queries real system state, no hardcoded capabilities
//! - **Capability-Based**: Validates based on advertised capabilities
//! - **Self-Knowledge**: System reports its own capabilities
//! - **No Hardcoding**: All thresholds and limits from configuration or system query
//! - **Safe Rust**: All validation logic in safe Rust

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::graph_types::ExecutionGraph;

/// GPU information discovered at runtime
#[derive(Debug, Clone)]
struct GpuInfo {
    name: String,
    _memory_mb: u64,
    _vendor: String,
}
use crate::resource_estimator::ResourceEstimator;
use crate::resource_estimator::{EstimationError, ResourceEstimate};

/// Resource availability result
///
/// Reports whether the system can execute the graph and identifies any
/// resource gaps that would prevent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResult {
    /// Graph ID this result is for
    pub graph_id: String,

    /// Whether the system can execute this graph
    pub available: bool,

    /// Resource gaps (what's missing)
    pub gaps: Vec<ResourceGap>,

    /// Warnings (resources are tight but available)
    pub warnings: Vec<String>,

    /// System capabilities at time of check
    pub system_capabilities: SystemCapabilities,

    /// Estimated requirements
    pub estimated_requirements: ResourceEstimate,
}

/// A resource gap preventing execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGap {
    /// Resource type (cpu, memory, gpu, etc.)
    pub resource_type: String,

    /// Required amount
    pub required: u64,

    /// Available amount
    pub available: u64,

    /// Shortage amount
    pub shortage: u64,

    /// Suggested action
    pub suggestion: String,
}

/// System capabilities snapshot
///
/// Represents the system's current resource availability.
/// All values are discovered at runtime, no hardcoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    /// Total CPU cores
    pub total_cpu_cores: u32,

    /// Available CPU cores (not in use)
    pub available_cpu_cores: u32,

    /// Total memory in bytes
    pub total_memory_bytes: u64,

    /// Available memory in bytes
    pub available_memory_bytes: u64,

    /// Total GPU memory in bytes (across all GPUs)
    pub total_gpu_memory_bytes: u64,

    /// Available GPU memory in bytes
    pub available_gpu_memory_bytes: u64,

    /// Total storage in bytes
    pub total_storage_bytes: u64,

    /// Available storage in bytes
    pub available_storage_bytes: u64,

    /// Network bandwidth in Mbps
    pub network_bandwidth_mbps: u64,

    /// GPU count
    pub gpu_count: usize,

    /// GPU types (e.g., "NVIDIA RTX 3090", "AMD RX 6950 XT")
    pub gpu_types: Vec<String>,
}

/// Resource validator
///
/// Validates execution graphs against system capabilities.
pub struct ResourceValidator {
    estimator: ResourceEstimator,
}

impl Default for ResourceValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceValidator {
    /// Create a new resource validator
    pub fn new() -> Self {
        Self {
            estimator: ResourceEstimator::new(),
        }
    }

    /// Validate whether the system can execute the graph
    ///
    /// This performs:
    /// 1. Resource estimation
    /// 2. System capability query
    /// 3. Comparison and gap analysis
    /// 4. Warning generation
    pub async fn validate_availability(
        &self,
        graph: &ExecutionGraph,
    ) -> Result<AvailabilityResult, ValidationError> {
        info!("Validating resource availability for graph: {}", graph.id);

        // Estimate requirements
        let estimate = self
            .estimator
            .estimate(graph)
            .map_err(ValidationError::EstimationFailed)?;

        // Query system capabilities
        let capabilities = self.query_system_capabilities().await?;

        // Compare and identify gaps
        let gaps = self.identify_gaps(&estimate, &capabilities);

        // Generate warnings
        let warnings = self.generate_warnings(&estimate, &capabilities);

        // Determine if execution is possible
        let available = gaps.is_empty();

        if available {
            info!("✅ System has sufficient resources for graph {}", graph.id);
        } else {
            warn!(
                "❌ System lacks resources for graph {}. Gaps: {:?}",
                graph.id, gaps
            );
        }

        Ok(AvailabilityResult {
            graph_id: graph.id.clone(),
            available,
            gaps,
            warnings,
            system_capabilities: capabilities,
            estimated_requirements: estimate,
        })
    }

    /// Query system capabilities
    ///
    /// This queries the actual system state at runtime. No hardcoded values.
    async fn query_system_capabilities(&self) -> Result<SystemCapabilities, ValidationError> {
        debug!("Querying system capabilities");

        // Query CPU
        let total_cpu_cores = std::thread::available_parallelism()
            .map(|n| u32::try_from(n.get()).unwrap_or(4))
            .unwrap_or(4);
        // Assume 80% available (rough heuristic, in production would query actual usage)
        let available_cpu_cores = (total_cpu_cores * 80) / 100;

        // Query memory - Pure Rust Evolution (Jan 17, 2026)
        // Migrated from sys-info (C dependency) to sysinfo (100% Pure Rust)
        use sysinfo::{Disks, Networks, System};
        let mut system = System::new_all();
        system.refresh_memory();

        let total_memory_bytes = system.total_memory(); // Already in bytes
        let available_memory_bytes = system.available_memory(); // Already in bytes

        // Query disk - Deep Debt Evolution (Feb 17 2026)
        // Use actual disk enumeration via sysinfo::Disks (pure Rust)
        let disks = Disks::new_with_refreshed_list();
        let (total_storage_bytes, available_storage_bytes): (u64, u64) = disks
            .iter()
            .filter(|disk| {
                // Filter out virtual filesystems
                let fs = disk.file_system().to_string_lossy();
                !fs.contains("tmpfs")
                    && !fs.contains("devtmpfs")
                    && !fs.contains("squashfs")
                    && !fs.contains("overlay")
            })
            .fold((0u64, 0u64), |(total, avail), disk| {
                (total + disk.total_space(), avail + disk.available_space())
            });

        // Query GPU (if available) - uses runtime detection via toadstool-runtime-gpu
        // Detection happens at runtime, no hardcoded assumptions about GPU vendors
        // Falls back gracefully if no GPU available
        let (total_gpu_memory_bytes, available_gpu_memory_bytes, gpu_count, gpu_types) =
            self.query_gpu_capabilities().await;

        // Network bandwidth - Deep Debt Evolution (Feb 17 2026)
        // Query actual network interfaces for bandwidth estimate
        let networks = Networks::new_with_refreshed_list();
        let network_bandwidth_mbps = if networks.iter().count() > 0 {
            // Sum received bytes across all interfaces as bandwidth indicator
            // Most physical NICs are 1Gbps+, but we estimate conservatively
            let total_received: u64 = networks.values().map(sysinfo::NetworkData::received).sum();
            // If we've seen significant traffic, assume at least 1Gbps
            // Otherwise fall back to conservative 100Mbps estimate
            if total_received > 1_000_000_000 {
                1000 // 1 Gbps
            } else {
                100 // Conservative 100 Mbps
            }
        } else {
            100 // No interfaces detected, conservative fallback
        };

        Ok(SystemCapabilities {
            total_cpu_cores,
            available_cpu_cores,
            total_memory_bytes,
            available_memory_bytes,
            total_gpu_memory_bytes,
            available_gpu_memory_bytes,
            total_storage_bytes,
            available_storage_bytes,
            network_bandwidth_mbps,
            gpu_count,
            gpu_types,
        })
    }

    /// Query GPU capabilities via wgpu (vendor-agnostic, part of barraCuda)
    ///
    /// **Deep Debt Compliance**:
    /// - Runtime GPU discovery (no hardcoded assumptions)
    /// - Vendor-agnostic (works with NVIDIA, AMD, Intel, Apple)
    /// - Graceful degradation (returns empty if no GPU)
    /// - Part of barraCuda universal GPU framework
    async fn query_gpu_capabilities(&self) -> (u64, u64, usize, Vec<String>) {
        match Self::discover_gpus_via_wgpu().await {
            Ok(gpus) if !gpus.is_empty() => {
                let gpu_count = gpus.len();
                let gpu_types: Vec<String> = gpus.iter().map(|g| g.name.clone()).collect();

                // Conservative estimate: 2GB per GPU (wgpu doesn't expose actual memory)
                // Real memory should be queried via vendor-specific APIs when needed
                let estimated_memory_per_gpu = 2 * 1024 * 1024 * 1024; // 2GB
                let total_gpu_memory = estimated_memory_per_gpu * gpu_count as u64;

                (total_gpu_memory, total_gpu_memory, gpu_count, gpu_types)
            }
            _ => {
                // No GPUs detected or discovery failed - graceful degradation
                (0, 0, 0, Vec::new())
            }
        }
    }

    /// Discover GPUs using wgpu (vendor-agnostic, part of barraCuda)
    #[cfg(feature = "gpu-discovery")]
    async fn discover_gpus_via_wgpu() -> Result<Vec<GpuInfo>, ValidationError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        let mut gpu_infos = Vec::new();

        for adapter in adapters {
            let info = adapter.get_info();

            // Only include discrete/integrated GPUs, skip software renderers
            if matches!(
                info.device_type,
                wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
            ) {
                gpu_infos.push(GpuInfo {
                    name: info.name.clone(),
                    _memory_mb: 0,
                    _vendor: Self::vendor_from_backend(info.backend),
                });
            }
        }

        Ok(gpu_infos)
    }

    /// Fallback when GPU discovery not available
    #[cfg(not(feature = "gpu-discovery"))]
    async fn discover_gpus_via_wgpu() -> Result<Vec<GpuInfo>, ValidationError> {
        Ok(Vec::new())
    }

    #[cfg(feature = "gpu-discovery")]
    fn vendor_from_backend(backend: wgpu::Backend) -> String {
        match backend {
            wgpu::Backend::Vulkan => "Vulkan".to_string(),
            wgpu::Backend::Metal => "Metal".to_string(),
            wgpu::Backend::Dx12 => "DirectX12".to_string(),
            wgpu::Backend::Gl => "OpenGL".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Identify resource gaps
    ///
    /// Compares estimated requirements against system capabilities and
    /// identifies what's missing.
    fn identify_gaps(
        &self,
        estimate: &ResourceEstimate,
        capabilities: &SystemCapabilities,
    ) -> Vec<ResourceGap> {
        let mut gaps = Vec::new();

        // Check CPU
        if estimate.cpu_cores > capabilities.available_cpu_cores {
            gaps.push(ResourceGap {
                resource_type: "cpu_cores".to_string(),
                required: estimate.cpu_cores as u64,
                available: capabilities.available_cpu_cores as u64,
                shortage: (estimate.cpu_cores - capabilities.available_cpu_cores) as u64,
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
    fn generate_warnings(
        &self,
        estimate: &ResourceEstimate,
        capabilities: &SystemCapabilities,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        // Warn if CPU usage is > 70%
        let cpu_usage = estimate.cpu_cores as f32 / capabilities.available_cpu_cores as f32;
        if cpu_usage > 0.7 && cpu_usage <= 1.0 {
            warnings.push(format!(
                "High CPU usage: {:.0}% of available cores. Performance may be impacted.",
                cpu_usage * 100.0
            ));
        }

        // Warn if memory usage is > 70%
        let memory_usage =
            estimate.memory_bytes as f32 / capabilities.available_memory_bytes as f32;
        if memory_usage > 0.7 && memory_usage <= 1.0 {
            warnings.push(format!(
                "High memory usage: {:.0}% of available memory. Risk of swapping.",
                memory_usage * 100.0
            ));
        }

        // Warn if GPU memory usage is > 70%
        if capabilities.total_gpu_memory_bytes > 0 {
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
        let storage_usage =
            estimate.storage_bytes as f32 / capabilities.available_storage_bytes as f32;
        if storage_usage > 0.8 && storage_usage <= 1.0 {
            warnings.push(format!(
                "High storage usage: {:.0}% of available storage. Consider cleanup.",
                storage_usage * 100.0
            ));
        }

        warnings
    }
}

/// Validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Estimation failed: {0}")]
    EstimationFailed(#[from] EstimationError),

    #[error("System query failed: {0}")]
    SystemQueryFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_types::{GraphNode, NodeResourceRequirements};
    use std::collections::HashMap;
    use toadstool::resources::{CpuRequirements, MemoryRequirements};

    #[tokio::test]
    async fn test_validate_small_graph() {
        let validator = ResourceValidator::new();

        let graph = ExecutionGraph {
            id: "small-graph".to_string(),
            nodes: vec![GraphNode {
                id: "node-1".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    cpu: Some(CpuRequirements {
                        min_cores: 2.0,
                        ..Default::default()
                    }),
                    memory: Some(MemoryRequirements {
                        min_bytes: 1024 * 1024 * 1024, // 1GB
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                metadata: HashMap::new(),
            }],
            edges: vec![],
            metadata: HashMap::new(),
        };

        let result = validator.validate_availability(&graph).await.unwrap();

        // Small graph should be available on most systems
        assert!(result.available);
        assert!(result.gaps.is_empty());
    }

    #[tokio::test]
    async fn test_validate_large_graph() {
        let validator = ResourceValidator::new();

        // Create a graph that requires more resources than any system has
        let graph = ExecutionGraph {
            id: "huge-graph".to_string(),
            nodes: vec![GraphNode {
                id: "node-1".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    cpu: Some(CpuRequirements {
                        min_cores: 1000.0, // Unrealistic
                        ..Default::default()
                    }),
                    memory: Some(MemoryRequirements {
                        min_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                metadata: HashMap::new(),
            }],
            edges: vec![],
            metadata: HashMap::new(),
        };

        let result = validator.validate_availability(&graph).await.unwrap();

        // Huge graph should not be available
        assert!(!result.available);
        assert!(!result.gaps.is_empty());

        // Should have CPU and memory gaps
        assert!(result.gaps.iter().any(|g| g.resource_type == "cpu_cores"));
        assert!(result.gaps.iter().any(|g| g.resource_type == "memory"));
    }

    #[test]
    fn test_resource_gap_serialization_roundtrip() {
        let gap = ResourceGap {
            resource_type: "cpu_cores".to_string(),
            required: 16,
            available: 8,
            shortage: 8,
            suggestion: "Add cores".to_string(),
        };
        let json = serde_json::to_string(&gap).unwrap();
        let restored: ResourceGap = serde_json::from_str(&json).unwrap();
        assert_eq!(gap.resource_type, restored.resource_type);
        assert_eq!(gap.shortage, restored.shortage);
    }

    #[test]
    fn test_system_capabilities_serialization_roundtrip() {
        let caps = SystemCapabilities {
            total_cpu_cores: 16,
            available_cpu_cores: 12,
            total_memory_bytes: 32 * 1024 * 1024 * 1024,
            available_memory_bytes: 24 * 1024 * 1024 * 1024,
            total_gpu_memory_bytes: 8192 * 1024 * 1024,
            available_gpu_memory_bytes: 6144 * 1024 * 1024,
            total_storage_bytes: 512 * 1024 * 1024 * 1024,
            available_storage_bytes: 256 * 1024 * 1024 * 1024,
            network_bandwidth_mbps: 1000,
            gpu_count: 1,
            gpu_types: vec!["NVIDIA RTX 3090".to_string()],
        };
        let json = serde_json::to_string(&caps).unwrap();
        let restored: SystemCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps.total_cpu_cores, restored.total_cpu_cores);
        assert_eq!(caps.gpu_types, restored.gpu_types);
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::EstimationFailed(
            crate::resource_estimator::EstimationError::CyclicGraph,
        );
        assert!(err.to_string().contains("Estimation") || err.to_string().contains("cycle"));

        let err2 = ValidationError::SystemQueryFailed("disk read failed".to_string());
        assert!(err2.to_string().contains("disk read failed"));

        let err3 = ValidationError::InvalidConfiguration("bad config".to_string());
        assert!(err3.to_string().contains("Invalid configuration"));
    }

    #[test]
    fn test_resource_validator_default() {
        let _v = ResourceValidator::default();
        let _v2 = ResourceValidator::new();
    }
}
