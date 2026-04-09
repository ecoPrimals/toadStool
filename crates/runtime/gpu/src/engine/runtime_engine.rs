// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`toadstool::execution::RuntimeEngine`] integration and request/response mapping.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, SystemTime};

use tracing::{info, warn};
use uuid::Uuid;

use toadstool::{
    WorkloadSpec, WorkloadType,
    error::{ToadStoolError, ToadStoolResult},
    execution::{
        ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities,
        RuntimeConfig, RuntimeEngine, RuntimeType,
    },
    resources::RuntimeMetrics,
};

use crate::types::{ComputeResult, ComputeWorkload, DeviceRequirements, KernelFormat};

use super::UniversalGpuEngine;

impl RuntimeEngine for UniversalGpuEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            // Already initialized in constructor
            Ok(())
        })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            let workload = Self::convert_request_to_workload(&request)?;
            let result = self.execute_workload(workload).await?;

            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput {
                    data: result
                        .primary_output
                        .buffers
                        .values()
                        .flatten()
                        .copied()
                        .collect(),
                    stdout: Some(format!(
                        "GPU execution completed on device: {:?}",
                        result.device_id
                    )),
                    stderr: if result.primary_output.errors.is_empty() {
                        None
                    } else {
                        Some(result.primary_output.errors.join("\n"))
                    },
                    exit_code: Some(0),
                    format: Some("gpu-compute".to_string()),
                    result: HashMap::new(),
                    metadata: HashMap::new(),
                },
                metrics: self.create_runtime_metrics(&result),
                duration: result.total_execution_time,
                runtime_used: RuntimeType::Gpu,
                warnings: if result.primary_output.errors.is_empty() {
                    vec![]
                } else {
                    result.primary_output.errors
                },
            })
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        let mut platform_features = HashMap::new();
        platform_features.insert("parallel_compute".to_string(), true);
        platform_features.insert("recursive_execution".to_string(), true);
        platform_features.insert("multi_framework".to_string(), true);
        platform_features.insert("universal_kernels".to_string(), true);
        platform_features.insert("auto_optimization".to_string(), true);

        RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Gpu],
            max_concurrent_executions: Some(64),
            supported_architectures: vec![
                "x86_64".to_string(),
                "aarch64".to_string(),
                "wasm32".to_string(),
            ],
            platform_features,
            version: "1.0.0".to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Gpu)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async {
            Ok(RuntimeMetrics {
                cpu: toadstool::resources::CpuMetrics {
                    usage_percent: 0.0, // GPU doesn't use CPU metrics
                    cores_used: 0.0,
                    cpu_time_seconds: 0.0,
                },
                memory: toadstool::resources::MemoryMetrics {
                    usage_percent: 0.0,
                    used_bytes: 0,
                    peak_bytes: 0,
                },
                storage: toadstool::resources::StorageMetrics {
                    usage_percent: 0.0,
                    used_bytes: 0,
                    bytes_read: 0,
                    bytes_written: 0,
                },
                network: toadstool::resources::NetworkMetrics {
                    bytes_sent: 0,
                    bytes_received: 0,
                    packets_sent: 0,
                    packets_received: 0,
                },
                gpu: Some(toadstool::resources::GpuMetrics {
                    usage_percent: 0.0,
                    memory_usage_percent: 0.0,
                    memory_used_bytes: 0,
                    temperature_celsius: None,
                }),
                timing: toadstool::resources::TimingMetrics {
                    start_time: SystemTime::now(),
                    end_time: Some(SystemTime::now()),
                    duration: Duration::ZERO,
                },
            })
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            // Destroy all active sessions
            let session_ids: Vec<Uuid> = {
                let sessions = self.active_sessions.read().await;
                sessions.keys().copied().collect()
            };

            for session_id in session_ids {
                if let Err(e) = self.destroy_compute_session(session_id).await {
                    warn!(
                        "Failed to destroy session {} during shutdown: {}",
                        session_id, e
                    );
                }
            }

            info!("Universal GPU Engine shutdown complete");
            Ok(())
        })
    }
}

impl UniversalGpuEngine {
    /// Convert execution request to compute workload
    pub(super) fn convert_request_to_workload(
        request: &ExecutionRequest,
    ) -> ToadStoolResult<ComputeWorkload> {
        let kernel_source = match &request.workload {
            WorkloadSpec::Gpu { program, .. } => {
                #[expect(deprecated, reason = "OpenCL/CUDA arms for persisted GPU specs (S198)")]
                match program {
                    toadstool::workload::GpuProgramSource::OpenCL { source }
                    | toadstool::workload::GpuProgramSource::Cuda { source } => source.clone(),
                    toadstool::workload::GpuProgramSource::Vulkan { spirv } => {
                        // Convert SPIR-V bytes to string representation
                        format!("SPIR-V binary: {} bytes", spirv.len())
                    }
                }
            }
            _ => {
                return Err(ToadStoolError::runtime(
                    "Only GPU workloads are supported by GPU runtime",
                ));
            }
        };

        Ok(ComputeWorkload {
            name: request.execution_id.to_string(),
            kernel_source,
            kernel_format: KernelFormat::OpenClC, // Default, could be inferred from program type
            inputs: Vec::new(),                   // Would need to extract from request
            requirements: DeviceRequirements::minimal(),
            parent_session: None,
            recursive_workloads: Vec::new(),
            priority: 1,
        })
    }

    /// Create runtime metrics from compute result
    fn create_runtime_metrics(&self, result: &ComputeResult) -> RuntimeMetrics {
        RuntimeMetrics {
            cpu: toadstool::resources::CpuMetrics {
                usage_percent: 0.0,
                cores_used: 0.0,
                cpu_time_seconds: 0.0,
            },
            memory: toadstool::resources::MemoryMetrics {
                usage_percent: 0.0,
                used_bytes: result.primary_output.metrics.memory_used,
                peak_bytes: result.primary_output.metrics.memory_used,
            },
            storage: toadstool::resources::StorageMetrics {
                usage_percent: 0.0,
                used_bytes: 0,
                bytes_read: 0,
                bytes_written: 0,
            },
            network: toadstool::resources::NetworkMetrics {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
            },
            gpu: Some(toadstool::resources::GpuMetrics {
                usage_percent: 0.0,
                memory_usage_percent: 0.0,
                memory_used_bytes: result.primary_output.metrics.memory_used,
                temperature_celsius: None,
            }),
            timing: toadstool::resources::TimingMetrics {
                start_time: SystemTime::now(),
                end_time: Some(SystemTime::now()),
                duration: result.total_execution_time,
            },
        }
    }
}
