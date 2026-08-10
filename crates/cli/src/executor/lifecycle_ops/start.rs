// SPDX-License-Identifier: AGPL-3.0-or-later
//! Start operations: biome, primal, and service startup

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_WORKLOAD_TIMEOUT_SECS: u64 = 3600;

use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
use std::fs;
use tracing::info;
use uuid::Uuid;

use super::super::{BiomeExecutor, BiomeProcess, ProcessType, RunningBiome};
use crate::{BiomeInfo, BiomeManifest, BiomeStatus, ResourceUsage, ServiceInfo, WorkloadSource};
use toadstool::{
    ExecutionInput, ExecutionRequest, ResourceRequirements, RuntimeType, SecurityContext,
    WorkloadSpec,
};

/// Parse env vars from "KEY=VALUE" strings. Used by start_biome_internal.
pub(super) fn parse_env_vars(env_vars: &[String]) -> HashMap<String, String> {
    let mut environment = HashMap::with_capacity(env_vars.len());
    for env_var in env_vars {
        if let Some((key, value)) = env_var.split_once('=') {
            environment.insert(key.to_owned(), value.to_owned());
        }
    }
    environment
}

impl BiomeExecutor {
    pub(in crate::executor) async fn start_biome_internal(
        &self,
        biome_name: &str,
        manifest: BiomeManifest,
        env_vars: Vec<String>,
        _detached: bool,
        _debug: bool,
        security_level: &str,
    ) -> crate::Result<BiomeInfo> {
        let biome_id = Uuid::new_v4();
        let start_time = std::time::SystemTime::now();

        info!("🔧 Initializing biome infrastructure");

        let env = PathEnv::from_env();
        let paths = PlatformPaths::new(&env);
        let log_dir = paths.toadstool_log_dir().join(biome_name);
        fs::create_dir_all(&log_dir)?;

        let environment = parse_env_vars(&env_vars);

        let mut processes = Vec::new();
        let mut log_files = HashMap::new();

        let mut started_crypto_provider: Option<String> = None;

        if manifest.security.security_required {
            info!(
                "🔐 Security provider required - use UniversalServiceAdapter.discover(\"security\")"
            );

            if let Some((manifest_key, security_config)) =
                manifest.find_primal_with_capability("crypto")
            {
                info!("🔐 Starting crypto capability provider: {manifest_key}");
                let process = self
                    .start_primal(
                        manifest_key,
                        security_config,
                        &environment,
                        &log_dir,
                        security_level,
                    )
                    .await?;
                processes.push(process);
                log_files.insert(
                    manifest_key.to_string(),
                    log_dir.join(format!("{manifest_key}.log")),
                );
                started_crypto_provider = Some(manifest_key.to_string());
            }
        }

        for (primal_name, primal_config) in &manifest.primals {
            if started_crypto_provider.as_deref() == Some(primal_name.as_str()) {
                continue;
            }

            if primal_config.enabled {
                info!("🔧 Starting primal: {}", primal_name);
                let process = self
                    .start_primal(
                        primal_name,
                        primal_config,
                        &environment,
                        &log_dir,
                        security_level,
                    )
                    .await?;
                processes.push(process);
                log_files.insert(
                    String::from(primal_name),
                    log_dir.join(format!("{primal_name}.log")),
                );
            }
        }

        for (service_name, service_config) in &manifest.services {
            info!("🚀 Starting service: {}", service_name);
            let process = self
                .start_service(
                    service_name,
                    service_config,
                    &environment,
                    &log_dir,
                    security_level,
                )
                .await?;
            processes.push(process);
            log_files.insert(
                String::from(service_name),
                log_dir.join(format!("{service_name}.log")),
            );
        }

        let biome_info = BiomeInfo {
            id: biome_id,
            name: biome_name.to_string(),
            status: BiomeStatus::Running,
            created: start_time,
            started: Some(start_time),
            manifest_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            resource_usage: ResourceUsage {
                cpu_percent: 0.0,
                memory_bytes: 0,
                storage_bytes: 0,
                network_rx_bytes: 0,
                network_tx_bytes: 0,
            },
            services: manifest
                .services
                .keys()
                .map(|name| ServiceInfo {
                    name: name.clone(),
                    status: String::from("running"),
                    replicas: 1,
                    ports: vec![],
                    health: String::from("healthy"),
                })
                .collect(),
        };

        let running_biome = RunningBiome {
            info: biome_info.clone(),
            _manifest: manifest,
            process_handles: processes,
            log_files,
        };

        {
            let mut biomes = self.biomes.write().unwrap_or_else(|e| e.into_inner());
            biomes.insert(biome_name.to_string(), running_biome);
        }

        Ok(biome_info)
    }

    async fn start_primal(
        &self,
        name: &str,
        config: &crate::PrimalConfig,
        environment: &HashMap<String, String>,
        _log_dir: &Path,
        _security_level: &str,
    ) -> crate::Result<BiomeProcess> {
        let execution_id = Uuid::new_v4();

        let workload = self.workload_source_to_spec(&config.source).await?;

        let request = ExecutionRequest {
            execution_id,
            workload,
            runtime_hint: Some(RuntimeType::Native),
            resources: ResourceRequirements::default(),
            security_context: SecurityContext::default(),
            timeout: Some(Duration::from_secs(DEFAULT_WORKLOAD_TIMEOUT_SECS)),
            environment: environment.clone(),
            input_data: ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        };

        let _execution_id = self.distributed.submit_execution(request).await?;

        Ok(BiomeProcess {
            name: name.to_string(),
            process_type: ProcessType::Primal(name.to_string()),
            execution_id,
            pid: Some(1000 + (execution_id.as_u128() % 30_000) as u32),
            _started_at: std::time::SystemTime::now(),
        })
    }

    async fn start_service(
        &self,
        name: &str,
        config: &crate::ServiceConfig,
        environment: &HashMap<String, String>,
        _log_dir: &Path,
        _security_level: &str,
    ) -> crate::Result<BiomeProcess> {
        let execution_id = Uuid::new_v4();

        let workload = self.workload_source_to_spec(&config.source).await?;

        let mut service_env = environment.clone();
        service_env.extend(config.environment.clone());

        let request = ExecutionRequest {
            execution_id,
            workload,
            runtime_hint: Some(RuntimeType::Native),
            resources: ResourceRequirements::default(),
            security_context: SecurityContext::default(),
            timeout: Some(Duration::from_secs(DEFAULT_WORKLOAD_TIMEOUT_SECS)),
            environment: service_env,
            input_data: ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        };

        let _execution_id = self.distributed.submit_execution(request).await?;

        Ok(BiomeProcess {
            name: name.to_string(),
            process_type: ProcessType::Service(name.to_string()),
            execution_id,
            pid: Some(2000 + (execution_id.as_u128() % 30_000) as u32),
            _started_at: std::time::SystemTime::now(),
        })
    }

    async fn workload_source_to_spec(
        &self,
        source: &WorkloadSource,
    ) -> crate::Result<WorkloadSpec> {
        match source {
            WorkloadSource::Container {
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
            WorkloadSource::Wasm {
                source,
                checksum,
                wasi_config: _wasi_config,
            } => {
                let module_data = self
                    .load_wasm_with_verification(source, &Some(checksum.clone()))
                    .await?;
                Ok(WorkloadSpec::Wasm {
                    module: toadstool::workload::WasmModuleSource::Bytes {
                        data: module_data.into(),
                    },
                    args: None,
                    wasi_config: None,
                    env_vars: HashMap::new(),
                })
            }
            WorkloadSource::Local { path } => Ok(WorkloadSpec::Native {
                executable: toadstool::workload::ExecutableSource::File { path: path.clone() },
                args: None,
                working_dir: None,
                env_vars: HashMap::new(),
                user: None,
            }),
            _ => Err(crate::CliError::Other(format!(
                "Unsupported workload source: {source:?}"
            ))),
        }
    }
}
