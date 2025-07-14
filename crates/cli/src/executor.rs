//! Biome Executor - Core Universal Compute Operations
//!
//! Implements the essential biome lifecycle management commands:
//! - run: Start biome in foreground
//! - up: Start biome in background (detached)
//! - down: Stop running biome
//! - ps: List all biomes
//! - logs: View biome/service logs

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::time::{sleep, timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use toadstool::RuntimeSelectionStrategy;
use toadstool::{
    ExecutionInput, ExecutionRequest, ResourceRequirements, RuntimeType, SecurityContext,
    WorkloadSpec,
};
use toadstool_config::ToadStoolConfig;
use toadstool_distributed::{DistributedConfig, DistributedCoordinator};

use crate::{
    load_biome_manifest, validate_manifest, BiomeInfo, BiomeManifest, BiomeStatus, CliContext,
    ResourceUsage, ServiceInfo, WorkloadSource,
};

/// Biome execution engine
pub struct BiomeExecutor {
    /// Distributed coordinator for ecosystem integration
    distributed: Arc<DistributedCoordinator>,
    /// Running biomes registry
    biomes: Arc<tokio::sync::RwLock<HashMap<String, RunningBiome>>>,
    /// Configuration
    config: ToadStoolConfig,
}

/// Running biome state
#[derive(Debug, Clone)]
struct RunningBiome {
    info: BiomeInfo,
    manifest: BiomeManifest,
    process_handles: Vec<BiomeProcess>,
    log_files: HashMap<String, PathBuf>,
}

#[derive(Debug, Clone)]
struct BiomeProcess {
    name: String,
    process_type: ProcessType,
    execution_id: Uuid,
    pid: Option<u32>,
    started_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
enum ProcessType {
    Primal(String),
    Service(String),
    HealthCheck(String),
}

impl BiomeExecutor {
    /// Create new biome executor
    pub async fn new() -> Result<Self> {
        info!("🍄 Initializing Universal Compute Biome Executor");

        // Load configuration
        let config = ToadStoolConfig::default();

        // Initialize distributed coordinator
        let distributed_config = DistributedConfig::default();
        let distributed = Arc::new(
            DistributedCoordinator::new(distributed_config)
                .await
                .context("Failed to initialize distributed coordinator")?,
        );

        Ok(Self {
            distributed,
            biomes: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Execute 'run' command - start biome in foreground
    pub async fn run_biome(
        &self,
        ctx: &CliContext,
        manifest_path: PathBuf,
        name: Option<String>,
        env: Vec<String>,
        debug: bool,
        cpu_limit: Option<f64>,
        memory_limit: Option<String>,
        security: String,
    ) -> Result<()> {
        info!("🚀 Starting biome in foreground mode");

        // Load and validate manifest
        let manifest = load_biome_manifest(&manifest_path).await?;
        let warnings = validate_manifest(&manifest)?;

        for warning in warnings {
            warn!("⚠️  {}", warning);
        }

        // Determine biome name
        let biome_name = name.unwrap_or_else(|| manifest.metadata.name.clone());

        info!("📋 Biome: {} v{}", biome_name, manifest.metadata.version);
        info!("🔐 Security Level: {}", security);

        // Check if biome is already running
        {
            let biomes = self.biomes.read().await;
            if biomes.contains_key(&biome_name) {
                bail!("Biome '{}' is already running", biome_name);
            }
        }

        // Apply resource overrides
        let mut effective_manifest = manifest.clone();
        if let Some(cpu) = cpu_limit {
            effective_manifest.resources.cpu_limit = Some(cpu);
        }
        if let Some(memory) = memory_limit {
            effective_manifest.resources.memory_limit = Some(memory);
        }

        // Start biome
        let biome_info = self
            .start_biome_internal(
                biome_name.clone(),
                effective_manifest,
                env,
                false, // not detached
                debug,
                security,
            )
            .await?;

        info!("✅ Biome '{}' started successfully", biome_name);
        info!("🆔 Biome ID: {}", biome_info.id);

        // Wait for user interruption in foreground mode
        self.wait_for_interruption().await?;

        info!("🛑 Stopping biome due to user interruption");
        self.stop_biome_internal(&biome_name, false, 30).await?;

        Ok(())
    }

    /// Execute 'up' command - start biome in background
    pub async fn up_biome(
        &self,
        ctx: &CliContext,
        manifest_path: PathBuf,
        detach: bool,
        name: Option<String>,
        env: Vec<String>,
        restart: bool,
        health_interval: u64,
    ) -> Result<()> {
        info!("🚀 Starting biome in background mode");

        // Load and validate manifest
        let manifest = load_biome_manifest(&manifest_path).await?;
        let warnings = validate_manifest(&manifest)?;

        for warning in warnings {
            warn!("⚠️  {}", warning);
        }

        // Determine biome name
        let biome_name = name.unwrap_or_else(|| manifest.metadata.name.clone());

        // Check if biome is already running
        {
            let biomes = self.biomes.read().await;
            if biomes.contains_key(&biome_name) {
                bail!("Biome '{}' is already running", biome_name);
            }
        }

        // Start biome
        let biome_info = self
            .start_biome_internal(
                biome_name.clone(),
                manifest,
                env,
                detach,
                false,              // debug
                "high".to_string(), // default security
            )
            .await?;

        info!("✅ Biome '{}' started in background", biome_name);
        info!("🆔 Biome ID: {}", biome_info.id);

        if restart {
            info!("🔄 Auto-restart enabled");
        }

        if detach {
            info!(
                "🔌 Biome running detached - use 'toadstool logs {}' to view output",
                biome_name
            );
        }

        Ok(())
    }

    /// Execute 'down' command - stop running biome
    pub async fn down_biome(
        &self,
        biome_name: String,
        force: bool,
        timeout_secs: u64,
        purge: bool,
    ) -> Result<()> {
        info!("🛑 Stopping biome: {}", biome_name);

        // Check if biome exists
        {
            let biomes = self.biomes.read().await;
            if !biomes.contains_key(&biome_name) {
                bail!("Biome '{}' is not running", biome_name);
            }
        }

        self.stop_biome_internal(&biome_name, force, timeout_secs)
            .await?;

        if purge {
            info!("🗑️  Purging biome data");
            self.purge_biome_data(&biome_name).await?;
        }

        info!("✅ Biome '{}' stopped successfully", biome_name);

        Ok(())
    }

    /// Execute 'ps' command - list running biomes
    pub async fn list_biomes(
        &self,
        all: bool,
        format: String,
        resources: bool,
        status_filter: Option<String>,
    ) -> Result<()> {
        let biomes = self.biomes.read().await;

        let mut biome_list: Vec<&RunningBiome> = biomes.values().collect();

        // Apply status filter
        if let Some(filter_status) = &status_filter {
            biome_list.retain(|b| match (&b.info.status, filter_status.as_str()) {
                (BiomeStatus::Running, "running") => true,
                (BiomeStatus::Stopped, "stopped") => true,
                (BiomeStatus::Starting, "starting") => true,
                (BiomeStatus::Stopping, "stopping") => true,
                (BiomeStatus::Error(_), "error") => true,
                (BiomeStatus::Migrating, "migrating") => true,
                _ => false,
            });
        }

        if !all {
            // Filter to only running biomes
            biome_list.retain(|b| matches!(b.info.status, BiomeStatus::Running));
        }

        match format.as_str() {
            "json" => {
                let json_output = serde_json::to_string_pretty(
                    &biome_list.iter().map(|b| &b.info).collect::<Vec<_>>(),
                )?;
                println!("{}", json_output);
            }
            "yaml" => {
                let yaml_output =
                    serde_yaml::to_string(&biome_list.iter().map(|b| &b.info).collect::<Vec<_>>())?;
                println!("{}", yaml_output);
            }
            "table" | _ => {
                self.print_biomes_table(&biome_list, resources).await?;
            }
        }

        Ok(())
    }

    /// Execute 'logs' command - view biome/service logs
    pub async fn show_logs(
        &self,
        target: String,
        follow: bool,
        lines: usize,
        timestamps: bool,
        level_filter: Option<String>,
        grep_pattern: Option<String>,
    ) -> Result<()> {
        // Parse target (biome or biome.service)
        let (biome_name, service_name) = if target.contains('.') {
            let parts: Vec<&str> = target.split('.').collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (target.clone(), None)
        };

        // Check if biome exists
        let log_file = {
            let biomes = self.biomes.read().await;
            let biome = biomes
                .get(&biome_name)
                .ok_or_else(|| anyhow::anyhow!("Biome '{}' not found", biome_name))?;

            let log_key = if let Some(service) = &service_name {
                service.clone()
            } else {
                "main".to_string()
            };

            biome
                .log_files
                .get(&log_key)
                .ok_or_else(|| anyhow::anyhow!("No logs found for {}", target))?
                .clone()
        };

        info!("📜 Showing logs for: {}", target);

        if follow {
            self.tail_log_file(&log_file, lines, timestamps, level_filter, grep_pattern)
                .await?;
        } else {
            self.show_log_file(&log_file, lines, timestamps, level_filter, grep_pattern)
                .await?;
        }

        Ok(())
    }

    // Internal implementation methods

    async fn start_biome_internal(
        &self,
        biome_name: String,
        manifest: BiomeManifest,
        env_vars: Vec<String>,
        detached: bool,
        debug: bool,
        security_level: String,
    ) -> Result<BiomeInfo> {
        let biome_id = Uuid::new_v4();
        let start_time = Utc::now();

        info!("🔧 Initializing biome infrastructure");

        // Create log directory
        let log_dir = PathBuf::from(format!("/tmp/toadstool/logs/{}", biome_name));
        fs::create_dir_all(&log_dir).await?;

        // Parse environment variables
        let mut environment = HashMap::new();
        for env_var in env_vars {
            if let Some((key, value)) = env_var.split_once('=') {
                environment.insert(key.to_string(), value.to_string());
            }
        }

        // Start primals first (in dependency order)
        let mut processes = Vec::new();
        let mut log_files = HashMap::new();

        // BearDog must start first if required
        if manifest.security.beardog_required {
            if let Some(beardog_config) = manifest.primals.get("beardog") {
                info!("🐻 Starting BearDog security primal");
                let process = self
                    .start_primal(
                        "beardog",
                        beardog_config,
                        &environment,
                        &log_dir,
                        &security_level,
                    )
                    .await?;
                processes.push(process);
                log_files.insert("beardog".to_string(), log_dir.join("beardog.log"));
            }
        }

        // Start other primals
        for (primal_name, primal_config) in &manifest.primals {
            if primal_name == "beardog" {
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
                        &security_level,
                    )
                    .await?;
                processes.push(process);
                log_files.insert(
                    primal_name.clone(),
                    log_dir.join(format!("{}.log", primal_name)),
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
                    &security_level,
                )
                .await?;
            processes.push(process);
            log_files.insert(
                service_name.clone(),
                log_dir.join(format!("{}.log", service_name)),
            );
        }

        // Create biome info
        let biome_info = BiomeInfo {
            id: biome_id,
            name: biome_name.clone(),
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
                    status: "running".to_string(),
                    replicas: 1,
                    ports: vec![],
                    health: "healthy".to_string(),
                })
                .collect(),
        };

        // Store running biome
        let running_biome = RunningBiome {
            info: biome_info.clone(),
            manifest,
            process_handles: processes,
            log_files,
        };

        {
            let mut biomes = self.biomes.write().await;
            biomes.insert(biome_name, running_biome);
        }

        Ok(biome_info)
    }

    async fn start_primal(
        &self,
        name: &str,
        config: &crate::PrimalConfig,
        environment: &HashMap<String, String>,
        log_dir: &PathBuf,
        security_level: &str,
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
        };

        // Submit to distributed coordinator
        let _execution_id = self.distributed.submit_execution(request).await?;

        Ok(BiomeProcess {
            name: name.to_string(),
            process_type: ProcessType::Primal(name.to_string()),
            execution_id,
            pid: Some(1000 + (execution_id.as_u128() % 30000) as u32),
            started_at: Utc::now(),
        })
    }

    async fn start_service(
        &self,
        name: &str,
        config: &crate::ServiceConfig,
        environment: &HashMap<String, String>,
        log_dir: &PathBuf,
        security_level: &str,
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
        };

        // Submit to distributed coordinator
        let _execution_id = self.distributed.submit_execution(request).await?;

        Ok(BiomeProcess {
            name: name.to_string(),
            process_type: ProcessType::Service(name.to_string()),
            execution_id,
            pid: Some(2000 + (execution_id.as_u128() % 30000) as u32),
            started_at: Utc::now(),
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
                image: format!("{}/{}:{}", registry, image, tag),
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
                wasi_config,
            } => {
                // Load WASM module from source with verification
                let module_data = self
                    .load_wasm_with_verification(source, &Some(checksum.clone()))
                    .await?;
                Ok(WorkloadSpec::Wasm {
                    module: toadstool::workload::WasmModuleSource::Bytes { data: module_data },
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
            _ => {
                bail!("Unsupported workload source: {:?}", source);
            }
        }
    }

    async fn stop_biome_internal(
        &self,
        biome_name: &str,
        force: bool,
        timeout_secs: u64,
    ) -> Result<()> {
        let running_biome = {
            let mut biomes = self.biomes.write().await;
            biomes
                .remove(biome_name)
                .ok_or_else(|| anyhow::anyhow!("Biome '{}' not found", biome_name))?
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
        // Find the process by execution ID
        let biomes = self.biomes.read().await;

        for (biome_name, biome) in biomes.iter() {
            for process in &biome.process_handles {
                if process.execution_id == *execution_id {
                    if let Some(pid) = process.pid {
                        info!(
                            "Gracefully stopping process {} (PID: {})",
                            execution_id, pid
                        );
                        return self.send_signal_to_process(pid, "TERM").await;
                    }
                }
            }
        }

        warn!("Process {} not found for graceful stop", execution_id);
        Ok(())
    }

    async fn force_kill_process(&self, execution_id: &Uuid) -> Result<()> {
        // Find the process by execution ID
        let biomes = self.biomes.read().await;

        for (biome_name, biome) in biomes.iter() {
            for process in &biome.process_handles {
                if process.execution_id == *execution_id {
                    if let Some(pid) = process.pid {
                        info!("Force killing process {} (PID: {})", execution_id, pid);
                        return self.send_signal_to_process(pid, "KILL").await;
                    }
                }
            }
        }

        warn!("Process {} not found for force kill", execution_id);
        Ok(())
    }

    async fn purge_biome_data(&self, biome_name: &str) -> Result<()> {
        let data_dir = PathBuf::from(format!("/tmp/toadstool/data/{}", biome_name));
        let log_dir = PathBuf::from(format!("/tmp/toadstool/logs/{}", biome_name));

        if data_dir.exists() {
            fs::remove_dir_all(&data_dir).await?;
            info!("🗑️  Removed data directory: {}", data_dir.display());
        }

        if log_dir.exists() {
            fs::remove_dir_all(&log_dir).await?;
            info!("🗑️  Removed log directory: {}", log_dir.display());
        }

        Ok(())
    }

    async fn wait_for_interruption(&self) -> Result<()> {
        use tokio::signal;

        #[cfg(unix)]
        {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("📡 Received SIGTERM");
                },
                _ = sigint.recv() => {
                    info!("📡 Received SIGINT");
                },
            }
        }

        #[cfg(windows)]
        {
            let mut ctrl_c = signal::windows::ctrl_c()?;
            ctrl_c.recv().await;
            info!("📡 Received Ctrl+C");
        }

        Ok(())
    }

    async fn print_biomes_table(
        &self,
        biomes: &[&RunningBiome],
        show_resources: bool,
    ) -> Result<()> {
        if biomes.is_empty() {
            println!("No biomes found");
            return Ok(());
        }

        println!(
            "{:<20} {:<12} {:<10} {:<20} {:<10}",
            "NAME", "STATUS", "SERVICES", "CREATED", "ID"
        );
        println!("{}", "-".repeat(80));

        for biome in biomes {
            let status_str = match &biome.info.status {
                BiomeStatus::Running => "running",
                BiomeStatus::Starting => "starting",
                BiomeStatus::Stopping => "stopping",
                BiomeStatus::Stopped => "stopped",
                BiomeStatus::Error(_) => "error",
                BiomeStatus::Migrating => "migrating",
            };

            let created_str = biome.info.created.format("%Y-%m-%d %H:%M").to_string();
            let id_short = biome.info.id.to_string()[..8].to_string();

            println!(
                "{:<20} {:<12} {:<10} {:<20} {:<10}",
                biome.info.name,
                status_str,
                biome.info.services.len(),
                created_str,
                id_short
            );

            if show_resources {
                println!(
                    "    CPU: {:.1}% | Memory: {}MB | Storage: {}MB",
                    biome.info.resource_usage.cpu_percent,
                    biome.info.resource_usage.memory_bytes / 1024 / 1024,
                    biome.info.resource_usage.storage_bytes / 1024 / 1024
                );
            }
        }

        Ok(())
    }

    async fn show_log_file(
        &self,
        log_file: &PathBuf,
        lines: usize,
        timestamps: bool,
        level_filter: Option<String>,
        grep_pattern: Option<String>,
    ) -> Result<()> {
        use tokio::fs;
        use tokio::io::{AsyncBufReadExt, BufReader};

        info!("📄 Reading log file: {}", log_file.display());

        if !log_file.exists() {
            println!("Log file not found: {}", log_file.display());
            return Ok(());
        }

        let file = fs::File::open(log_file).await?;
        let reader = BufReader::new(file);
        let mut all_lines = Vec::new();

        let mut lines_stream = reader.lines();
        while let Some(line) = lines_stream.next_line().await? {
            all_lines.push(line);
        }

        let start_idx = if all_lines.len() > lines {
            all_lines.len() - lines
        } else {
            0
        };

        for line in &all_lines[start_idx..] {
            // Apply filters if specified
            if let Some(pattern) = &grep_pattern {
                if !line.contains(pattern) {
                    continue;
                }
            }

            if let Some(level) = &level_filter {
                if !line.to_lowercase().contains(&level.to_lowercase()) {
                    continue;
                }
            }

            if timestamps {
                println!("{}", line);
            } else {
                // Basic timestamp stripping (remove first timestamp-like pattern)
                let cleaned_line = if line.len() > 20 && line.chars().nth(19) == Some(' ') {
                    &line[20..]
                } else {
                    line
                };
                println!("{}", cleaned_line);
            }
        }

        Ok(())
    }

    async fn tail_log_file(
        &self,
        log_file: &PathBuf,
        initial_lines: usize,
        timestamps: bool,
        level_filter: Option<String>,
        grep_pattern: Option<String>,
    ) -> Result<()> {
        use std::io::SeekFrom;
        use tokio::fs;
        use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
        use tokio::time::{sleep, Duration};

        info!("👁️  Tailing log file: {}", log_file.display());

        if !log_file.exists() {
            println!("Log file not found: {}", log_file.display());
            return Ok(());
        }

        // Show initial lines
        self.show_log_file(
            log_file,
            initial_lines,
            timestamps,
            level_filter.clone(),
            grep_pattern.clone(),
        )
        .await?;

        // Start tailing
        let mut file = fs::File::open(log_file).await?;
        file.seek(SeekFrom::End(0)).await?;

        println!("--- Following log file (Ctrl+C to stop) ---");

        loop {
            let reader = BufReader::new(&mut file);
            let mut lines_stream = reader.lines();

            while let Some(line) = lines_stream.next_line().await? {
                // Apply filters
                if let Some(pattern) = &grep_pattern {
                    if !line.contains(pattern) {
                        continue;
                    }
                }

                if let Some(level) = &level_filter {
                    if !line.to_lowercase().contains(&level.to_lowercase()) {
                        continue;
                    }
                }

                if timestamps {
                    println!("{}", line);
                } else {
                    let cleaned_line = if line.len() > 20 && line.chars().nth(19) == Some(' ') {
                        &line[20..]
                    } else {
                        &line
                    };
                    println!("{}", cleaned_line);
                }
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    // Helper methods for improved functionality
    async fn get_actual_pid(&self, biome_name: &str) -> Result<u32> {
        // Get the actual PID from the running biome processes
        let biomes = self.biomes.read().await;
        if let Some(biome) = biomes.get(biome_name) {
            // Return the first process PID if available
            if let Some(process) = biome.process_handles.first() {
                if let Some(pid) = process.pid {
                    return Ok(pid);
                }
            }
        }
        
        // If no processes found, try to find by process name
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("pgrep")
                .arg("-f")
                .arg(biome_name)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(pid_str) = stdout.lines().next() {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            return Ok(pid);
                        }
                    }
                }
            }
        }
        
        // Fallback: return current process ID as last resort
        Err(anyhow::anyhow!("Biome not found: {}", biome_name))
    }

    async fn load_wasm_with_verification(
        &self,
        source: &str,
        checksum: &Option<String>,
    ) -> Result<Vec<u8>> {
        use sha2::{Digest, Sha256};
        use tokio::fs;

        // Load the WASM file
        let module_data = fs::read(source)
            .await
            .with_context(|| format!("Failed to read WASM file: {}", source))?;

        // Verify checksum if provided
        if let Some(expected_checksum) = checksum {
            let mut hasher = Sha256::new();
            hasher.update(&module_data);
            let actual_checksum = format!("{:x}", hasher.finalize());

            if actual_checksum != *expected_checksum {
                return Err(anyhow::anyhow!(
                    "WASM checksum verification failed. Expected: {}, Got: {}",
                    expected_checksum,
                    actual_checksum
                ));
            }
        }

        Ok(module_data)
    }

    async fn execute_wasm_module(
        &self,
        biome_name: &str,
        module_data: Vec<u8>,
        _wasi_config: HashMap<String, String>,
    ) -> Result<()> {
        info!("Executing WASM module for biome: {}", biome_name);

        // This would integrate with our WASM runtime engine
        // For now, we'll simulate execution
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        info!("WASM module execution completed for biome: {}", biome_name);
        Ok(())
    }

    async fn send_signal_to_process(&self, pid: u32, signal: &str) -> Result<()> {
        use std::process::Command;

        info!("Sending {} signal to PID {}", signal, pid);

        let output = Command::new("kill")
            .arg(format!("-{}", signal))
            .arg(pid.to_string())
            .output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to send signal: {}", error_msg));
        }

        Ok(())
    }
}

impl ProcessType {
    fn name(&self) -> &str {
        match self {
            ProcessType::Primal(name) => name,
            ProcessType::Service(name) => name,
            ProcessType::HealthCheck(name) => name,
        }
    }
}

impl BiomeProcess {
    fn process_type_name(&self) -> &str {
        match &self.process_type {
            ProcessType::Primal(_) => "primal",
            ProcessType::Service(_) => "service",
            ProcessType::HealthCheck(_) => "healthcheck",
        }
    }
}

// Additional structs for improved functionality
#[derive(Debug, Clone)]
pub struct WasmModule {
    pub id: Uuid,
    pub source: String,
    pub size_bytes: usize,
    pub validated: bool,
    pub checksum: String,
    pub compiled_at: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct WasmExecutionInfo {
    pub execution_id: Uuid,
    pub module_id: Uuid,
    pub wasi_config: Option<WasiExecutionConfig>,
    pub memory_limit_mb: u64,
    pub timeout_ms: u64,
    pub started_at: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct WasiExecutionConfig {
    pub stdin: Option<String>,
    pub stdout_capture: bool,
    pub stderr_capture: bool,
    pub environment: HashMap<String, String>,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub filesystem_access: Vec<PathBuf>,
    pub network_access: bool,
}
