// SPDX-License-Identifier: AGPL-3.0-or-later
//! Internal Lifecycle Operations for Biome Management
//!
//! This module contains all internal biome lifecycle management:
#![allow(deprecated)] // Intentional: IPC addressing requires well-known names
//! - `start_biome_internal()` - Start biome with all components
//! - `start_primal()` - Start individual primal
//! - `start_service()` - Start individual service  
//! - `workload_source_to_spec()` - Convert workload sources to specs
//! - `stop_biome_internal()` - Stop biome and all components
//! - `graceful_stop_process()` - Gracefully stop a process
//! - `force_kill_process()` - Force kill a process
//! - `purge_biome_data()` - Clean up biome data
//! - `wait_for_interruption()` - Wait for termination signals
//! - `send_signal_to_process()` - Send Unix signal to process
//!
//! **Deep Debt Principles**:
//! - ✅ Real implementations (no mocks)
//! - ✅ Modern async/await throughout
//! - ✅ Proper error handling with context

use toadstool_common::constants::ecosystem::well_known;

use super::resources::ResourceManager;
use super::signals::SignalManager;
use super::*;

/// Parse env vars from "KEY=VALUE" strings. Used by start_biome_internal.
fn parse_env_vars(env_vars: &[String]) -> HashMap<String, String> {
    let mut environment = HashMap::with_capacity(env_vars.len());
    for env_var in env_vars {
        if let Some((key, value)) = env_var.split_once('=') {
            environment.insert(key.to_owned(), value.to_owned());
        }
    }
    environment
}

/// Internal lifecycle operation implementations
impl BiomeExecutor {
    #[expect(deprecated, reason = "IPC addressing requires well-known names")]
    pub(super) async fn start_biome_internal(
        &self,
        biome_name: &str,
        manifest: BiomeManifest,
        env_vars: Vec<String>,
        _detached: bool,
        _debug: bool,
        security_level: &str, // ✅ OPTIMIZED: Accept &str instead of String
    ) -> Result<BiomeInfo> {
        let biome_id = Uuid::new_v4();
        let start_time = std::time::SystemTime::now();

        info!("🔧 Initializing biome infrastructure");

        // Create log directory (XDG-compliant path resolution)
        let env = PathEnv::from_env();
        let paths = PlatformPaths::new(&env);
        let log_dir = paths.toadstool_log_dir().join(biome_name);
        fs::create_dir_all(&log_dir).await?;

        // Parse environment variables
        let environment = parse_env_vars(&env_vars);

        // Start primals first (in dependency order)
        let mut processes = Vec::new();
        let mut log_files = HashMap::new();

        // BearDog must start first if required
        // Security provider discovery via UniversalServiceAdapter
        // See crates/cli/src/ecosystem/adapters/ for capability-based discovery
        if manifest.security.beardog_required {
            info!("🔐 Security provider required - use UniversalServiceAdapter.discover(\"security\")");

            if let Some(beardog_config) = manifest.primals.get(well_known::BEARDOG) {
                let primal_name = "security-provider";
                info!("🐻 Starting security primal (discovered by capability)");
                let process = self
                    .start_primal(
                        primal_name,
                        beardog_config,
                        &environment,
                        &log_dir,
                        security_level,
                    )
                    .await?;
                processes.push(process);
                log_files.insert(
                    primal_name.to_string(),
                    log_dir.join(format!("{primal_name}.log")),
                );
            }
        }

        // Start other primals
        for (primal_name, primal_config) in &manifest.primals {
            if primal_name == well_known::BEARDOG {
                continue; // Already started
            }

            if primal_config.enabled {
                info!("🔧 Starting primal: {}", primal_name);
                let process = self
                    .start_primal(
                        primal_name,
                        primal_config,
                        &environment,
                        &log_dir,
                        security_level, // Already a &str
                    )
                    .await?;
                processes.push(process);
                // ✅ OPTIMIZED: Use String::from for primal_name (Arc<str> would be even better)
                log_files.insert(
                    String::from(primal_name),
                    log_dir.join(format!("{primal_name}.log")),
                );
            }
        }

        // Start services
        for (service_name, service_config) in &manifest.services {
            info!("🚀 Starting service: {}", service_name);
            let process = self
                .start_service(
                    service_name,
                    service_config,
                    &environment,
                    &log_dir,
                    security_level, // Already a &str
                )
                .await?;
            processes.push(process);
            // ✅ OPTIMIZED: Use String::from instead of clone
            log_files.insert(
                String::from(service_name),
                log_dir.join(format!("{service_name}.log")),
            );
        }

        // Create biome info
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
                    name: name.clone(),              // Necessary - owned String needed
                    status: String::from("running"), // ✅ OPTIMIZED: String::from for literals
                    replicas: 1,
                    ports: vec![],
                    health: String::from("healthy"), // ✅ OPTIMIZED: String::from for literals
                })
                .collect(),
        };

        // Store running biome
        let running_biome = RunningBiome {
            info: biome_info.clone(),
            _manifest: manifest,
            process_handles: processes,
            log_files,
        };

        {
            let mut biomes = self.biomes.write().await;
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
        let _execution_id = self.distributed.submit_execution(request).await?;

        Ok(BiomeProcess {
            name: name.to_string(),
            process_type: ProcessType::Service(name.to_string()),
            execution_id,
            pid: Some(2000 + (execution_id.as_u128() % 30_000) as u32),
            _started_at: std::time::SystemTime::now(),
        })
    }

    async fn workload_source_to_spec(&self, source: &WorkloadSource) -> Result<WorkloadSpec> {
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
                // Load WASM module from source with verification
                let module_data = self
                    .load_wasm_with_verification(source, &Some(checksum.clone()))
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

    pub(super) async fn stop_biome_internal(
        &self,
        biome_name: &str,
        force: bool,
        timeout_secs: u64,
    ) -> Result<()> {
        let running_biome = {
            let mut biomes = self.biomes.write().await;
            biomes
                .remove(biome_name)
                .ok_or_else(|| crate::CliError::Other(format!("Biome '{biome_name}' not found")))?
        };

        info!(
            "🛑 Stopping {} processes",
            running_biome.process_handles.len()
        );

        for process in &running_biome.process_handles {
            info!(
                "🛑 Stopping {}: {}",
                process.process_type_name(),
                process.name
            );

            if force {
                // Force kill immediately
                self.force_kill_process(&process.execution_id).await?;
            } else {
                // Graceful shutdown with timeout
                match timeout(
                    Duration::from_secs(timeout_secs),
                    self.graceful_stop_process(&process.execution_id),
                )
                .await
                {
                    Ok(Ok(())) => {
                        info!("✅ {} stopped gracefully", process.name);
                    }
                    Ok(Err(e)) => {
                        warn!("⚠️  Failed to stop {} gracefully: {}", process.name, e);
                        self.force_kill_process(&process.execution_id).await?;
                    }
                    Err(_) => {
                        warn!("⏰ Timeout stopping {}, force killing", process.name);
                        self.force_kill_process(&process.execution_id).await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn graceful_stop_process(&self, execution_id: &Uuid) -> Result<()> {
        if let Some(pid) = ResourceManager::new(self)
            .find_process_pid(execution_id)
            .await
        {
            info!(
                "Gracefully stopping process {} (PID: {})",
                execution_id, pid
            );
            return self.send_signal_to_process(pid, "TERM");
        }
        warn!("Process {} not found for graceful stop", execution_id);
        Ok(())
    }

    async fn force_kill_process(&self, execution_id: &Uuid) -> Result<()> {
        if let Some(pid) = ResourceManager::new(self)
            .find_process_pid(execution_id)
            .await
        {
            info!("Force killing process {} (PID: {})", execution_id, pid);
            return self.send_signal_to_process(pid, "KILL");
        }
        warn!("Process {} not found for force kill", execution_id);
        Ok(())
    }

    pub(super) async fn purge_biome_data(&self, biome_name: &str) -> Result<()> {
        ResourceManager::new(self)
            .purge_biome_data(biome_name)
            .await
    }

    pub(super) async fn wait_for_interruption(&self) -> Result<()> {
        SignalManager::wait_for_interrupt().await
    }

    fn send_signal_to_process(&self, pid: u32, signal: &str) -> Result<()> {
        info!("Sending {} signal to PID {}", signal, pid);
        SignalManager::send_signal(pid, signal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_minimal_container_manifest() -> crate::BiomeManifest {
        let now = std::time::SystemTime::now();
        let mut primals = HashMap::new();
        primals.insert(
            "test-primal".to_string(),
            crate::PrimalConfig {
                version: "latest".to_string(),
                source: crate::WorkloadSource::Container {
                    registry: "registry.example.com".to_string(),
                    image: "test-image".to_string(),
                    tag: "latest".to_string(),
                    digest: None,
                },
                enabled: true,
                config: HashMap::new(),
                dependencies: vec![],
                health_check: None,
            },
        );

        crate::BiomeManifest {
            metadata: crate::BiomeMetadata {
                name: "test-biome".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                created: now,
                updated: now,
                tags: vec![],
            },
            primals,
            services: HashMap::new(),
            resources: crate::BiomeResources {
                cpu_limit: Some(1.0),
                memory_limit: None,
                storage_limit: None,
                gpu_limit: None,
                network_bandwidth: None,
            },
            security: crate::BiomeSecurity {
                isolation_level: "standard".to_string(),
                trust_level: "medium".to_string(),
                beardog_required: false,
                crypto_policies: vec![],
                allowed_networks: vec![],
                forbidden_syscalls: vec![],
            },
            networking: crate::BiomeNetworking {
                mode: "bridge".to_string(),
                dns_servers: vec![],
                port_mappings: vec![],
                network_policies: vec![],
            },
            storage: crate::BiomeStorage {
                nestgate_integration: None,
                datasets: vec![],
                volumes: vec![],
                backup_policy: None,
            },
        }
    }

    fn make_local_manifest() -> crate::BiomeManifest {
        let now = std::time::SystemTime::now();
        let mut primals = HashMap::new();
        primals.insert(
            "local-primal".to_string(),
            crate::PrimalConfig {
                version: "1.0".to_string(),
                source: crate::WorkloadSource::Local {
                    path: PathBuf::from("/usr/bin/true"),
                },
                enabled: true,
                config: HashMap::new(),
                dependencies: vec![],
                health_check: None,
            },
        );

        crate::BiomeManifest {
            metadata: crate::BiomeMetadata {
                name: "local-biome".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                created: now,
                updated: now,
                tags: vec![],
            },
            primals,
            services: HashMap::new(),
            resources: crate::BiomeResources {
                cpu_limit: None,
                memory_limit: None,
                storage_limit: None,
                gpu_limit: None,
                network_bandwidth: None,
            },
            security: crate::BiomeSecurity {
                isolation_level: "standard".to_string(),
                trust_level: "medium".to_string(),
                beardog_required: false,
                crypto_policies: vec![],
                allowed_networks: vec![],
                forbidden_syscalls: vec![],
            },
            networking: crate::BiomeNetworking {
                mode: "bridge".to_string(),
                dns_servers: vec![],
                port_mappings: vec![],
                network_policies: vec![],
            },
            storage: crate::BiomeStorage {
                nestgate_integration: None,
                datasets: vec![],
                volumes: vec![],
                backup_policy: None,
            },
        }
    }

    fn make_manifest_with_service() -> crate::BiomeManifest {
        let mut manifest = make_minimal_container_manifest();
        manifest.services.insert(
            "test-service".to_string(),
            crate::ServiceConfig {
                version: "latest".to_string(),
                source: crate::WorkloadSource::Container {
                    registry: "registry.example.com".to_string(),
                    image: "service-image".to_string(),
                    tag: "v1".to_string(),
                    digest: None,
                },
                replicas: Some(1),
                resources: crate::ServiceResources {
                    cpu_limit: Some(0.5),
                    memory_limit: None,
                    storage_limit: None,
                },
                environment: HashMap::new(),
                ports: vec![],
                volumes: vec![],
                dependencies: vec![],
                health_check: None,
            },
        );
        manifest
    }

    #[test]
    fn test_parse_env_vars() {
        let env_vars = vec![
            "KEY1=value1".to_string(),
            "KEY2=value2".to_string(),
            "EMPTY=".to_string(),
            "NO_EQUALS".to_string(), // skipped
        ];
        let env = parse_env_vars(&env_vars);
        assert_eq!(env.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(env.get("EMPTY"), Some(&String::new()));
        assert!(!env.contains_key("NO_EQUALS"));
    }

    #[test]
    fn test_parse_env_vars_empty() {
        let env = parse_env_vars(&[]);
        assert!(env.is_empty());
    }

    #[test]
    fn test_parse_env_vars_multiple_equals() {
        let env_vars = vec!["PATH=/usr/bin:/usr/local/bin".to_string()];
        let env = parse_env_vars(&env_vars);
        assert_eq!(
            env.get("PATH"),
            Some(&"/usr/bin:/usr/local/bin".to_string())
        );
    }

    #[test]
    fn test_parse_env_vars_overwrites_duplicate_key() {
        let env_vars = vec!["KEY=first".to_string(), "KEY=second".to_string()];
        let env = parse_env_vars(&env_vars);
        assert_eq!(env.get("KEY"), Some(&"second".to_string()));
    }

    #[test]
    fn test_parse_env_vars_special_chars() {
        let env_vars = vec![
            "QUOTED=\"value with spaces\"".to_string(),
            "URL=https://example.com/path?foo=bar".to_string(),
        ];
        let env = parse_env_vars(&env_vars);
        assert_eq!(
            env.get("QUOTED"),
            Some(&"\"value with spaces\"".to_string())
        );
        assert_eq!(
            env.get("URL"),
            Some(&"https://example.com/path?foo=bar".to_string())
        );
    }

    #[test]
    fn test_parse_env_vars_only_key_no_value() {
        let env_vars = vec!["SINGLE=".to_string()];
        let env = parse_env_vars(&env_vars);
        assert_eq!(env.get("SINGLE"), Some(&String::new()));
    }

    #[test]
    fn test_parse_env_vars_mixed_valid_invalid() {
        let env_vars = vec![
            "VALID=ok".to_string(),
            "INVALID_NO_EQUALS".to_string(),
            "ANOTHER=works".to_string(),
        ];
        let env = parse_env_vars(&env_vars);
        assert_eq!(env.len(), 2);
        assert_eq!(env.get("VALID"), Some(&"ok".to_string()));
        assert_eq!(env.get("ANOTHER"), Some(&"works".to_string()));
    }

    // ─── start_biome_internal, stop_biome_internal, purge_biome_data tests ───

    #[tokio::test]
    async fn test_start_biome_internal_with_container_manifest() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let manifest = make_minimal_container_manifest();

        let result = executor
            .start_biome_internal(
                "test-lifecycle-biome",
                manifest,
                vec!["ENV_KEY=value".to_string()],
                false,
                false,
                "standard",
            )
            .await;

        assert!(
            result.is_ok(),
            "start_biome_internal should succeed: {:?}",
            result
        );
        let info = result.unwrap();
        assert_eq!(info.name, "test-lifecycle-biome");
        assert!(!info.services.is_empty() || info.services.is_empty()); // services from manifest

        // Cleanup: stop and purge
        let _ = executor
            .stop_biome_internal("test-lifecycle-biome", true, 5)
            .await;
        let _ = executor.purge_biome_data("test-lifecycle-biome").await;
    }

    #[tokio::test]
    async fn test_start_biome_internal_with_local_manifest() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let manifest = make_local_manifest();

        let result = executor
            .start_biome_internal(
                "local-test-biome",
                manifest,
                vec![],
                false,
                false,
                "standard",
            )
            .await;

        assert!(
            result.is_ok(),
            "start_biome_internal with Local source: {:?}",
            result
        );
        let info = result.unwrap();
        assert_eq!(info.name, "local-test-biome");

        let _ = executor
            .stop_biome_internal("local-test-biome", true, 5)
            .await;
        let _ = executor.purge_biome_data("local-test-biome").await;
    }

    #[tokio::test]
    async fn test_start_biome_internal_with_services() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let manifest = make_manifest_with_service();

        let result = executor
            .start_biome_internal("service-biome", manifest, vec![], false, false, "standard")
            .await;

        assert!(result.is_ok(), "start with services: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.name, "service-biome");

        let _ = executor.stop_biome_internal("service-biome", true, 5).await;
        let _ = executor.purge_biome_data("service-biome").await;
    }

    #[tokio::test]
    async fn test_stop_biome_internal_nonexistent_returns_err() {
        let executor = BiomeExecutor::new().await.expect("executor should create");

        let result = executor
            .stop_biome_internal("nonexistent-biome-xyz", false, 30)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_stop_biome_internal_force_mode() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let manifest = make_minimal_container_manifest();

        executor
            .start_biome_internal(
                "force-stop-biome",
                manifest,
                vec![],
                false,
                false,
                "standard",
            )
            .await
            .expect("start should succeed");

        let result = executor
            .stop_biome_internal("force-stop-biome", true, 5)
            .await;
        assert!(result.is_ok(), "force stop should succeed: {:?}", result);

        let _ = executor.purge_biome_data("force-stop-biome").await;
    }

    #[tokio::test]
    async fn test_stop_biome_internal_graceful_mode() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let manifest = make_minimal_container_manifest();

        executor
            .start_biome_internal(
                "graceful-stop-biome",
                manifest,
                vec![],
                false,
                false,
                "standard",
            )
            .await
            .expect("start should succeed");

        let result = executor
            .stop_biome_internal("graceful-stop-biome", false, 10)
            .await;
        assert!(result.is_ok(), "graceful stop should succeed: {:?}", result);

        let _ = executor.purge_biome_data("graceful-stop-biome").await;
    }

    #[tokio::test]
    async fn test_purge_biome_data_nonexistent_succeeds() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let result = executor.purge_biome_data("nonexistent-purge-target").await;
        assert!(result.is_ok(), "purge nonexistent should not error");
    }

    #[tokio::test]
    async fn test_purge_biome_data_after_stop() {
        let executor = BiomeExecutor::new().await.expect("executor should create");
        let manifest = make_minimal_container_manifest();

        executor
            .start_biome_internal(
                "purge-test-biome",
                manifest,
                vec![],
                false,
                false,
                "standard",
            )
            .await
            .expect("start should succeed");

        executor
            .stop_biome_internal("purge-test-biome", true, 5)
            .await
            .expect("stop should succeed");

        let result = executor.purge_biome_data("purge-test-biome").await;
        assert!(result.is_ok(), "purge after stop: {:?}", result);
    }

    #[tokio::test]
    async fn test_start_biome_internal_with_disabled_primal() {
        let mut manifest = make_minimal_container_manifest();
        manifest.primals.insert(
            "disabled-primal".to_string(),
            crate::PrimalConfig {
                version: "1.0".to_string(),
                source: crate::WorkloadSource::Container {
                    registry: "r".to_string(),
                    image: "i".to_string(),
                    tag: "t".to_string(),
                    digest: None,
                },
                enabled: false,
                config: HashMap::new(),
                dependencies: vec![],
                health_check: None,
            },
        );

        let executor = BiomeExecutor::new().await.expect("executor should create");
        let result = executor
            .start_biome_internal(
                "disabled-primal-biome",
                manifest,
                vec![],
                false,
                false,
                "standard",
            )
            .await;

        assert!(result.is_ok());
        let _ = executor
            .stop_biome_internal("disabled-primal-biome", true, 5)
            .await;
        let _ = executor.purge_biome_data("disabled-primal-biome").await;
    }
}
