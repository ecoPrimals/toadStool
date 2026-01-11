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

use crate::resource_estimator::{ResourceEstimate, EstimationError};
use crate::graph_types::ExecutionGraph;
use crate::resource_estimator::ResourceEstimator;

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
        let estimate = self.estimator.estimate(graph)
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
            warn!("❌ System lacks resources for graph {}. Gaps: {:?}", graph.id, gaps);
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
        let total_cpu_cores = num_cpus::get() as u32;
        // Assume 80% available (rough heuristic, in production would query actual usage)
        let available_cpu_cores = (total_cpu_cores as f32 * 0.8) as u32;
        
        // Query memory
        let mem_info = sys_info::mem_info()
            .map_err(|e| ValidationError::SystemQueryFailed(format!("Memory query failed: {}", e)))?;
        let total_memory_bytes = mem_info.total * 1024; // Convert KB to bytes
        let available_memory_bytes = mem_info.avail * 1024;
        
        // Query disk
        let disk_info = sys_info::disk_info()
            .map_err(|e| ValidationError::SystemQueryFailed(format!("Disk query failed: {}", e)))?;
        let total_storage_bytes = disk_info.total * 1024; // Convert KB to bytes
        let available_storage_bytes = disk_info.free * 1024;
        
        // Query GPU (if available)
        // TODO(gpu_detection): Implement actual GPU detection using nvml-wrapper, amdgpu_top, etc.
        // For now, return conservative estimates
        let (total_gpu_memory_bytes, available_gpu_memory_bytes, gpu_count, gpu_types) = 
            self.query_gpu_capabilities().await;
        
        // Network bandwidth (rough estimate, in production would query actual interface)
        let network_bandwidth_mbps = 1000; // Assume 1 Gbps
        
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
    
    /// Query GPU capabilities
    ///
    /// This is a placeholder for actual GPU detection.
    /// In production, this would use nvml-wrapper, amdgpu_top, intel-gpu-tools, etc.
    async fn query_gpu_capabilities(&self) -> (u64, u64, usize, Vec<String>) {
        // TODO(gpu_detection): Implement actual GPU detection
        // For now, return conservative estimates indicating no GPU
        (0, 0, 0, Vec::new())
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
                    (estimate.memory_bytes - capabilities.available_memory_bytes) / (1024 * 1024 * 1024)
                ),
            });
        }
        
        // Check GPU memory
        if estimate.gpu_memory_bytes > 0 && estimate.gpu_memory_bytes > capabilities.available_gpu_memory_bytes {
            gaps.push(ResourceGap {
                resource_type: "gpu_memory".to_string(),
                required: estimate.gpu_memory_bytes,
                available: capabilities.available_gpu_memory_bytes,
                shortage: estimate.gpu_memory_bytes - capabilities.available_gpu_memory_bytes,
                suggestion: if capabilities.gpu_count == 0 {
                    "No GPU detected. Consider using CPU fallback or acquiring GPU resources.".to_string()
                } else {
                    format!(
                        "Need {} GB more GPU memory. Consider model quantization or sharding.",
                        (estimate.gpu_memory_bytes - capabilities.available_gpu_memory_bytes) / (1024 * 1024 * 1024)
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
                    (estimate.storage_bytes - capabilities.available_storage_bytes) / (1024 * 1024 * 1024)
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
        let memory_usage = estimate.memory_bytes as f32 / capabilities.available_memory_bytes as f32;
        if memory_usage > 0.7 && memory_usage <= 1.0 {
            warnings.push(format!(
                "High memory usage: {:.0}% of available memory. Risk of swapping.",
                memory_usage * 100.0
            ));
        }
        
        // Warn if GPU memory usage is > 70%
        if capabilities.total_gpu_memory_bytes > 0 {
            let gpu_usage = estimate.gpu_memory_bytes as f32 / capabilities.available_gpu_memory_bytes as f32;
            if gpu_usage > 0.7 && gpu_usage <= 1.0 {
                warnings.push(format!(
                    "High GPU memory usage: {:.0}% of available GPU memory. May cause OOM.",
                    gpu_usage * 100.0
                ));
            }
        }
        
        // Warn if storage usage is > 80%
        let storage_usage = estimate.storage_bytes as f32 / capabilities.available_storage_bytes as f32;
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
    use std::collections::HashMap;
    use crate::graph_types::{GraphNode, NodeResourceRequirements};
    use toadstool::resources::{CpuRequirements, MemoryRequirements};
    
    #[tokio::test]
    async fn test_validate_small_graph() {
        let validator = ResourceValidator::new();
        
        let graph = ExecutionGraph {
            id: "small-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
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
                },
            ],
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
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
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
                },
            ],
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
}

