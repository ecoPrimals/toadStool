//! CUDA compute resource and context

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use cudarc::driver::safe::CudaContext;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

use crate::universal::*;

use super::ptx;
use super::{CudaBackend, DeviceInfo};

/// CUDA compute resource implementation
pub struct CudaComputeResource {
    pub(crate) backend: Arc<CudaBackend>,
    resource_id: String,
    capabilities: ComputeCapabilities,
}

impl CudaComputeResource {
    /// Create new CUDA compute resource
    pub fn new() -> ToadStoolResult<Self> {
        let backend = CudaBackend::new()?;
        let resource_id = format!(
            "cuda-{}",
            backend.device_info.name.replace(' ', "-").to_lowercase()
        );
        let capabilities = backend.capabilities();

        Ok(Self {
            backend: Arc::new(backend),
            resource_id,
            capabilities,
        })
    }

    /// Create with custom device selector
    pub fn with_selector<F>(selector: F) -> ToadStoolResult<Self>
    where
        F: FnOnce(Vec<(Arc<CudaContext>, DeviceInfo)>) -> Option<(Arc<CudaContext>, DeviceInfo)>,
    {
        let backend = CudaBackend::with_device_selector(selector)?;
        let resource_id = format!(
            "cuda-{}",
            backend.device_info.name.replace(' ', "-").to_lowercase()
        );
        let capabilities = backend.capabilities();

        Ok(Self {
            backend: Arc::new(backend),
            resource_id,
            capabilities,
        })
    }

    async fn query_gpu_utilization(&self) -> Option<f32> {
        let ordinal = self.backend.device_info.ordinal;

        if let Ok(output) = tokio::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=utilization.gpu",
                "--format=csv,noheader,nounits",
                &format!("--id={}", ordinal),
            ])
            .output()
            .await
        {
            if output.status.success() {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    if let Ok(util) = stdout.trim().parse::<f32>() {
                        return Some(util / 100.0);
                    }
                }
            }
        }

        None
    }

    fn estimate_time_from_requirements(
        &self,
        requirements: &ComputeRequirements,
    ) -> std::time::Duration {
        let estimated_flops = requirements.estimated_operations.unwrap_or(1_000_000) as f64;
        let peak_flops = self.capabilities.performance.peak_flops;
        let sustained_percent =
            self.capabilities.performance.sustained_performance_percent as f64 / 100.0;
        let effective_flops = peak_flops * sustained_percent;

        let compute_seconds = estimated_flops / effective_flops;

        let data_bytes = requirements.memory_bytes as f64;
        let bandwidth = self.capabilities.memory.bandwidth_bytes_per_sec as f64;
        let transfer_seconds = (data_bytes * 2.0) / bandwidth;

        let launch_overhead_seconds =
            self.capabilities.performance.startup_latency_us as f64 / 1_000_000.0;

        let total_seconds = (compute_seconds + transfer_seconds + launch_overhead_seconds) * 1.15;

        std::time::Duration::from_secs_f64(total_seconds.max(0.001))
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl UniversalComputeResource for CudaComputeResource {
    fn capabilities(&self) -> &ComputeCapabilities {
        &self.capabilities
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    async fn create_context(&self) -> ToadStoolResult<Box<dyn ComputeContext>> {
        Ok(Box::new(CudaComputeContext {
            backend: Arc::clone(&self.backend),
            context_id: Uuid::new_v4(),
            resource_id: self.resource_id.clone(),
        }))
    }

    async fn utilization(&self) -> f32 {
        self.query_gpu_utilization().await.unwrap_or(0.0)
    }

    fn estimate_execution_time(&self, requirements: &ComputeRequirements) -> std::time::Duration {
        self.estimate_time_from_requirements(requirements)
    }
}

/// CUDA compute context
#[allow(dead_code)]
pub struct CudaComputeContext {
    pub(crate) backend: Arc<CudaBackend>,
    context_id: Uuid,
    resource_id: String,
}

#[async_trait]
impl ComputeContext for CudaComputeContext {
    fn context_id(&self) -> Uuid {
        self.context_id
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    async fn close(self: Box<Self>) -> ToadStoolResult<()> {
        tracing::debug!("Closing CUDA context {}", self.context_id);
        Ok(())
    }

    async fn execute(&mut self, workload: &UniversalWorkload) -> ToadStoolResult<WorkloadResult> {
        let start = std::time::Instant::now();
        tracing::info!("🚀 Executing workload {} on CUDA GPU", workload.id);

        match &workload.kernel {
            UniversalKernel::Source {
                language,
                code,
                entry_point,
            } => {
                if *language != KernelLanguage::Cuda {
                    return Err(ToadStoolError::runtime(format!(
                        "CUDA backend only supports CUDA kernels, got {:?}",
                        language
                    )));
                }

                let module_name = format!("workload_{}", workload.id);
                let _module = self.backend.load_ptx(code, &module_name).await?;

                let input_vecs: Vec<Vec<f32>> = workload
                    .inputs
                    .iter()
                    .map(|buf| {
                        buf.data
                            .chunks_exact(4)
                            .map(|chunk| {
                                f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                            })
                            .collect()
                    })
                    .collect();

                let output_elements = workload.output_size / 4;
                let block_size = 256u32;
                let grid_size = ((output_elements as u32 + block_size - 1) / block_size).max(1);

                let input_refs: Vec<&[f32]> = input_vecs.iter().map(|v| v.as_slice()).collect();
                let output = self
                    .backend
                    .execute_kernel::<f32>(
                        &module_name,
                        entry_point,
                        &input_refs,
                        output_elements,
                        (grid_size, 1, 1),
                        (block_size, 1, 1),
                    )
                    .await?;

                let output_bytes: Vec<u8> = output.iter().flat_map(|f| f.to_le_bytes()).collect();

                Ok(WorkloadResult {
                    output: output_bytes,
                    execution_time: start.elapsed(),
                    resource_used: self.resource_id.clone(),
                    metrics: HashMap::new(),
                })
            }

            UniversalKernel::Operation {
                operation,
                parameters,
            } => match operation {
                Operation::MatrixMultiply => self.execute_matmul(workload, parameters, start).await,
                Operation::Reduction => self.execute_reduction(workload, parameters, start).await,
                Operation::GeneralCompute => Err(ToadStoolError::runtime(
                    "GeneralCompute operation requires explicit CUDA kernel source",
                )),
                _ => Err(ToadStoolError::runtime(format!(
                    "Operation {:?} not yet implemented for CUDA. Use WebGPU backend.",
                    operation
                ))),
            },

            UniversalKernel::Binary { format, .. } => Err(ToadStoolError::runtime(format!(
                "Binary format {:?} not supported for CUDA. Use PTX source.",
                format
            ))),

            UniversalKernel::Library { name, version } => Err(ToadStoolError::runtime(format!(
                "Library '{}' version '{}' not available in CUDA backend",
                name, version
            ))),
        }
    }
}

impl CudaComputeContext {
    async fn execute_matmul(
        &self,
        workload: &UniversalWorkload,
        _parameters: &HashMap<String, serde_json::Value>,
        start: std::time::Instant,
    ) -> ToadStoolResult<WorkloadResult> {
        if workload.inputs.len() < 2 {
            return Err(ToadStoolError::runtime(
                "MatrixMultiply requires at least 2 input buffers",
            ));
        }

        let module_name = format!("matmul_{}", workload.id);
        let _module = self
            .backend
            .load_ptx(ptx::MATMUL_SIMPLE, &module_name)
            .await?;

        let a_data: Vec<f32> = workload.inputs[0]
            .data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let b_data: Vec<f32> = workload.inputs[1]
            .data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let n = a_data.len().min(b_data.len());
        let block_size = 256u32;
        let grid_size = ((n as u32 + block_size - 1) / block_size).max(1);

        let output = self
            .backend
            .execute_kernel::<f32>(
                &module_name,
                "matmul_simple",
                &[&a_data, &b_data],
                n,
                (grid_size, 1, 1),
                (block_size, 1, 1),
            )
            .await?;

        let output_bytes: Vec<u8> = output.iter().flat_map(|f| f.to_le_bytes()).collect();

        Ok(WorkloadResult {
            output: output_bytes,
            execution_time: start.elapsed(),
            resource_used: self.resource_id.clone(),
            metrics: HashMap::new(),
        })
    }

    async fn execute_reduction(
        &self,
        workload: &UniversalWorkload,
        _parameters: &HashMap<String, serde_json::Value>,
        start: std::time::Instant,
    ) -> ToadStoolResult<WorkloadResult> {
        if workload.inputs.is_empty() {
            return Err(ToadStoolError::runtime(
                "Reduction requires at least 1 input buffer",
            ));
        }

        let module_name = format!("reduce_{}", workload.id);
        let _module = self.backend.load_ptx(ptx::REDUCE_SUM, &module_name).await?;

        let input_data: Vec<f32> = workload.inputs[0]
            .data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let n = input_data.len();
        let block_size = 256u32;
        let grid_size = ((n as u32 + block_size - 1) / block_size).max(1);
        let output_size = grid_size as usize;

        let partial_sums = self
            .backend
            .execute_kernel::<f32>(
                &module_name,
                "reduce_sum",
                &[&input_data],
                output_size,
                (grid_size, 1, 1),
                (block_size, 1, 1),
            )
            .await?;

        let final_sum: f32 = partial_sums.iter().sum();
        let output_bytes = final_sum.to_le_bytes().to_vec();

        Ok(WorkloadResult {
            output: output_bytes,
            execution_time: start.elapsed(),
            resource_used: self.resource_id.clone(),
            metrics: HashMap::new(),
        })
    }
}
