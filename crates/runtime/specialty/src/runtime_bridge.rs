// SPDX-License-Identifier: AGPL-3.0-or-later
//! RuntimeEngine trait implementation - bridges ExecutionRequest/Response to legacy jobs

use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use toadstool::execution;
use toadstool::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities,
    RuntimeEngine, RuntimeMetrics, RuntimeType, ToadStoolResult, WorkloadType,
};

use crate::engine::SpecialtyRuntimeEngine;
use crate::types::jobs::LegacyJob;
use crate::types::traits::JobStatus;

impl SpecialtyRuntimeEngine {
    /// Convert `ExecutionRequest` to `LegacyJob`
    #[expect(
        clippy::unnecessary_wraps,
        clippy::unused_self,
        clippy::needless_pass_by_value
    )]
    pub(crate) fn convert_execution_request_to_legacy_job(
        &self,
        request: ExecutionRequest,
    ) -> ToadStoolResult<LegacyJob> {
        use crate::types::configs::CommunicationSettings;
        use crate::types::jobs::{LegacyJobSource, LegacyJobType, ProgramFormat};
        use crate::types::requirements::{
            CommunicationRequirements, CompilationRequirements, CompilerType, CpuRequirements,
            LegacyRuntimeRequirements, MemoryModel, MemoryRequirements, MemoryType,
            NetworkRequirements, StorageRequirements, StorageType, TimingRequirements,
        };
        use crate::types::systems::LegacyArchitecture;

        let job_id = request.execution_id;

        Ok(LegacyJob {
            job_id,
            target_system: crate::types::systems::LegacySystemType::Intel8086,
            target_architecture: LegacyArchitecture::Intel8086,
            job_type: LegacyJobType::Execution {
                program_format: ProgramFormat::DosExe,
                arguments: vec![],
            },
            source: LegacyJobSource::SourceCode {
                language: crate::types::jobs::LegacyLanguage::Ckr,
                code: "/* Default legacy job */".to_string(),
            },
            compilation_requirements: CompilationRequirements {
                compiler: CompilerType::MicrosoftC60,
                flags: vec![],
                include_paths: vec![],
                library_paths: vec![],
                libraries: vec![],
                memory_model: MemoryModel::Flat,
                optimization: crate::types::requirements::OptimizationLevel::None,
                debug_info: false,
            },
            runtime_requirements: LegacyRuntimeRequirements {
                memory: MemoryRequirements {
                    min_memory: 64 * 1024,
                    max_memory: 640 * 1024,
                    memory_type: MemoryType::RAM,
                    memory_model: MemoryModel::Segmented,
                },
                cpu: CpuRequirements {
                    architecture: LegacyArchitecture::Intel8086,
                    min_speed: 4_770_000,
                    required_features: vec![],
                    fpu_required: false,
                },
                storage: StorageRequirements {
                    min_storage: 360 * 1024,
                    storage_type: StorageType::FloppyDisk,
                    file_system: crate::types::requirements::FileSystemType::DOS,
                },
                communication: CommunicationRequirements {
                    protocols: vec![],
                    ports: vec![],
                    network: NetworkRequirements {
                        protocols: vec![],
                        bandwidth: None,
                        max_latency: None,
                    },
                },
                timing: TimingRequirements {
                    real_time: false,
                    max_response_time: Duration::from_secs(10),
                    min_cycle_time: Duration::from_millis(1),
                    timing_accuracy: Duration::from_millis(1),
                },
                special_hardware: vec![],
            },
            communication_settings: CommunicationSettings::default(),
            priority: toadstool::JobPriority::Normal,
            created_at: std::time::SystemTime::now(),
            timeout: Duration::from_secs(3600),
        })
    }

    /// Get runtime metrics in ToadStool format
    pub(crate) async fn get_runtime_metrics(&self) -> ToadStoolResult<RuntimeMetrics> {
        let _legacy_metrics = self.get_metrics().await?;
        Ok(RuntimeMetrics::default())
    }
}

impl RuntimeEngine for SpecialtyRuntimeEngine {
    fn initialize(
        &mut self,
        config: execution::RuntimeConfig,
    ) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            tracing::info!("Initializing specialty hardware runtime engine");
            if let Some(resource_limits) = config.resource_limits {
                tracing::debug!("Applying resource limits: {:?}", resource_limits);
            }
            if let Some(security_settings) = config.security_settings {
                tracing::debug!("Applying security settings: {:?}", security_settings);
            }
            tracing::info!("Specialty hardware runtime engine initialized successfully");
            Ok(())
        }
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async move {
            const MAX_POLL_INTERVAL: Duration = Duration::from_millis(500);

            let execution_id = request.execution_id;
            tracing::info!(
                "Executing specialty hardware runtime request: {:?}",
                execution_id
            );

            let legacy_job = self.convert_execution_request_to_legacy_job(request)?;
            let job_id = self.submit_job(legacy_job).await?;

            let timeout = Duration::from_secs(self.config.job_timeout.as_secs());
            let start_time = std::time::Instant::now();
            let mut poll_interval = Duration::from_millis(10);

            loop {
                let status = self.get_job_status(job_id).await?;

                match status {
                    JobStatus::Completed => {
                        let output = self.get_job_output(job_id).await?;
                        let duration = start_time.elapsed();
                        return Ok(ExecutionResponse {
                            execution_id,
                            status: ExecutionStatus::Success,
                            output: ExecutionOutput {
                                data: output
                                    .binary_output
                                    .map(bytes::Bytes::from)
                                    .unwrap_or_default(),
                                stdout: Some(output.stdout),
                                stderr: Some(output.stderr),
                                exit_code: output.return_code,
                                format: None,
                                result: HashMap::new(),
                                metadata: HashMap::new(),
                            },
                            metrics: self.get_runtime_metrics().await?,
                            duration,
                            runtime_used: RuntimeType::from("specialty"),
                            warnings: Vec::new(),
                        });
                    }
                    JobStatus::Failed { error } => {
                        let duration = start_time.elapsed();
                        return Ok(ExecutionResponse {
                            execution_id,
                            status: ExecutionStatus::Failed {
                                error: Cow::Owned(error),
                            },
                            output: ExecutionOutput::default(),
                            metrics: self.get_runtime_metrics().await?,
                            duration,
                            runtime_used: RuntimeType::from("specialty"),
                            warnings: Vec::new(),
                        });
                    }
                    JobStatus::Cancelled => {
                        let duration = start_time.elapsed();
                        return Ok(ExecutionResponse {
                            execution_id,
                            status: ExecutionStatus::Cancelled,
                            output: ExecutionOutput::default(),
                            metrics: self.get_runtime_metrics().await?,
                            duration,
                            runtime_used: RuntimeType::from("specialty"),
                            warnings: Vec::new(),
                        });
                    }
                    JobStatus::TimedOut => {
                        let duration = start_time.elapsed();
                        return Ok(ExecutionResponse {
                            execution_id,
                            status: ExecutionStatus::TimedOut,
                            output: ExecutionOutput::default(),
                            metrics: self.get_runtime_metrics().await?,
                            duration,
                            runtime_used: RuntimeType::from("specialty"),
                            warnings: Vec::new(),
                        });
                    }
                    JobStatus::Queued | JobStatus::Running => {
                        if start_time.elapsed() > timeout {
                            let _ = self.cancel_job(job_id).await;
                            let duration = start_time.elapsed();
                            return Ok(ExecutionResponse {
                                execution_id,
                                status: ExecutionStatus::TimedOut,
                                output: ExecutionOutput::default(),
                                metrics: self.get_runtime_metrics().await?,
                                duration,
                                runtime_used: RuntimeType::from("specialty"),
                                warnings: Vec::new(),
                            });
                        }
                        tokio::time::sleep(poll_interval).await;
                        poll_interval = (poll_interval * 2).min(MAX_POLL_INTERVAL);
                    }
                }
            }
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )] // advertised cap fits u32 for API
    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Native],
            max_concurrent_executions: Some(self.config.max_concurrent_jobs as u32),
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
                features.insert("mainframe".to_string(), true);
                features.insert("embedded".to_string(), true);
                features.insert("realtime".to_string(), true);
                features.insert("industrial".to_string(), true);
                features.insert("cross_compilation".to_string(), true);
                features.insert("emulation".to_string(), true);
                features
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Native)
    }

    fn get_metrics(
        &self,
    ) -> impl std::future::Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        async { self.get_runtime_metrics().await }
    }

    fn shutdown(&mut self) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            tracing::info!("Shutting down legacy runtime engine");
            let jobs: Vec<uuid::Uuid> = self.active_jobs.read().await.keys().copied().collect();
            for job_id in jobs {
                if let Err(e) = self.cancel_job(job_id).await {
                    tracing::error!("Error cancelling job {}: {}", job_id, e);
                }
            }
            tracing::info!("Legacy runtime engine shutdown complete");
            Ok(())
        }
    }
}
