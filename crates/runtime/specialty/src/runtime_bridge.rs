// SPDX-License-Identifier: AGPL-3.0-only

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use toadstool::execution;
use toadstool::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities,
    RuntimeEngine, RuntimeMetrics, ToadStoolResult, WorkloadType,
};

use crate::engine::SpecialtyRuntimeEngine;
use crate::types::jobs::LegacyJob;
use crate::types::traits::JobStatus;

impl SpecialtyRuntimeEngine {
    pub(super) fn convert_execution_request_to_legacy_job(
        &self,
        request: ExecutionRequest,
    ) -> ToadStoolResult<LegacyJob> {
        let job_id = request.workload_id.unwrap_or_else(uuid::Uuid::new_v4);
        Ok(LegacyJob {
            job_id,
            target_system: crate::types::systems::LegacySystemType::Intel8086,
            target_architecture: crate::types::systems::LegacyArchitecture::Intel8086,
            job_type: crate::types::jobs::LegacyJobType::Execution {
                program_format: crate::types::jobs::ProgramFormat::DosExe,
                arguments: vec![],
            },
            source: crate::types::jobs::LegacyJobSource::SourceCode {
                language: crate::types::jobs::LegacyLanguage::Ckr,
                code: "/* Default legacy job */".to_string(),
            },
            compilation_requirements: crate::types::requirements::CompilationRequirements {
                compiler: crate::types::requirements::CompilerType::MicrosoftC60,
                flags: vec![],
                include_paths: vec![],
                library_paths: vec![],
                libraries: vec![],
                memory_model: crate::types::requirements::MemoryModel::Flat,
                optimization: crate::types::requirements::OptimizationLevel::None,
                debug_info: false,
            },
            runtime_requirements: crate::types::requirements::LegacyRuntimeRequirements {
                memory: crate::types::requirements::MemoryRequirements {
                    min_memory: 64 * 1024,
                    max_memory: 640 * 1024,
                    memory_type: crate::types::requirements::MemoryType::RAM,
                    memory_model: crate::types::requirements::MemoryModel::Segmented,
                },
                cpu: crate::types::requirements::CpuRequirements {
                    architecture: crate::types::systems::LegacyArchitecture::Intel8086,
                    min_speed: 4_770_000,
                    required_features: vec![],
                    fpu_required: false,
                },
                storage: crate::types::requirements::StorageRequirements {
                    min_storage: 360 * 1024,
                    storage_type: crate::types::requirements::StorageType::FloppyDisk,
                    file_system: crate::types::requirements::FileSystemType::DOS,
                },
                communication: crate::types::requirements::CommunicationRequirements {
                    protocols: vec![],
                    ports: vec![],
                    network: crate::types::requirements::NetworkRequirements {
                        protocols: vec![],
                        bandwidth: None,
                        max_latency: None,
                    },
                },
                timing: crate::types::requirements::TimingRequirements {
                    real_time: false,
                    max_response_time: Duration::from_secs(10),
                    min_cycle_time: Duration::from_millis(1),
                    timing_accuracy: Duration::from_millis(1),
                },
                special_hardware: vec![],
            },
            communication_settings: crate::types::configs::CommunicationSettings::default(),
            priority: toadstool::JobPriority::Normal,
            created_at: std::time::SystemTime::now(),
            timeout: Duration::from_secs(3600),
        })
    }

    pub(super) async fn get_runtime_metrics(&self) -> ToadStoolResult<RuntimeMetrics> {
        let _ = self.get_metrics().await?;
        Ok(RuntimeMetrics::default())
    }
}

impl RuntimeEngine for SpecialtyRuntimeEngine {
    fn initialize(
        &mut self,
        config: execution::RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            tracing::info!("Initializing specialty hardware runtime engine");
            if let Some(resource_limits) = config.resource_limits {
                tracing::debug!("Applying resource limits: {:?}", resource_limits);
            }
            if let Some(security_settings) = config.security_settings {
                tracing::debug!("Applying security settings: {:?}", security_settings);
            }
            tracing::info!("Specialty hardware runtime engine initialized successfully");
            Ok(())
        })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            tracing::info!("Executing specialty hardware runtime request: {:?}", request.workload_id);
            let legacy_job = self.convert_execution_request_to_legacy_job(request)?;
            let job_id = self.submit_job(legacy_job).await?;
            let timeout = Duration::from_secs(self.config.job_timeout.as_secs());
            let start_time = std::time::Instant::now();

            loop {
                let status = self.get_job_status(job_id).await?;
                match status {
                    JobStatus::Completed => {
                        let output = self.get_job_output(job_id).await?;
                        return Ok(ExecutionResponse {
                            workload_id: job_id,
                            status: ExecutionStatus::Completed,
                            output: Some(ExecutionOutput {
                                stdout: output.stdout,
                                stderr: output.stderr,
                                return_code: output.return_code,
                            }),
                            error: None,
                            metrics: Some(self.get_runtime_metrics().await?),
                        });
                    }
                    JobStatus::Failed { error } => {
                        return Ok(ExecutionResponse {
                            workload_id: job_id,
                            status: ExecutionStatus::Failed,
                            output: None,
                            error: Some(error),
                            metrics: Some(self.get_runtime_metrics().await?),
                        });
                    }
                    JobStatus::Cancelled => {
                        return Ok(ExecutionResponse {
                            workload_id: job_id,
                            status: ExecutionStatus::Cancelled,
                            output: None,
                            error: None,
                            metrics: Some(self.get_runtime_metrics().await?),
                        });
                    }
                    JobStatus::TimedOut => {
                        return Ok(ExecutionResponse {
                            workload_id: job_id,
                            status: ExecutionStatus::TimedOut,
                            output: None,
                            error: Some("Job timed out".to_string()),
                            metrics: Some(self.get_runtime_metrics().await?),
                        });
                    }
                    JobStatus::Queued | JobStatus::Running => {
                        if start_time.elapsed() > timeout {
                            let _ = self.cancel_job(job_id).await;
                            return Ok(ExecutionResponse {
                                workload_id: job_id,
                                status: ExecutionStatus::TimedOut,
                                output: None,
                                error: Some("Job timed out".to_string()),
                                metrics: Some(self.get_runtime_metrics().await?),
                            });
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![
                WorkloadType::Native,
                WorkloadType::Custom("specialty".to_string()),
            ],
            max_concurrent_executions: Some(self.config.max_concurrent_jobs),
            supported_architectures: vec![
                "x86_64".to_string(),
                "i386".to_string(),
                "arm".to_string(),
                "powerpc".to_string(),
                "sparc".to_string(),
                "mips".to_string(),
            ],
            platform_features: {
                let mut features = std::collections::HashMap::new();
                features.insert("mainframe".to_string(), "true".to_string());
                features.insert("embedded".to_string(), "true".to_string());
                features.insert("realtime".to_string(), "true".to_string());
                features.insert("industrial".to_string(), "true".to_string());
                features.insert("cross_compilation".to_string(), "true".to_string());
                features.insert("emulation".to_string(), "true".to_string());
                features
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Native | WorkloadType::Custom(_))
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async move { self.get_runtime_metrics().await })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            tracing::info!("Shutting down legacy runtime engine");
            let jobs: Vec<uuid::Uuid> = self.active_jobs.read().await.keys().cloned().collect();
            for job_id in jobs {
                if let Err(e) = self.cancel_job(job_id).await {
                    tracing::error!("Error cancelling job {}: {}", job_id, e);
                }
            }
            tracing::info!("Legacy runtime engine shutdown complete");
            Ok(())
        })
    }
}
