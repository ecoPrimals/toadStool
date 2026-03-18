// SPDX-License-Identifier: AGPL-3.0-or-later
//! OpenCL Compute Context - workload execution
//!
//! Implements ComputeContext for OpenCL, dispatching UniversalWorkload to
//! built-in or custom OpenCL kernels.

use super::backend::OpenClBackend;
use super::kernels::{calculate_work_size, get_builtin_kernel};
use crate::universal::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

/// OpenCL compute context
pub struct OpenClComputeContext {
    pub(crate) backend: Arc<OpenClBackend>,
    pub(crate) context_id: Uuid,
    pub(crate) resource_id: String,
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl ComputeContext for OpenClComputeContext {
    fn context_id(&self) -> Uuid {
        self.context_id
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    async fn close(self: Box<Self>) -> ToadStoolResult<()> {
        // Cleanup happens automatically via Drop
        tracing::debug!("Closing OpenCL context {}", self.context_id);
        Ok(())
    }

    async fn execute(&mut self, workload: &UniversalWorkload) -> ToadStoolResult<WorkloadResult> {
        tracing::info!(
            "🚀 Executing workload {} on OpenCL GPU (REAL EXECUTION)",
            workload.id
        );

        let start_time = std::time::Instant::now();

        match &workload.kernel {
            UniversalKernel::Operation { operation, .. } => {
                // Get appropriate kernel source for operation
                let (kernel_source, kernel_name) = get_builtin_kernel(operation)?;

                // Compile program
                let program = self.backend.compile_program(kernel_source).await?;

                // Calculate work size based on data size
                let total_elements = workload
                    .inputs
                    .first()
                    .map(|input| input.data.len())
                    .unwrap_or(1024);
                let work_size = calculate_work_size(total_elements);

                // Execute (no extra args for general compute/matrix multiply)
                let extra_args = if matches!(operation, Operation::Reduction) {
                    // Reduction kernel needs the 'n' parameter
                    vec![
                        workload
                            .inputs
                            .first()
                            .map(|i| i.data.len() as i32)
                            .unwrap_or(0),
                    ]
                } else {
                    vec![]
                };

                let output_data = self
                    .backend
                    .execute_kernel(
                        &program,
                        kernel_name,
                        &workload.inputs,
                        workload.output_size,
                        work_size,
                        extra_args,
                    )
                    .await?;

                let execution_time = start_time.elapsed();

                Ok(WorkloadResult {
                    outputs: HashMap::from([(
                        "output_0".to_string(),
                        bytes::Bytes::from(output_data),
                    )]),
                    metrics: crate::universal::ExecutionMetrics {
                        execution_time,
                        memory_used: workload.output_size as u64,
                        energy_joules: Some(execution_time.as_secs_f64() * 15.0),
                        utilization: 0.85,
                    },
                    messages: vec![],
                })
            }
            UniversalKernel::Source {
                code,
                language,
                entry_point,
            } => {
                // Direct kernel source execution
                let kernel_name = entry_point.as_str();

                // Check language is OpenCL
                if !matches!(language, KernelLanguage::OpenClC) {
                    return Err(ToadStoolError::runtime(format!(
                        "Unsupported kernel language: {:?}. OpenCL backend requires OpenClC",
                        language
                    )));
                }

                let program = self.backend.compile_program(code).await?;

                let total_elements = workload
                    .inputs
                    .first()
                    .map(|input| input.data.len())
                    .unwrap_or(1024);
                let work_size = calculate_work_size(total_elements);

                // For custom kernels, user should handle extra args via parameters
                // For now, assume no extra args
                let output_data = self
                    .backend
                    .execute_kernel(
                        &program,
                        kernel_name,
                        &workload.inputs,
                        workload.output_size,
                        work_size,
                        vec![],
                    )
                    .await?;

                let execution_time = start_time.elapsed();

                Ok(WorkloadResult {
                    outputs: HashMap::from([(
                        "output_0".to_string(),
                        bytes::Bytes::from(output_data),
                    )]),
                    metrics: crate::universal::ExecutionMetrics {
                        execution_time,
                        memory_used: workload.output_size as u64,
                        energy_joules: Some(execution_time.as_secs_f64() * 15.0),
                        utilization: 0.85,
                    },
                    messages: vec![],
                })
            }
            _ => Err(ToadStoolError::runtime(
                "Unsupported kernel type for OpenCL",
            )),
        }
    }
}
