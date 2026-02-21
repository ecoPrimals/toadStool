//! Biome process spawning and management
//!
//! This module handles spawning and tracking processes for primals and services
//! within biomes, including workload conversion and distributed execution.

use anyhow::{bail, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use toadstool::{
    ExecutionInput, ExecutionRequest, ResourceRequirements, RuntimeType, SecurityContext,
    WorkloadSpec,
};
use toadstool_distributed::DistributedCoordinator;
use tokio::time::Duration;
use uuid::Uuid;

use super::{BiomeExecutor, BiomeProcess, ProcessType};

/// Process spawner for biome processes
#[allow(dead_code)]
pub(super) struct ProcessSpawner<'a> {
    executor: &'a BiomeExecutor,
}

#[allow(dead_code)]
impl<'a> ProcessSpawner<'a> {
    /// Create new process spawner
    pub fn new(executor: &'a BiomeExecutor) -> Self {
        Self { executor }
    }

    /// Start a primal process
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Workload conversion fails
    /// - Distributed execution submission fails
    pub async fn start_primal(
        &self,
        name: &str,
        config: &crate::PrimalConfig,
        environment: &HashMap<String, String>,
        _log_dir: &Path,
        _security_level: &str,
    ) -> Result<BiomeProcess> {
        let execution_id = Uuid::new_v4();

        // Convert primal config to execution request
        let workload = self.workload_source_to_spec(&config.source).await?;

        let request = ExecutionRequest {
            execution_id,
            workload,
            runtime_hint: Some(RuntimeType::Native),
            resources: ResourceRequirements::default(),
            security_context: SecurityContext::default(),
            timeout: Some(Duration::from_secs(3600)), // 1 hour default
            environment: environment.clone(),
            input_data: ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        };

        // Submit to distributed coordinator
        let _execution_id = self.executor.distributed.submit_execution(request).await?;

        Ok(BiomeProcess {
            name: name.to_string(),
            process_type: ProcessType::Primal(name.to_string()),
            execution_id,
            pid: Some(1000 + (execution_id.as_u128() % 30000) as u32),
            _started_at: Utc::now(),
        })
    }

    /// Start a service process
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Workload conversion fails
    /// - Distributed execution submission fails
    pub async fn start_service(
        &self,
        name: &str,
        config: &crate::ServiceConfig,
        environment: &HashMap<String, String>,
        _log_dir: &Path,
        _security_level: &str,
    ) -> Result<BiomeProcess> {
        let execution_id = Uuid::new_v4();

        // Convert service config to execution request
        let workload = self.workload_source_to_spec(&config.source).await?;

        let mut service_env = environment.clone();
        service_env.extend(config.environment.clone());

        let request = ExecutionRequest {
            execution_id,
            workload,
            runtime_hint: Some(RuntimeType::Native),
            resources: ResourceRequirements::default(),
            security_context: SecurityContext::default(),
            timeout: Some(Duration::from_secs(3600)), // 1 hour default
            environment: service_env,
            input_data: ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        };

        // Submit to distributed coordinator
        let _execution_id = self.executor.distributed.submit_execution(request).await?;

        Ok(BiomeProcess {
            name: name.to_string(),
            process_type: ProcessType::Service(name.to_string()),
            execution_id,
            pid: Some(2000 + (execution_id.as_u128() % 30000) as u32),
            _started_at: Utc::now(),
        })
    }

    /// Convert workload source to workload spec
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - WASM verification fails
    /// - Workload source is unsupported
    async fn workload_source_to_spec(
        &self,
        source: &crate::WorkloadSource,
    ) -> Result<WorkloadSpec> {
        match source {
            crate::WorkloadSource::Container {
                registry,
                image,
                tag,
                ..
            } => Ok(WorkloadSpec::Container {
                image: format!("{registry}/{image}:{tag}"),
                command: None,
                args: None,
                working_dir: None,
                env_vars: HashMap::new(),
                volumes: Vec::new(),
                ports: Vec::new(),
                registry_auth: None,
            }),
            crate::WorkloadSource::Wasm {
                source: _wasm_source,
                checksum,
                wasi_config: _wasi_config,
            } => {
                // Load WASM module from source with verification
                let module_data = self
                    .load_wasm_with_verification(_wasm_source, &Some(checksum.clone()))
                    .await?;
                Ok(WorkloadSpec::Wasm {
                    module: toadstool::workload::WasmModuleSource::Bytes {
                        data: module_data.into(),
                    },
                    args: None,
                    wasi_config: None, // WASI config conversion not implemented
                    env_vars: HashMap::new(),
                })
            }
            crate::WorkloadSource::Local { path } => Ok(WorkloadSpec::Native {
                executable: toadstool::workload::ExecutableSource::File { path: path.clone() },
                args: None,
                working_dir: None,
                env_vars: HashMap::new(),
                user: None,
            }),
            _ => {
                bail!("Unsupported workload source: {source:?}");
            }
        }
    }

    /// Load WASM module with verification
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Module loading fails
    /// - Checksum verification fails
    async fn load_wasm_with_verification(
        &self,
        source: &str,
        checksum: &Option<String>,
    ) -> Result<Vec<u8>> {
        self.executor
            .load_wasm_with_verification(source, checksum)
            .await
    }

    /// Get the distributed coordinator
    pub(super) fn distributed(&self) -> &DistributedCoordinator {
        &self.executor.distributed
    }
}
