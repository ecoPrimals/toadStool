impl BiomeExecutor {
    /// Create new biome executor
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The distributed coordinator fails to initialize
    /// - Configuration loading fails
    #[must_use = "BiomeExecutor creation should be checked"]
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

        // ✅ NEW: Connect to biomeOS registry for capability-based discovery
        let biomeos_client = match toadstool::biomeos_integration::BiomeOSClient::connect().await {
            Ok(client) => {
                info!("✅ Connected to biomeOS registry");
                
                // Register ToadStool capabilities
                if let Err(e) = client.register_self().await {
                    warn!("⚠️  Failed to register with biomeOS: {e}");
                } else {
                    info!("📝 Registered ToadStool capabilities with biomeOS");
                }
                
                Some(Arc::new(client))
            }
            Err(e) => {
                warn!("⚠️  biomeOS registry not available: {e}");
                warn!("   Running in standalone mode (no primal discovery)");
                None
            }
        };

        Ok(Self {
            distributed,
            biomes: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            _config: config,
            biomeos_client,
        })
    }

    /// Execute 'run' command - start biome in foreground
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Manifest loading or validation fails
    /// - Biome is already running with the same name
    /// - Biome startup fails
    /// - User interruption handling fails
    #[allow(clippy::too_many_arguments)]
    #[must_use = "Result of run_biome should be checked"]
    pub async fn run_biome(
        &self,
        _ctx: &CliContext,
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
                bail!("Biome '{biome_name}' is already running");
            }
        }

        // ✅ OPTIMIZED: Apply resource overrides in-place (avoid clone)
        let mut effective_manifest = manifest;
        if let Some(cpu) = cpu_limit {
            effective_manifest.resources.cpu_limit = Some(cpu);
        }
        if let Some(memory) = memory_limit {
            effective_manifest.resources.memory_limit = Some(memory);
        }

        // Start biome (pass by reference to avoid clone)
        let biome_info = self
            .start_biome_internal(
                &biome_name, // Now accepts &str - no clone needed
                effective_manifest,
                env,
                false, // not detached
                debug,
                &security, // ✅ OPTIMIZED: Pass &str reference
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Manifest loading or validation fails
    /// - Biome is already running with the same name
    /// - Biome startup in detached mode fails
    #[allow(clippy::too_many_arguments)]
    #[must_use = "Result of up_biome should be checked"]
    pub async fn up_biome(
        &self,
        _ctx: &CliContext,
        manifest_path: PathBuf,
        detach: bool,
        name: Option<String>,
        env: Vec<String>,
        restart: bool,
        _health_interval: u64,
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
                bail!("Biome '{biome_name}' is already running");
            }
        }

        // Start biome
        let biome_info = self
            .start_biome_internal(
                &biome_name,
                manifest,
                env,
                detach,
                false, // debug
                "high", // ✅ OPTIMIZED: Use &str instead of String allocation
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Biome is not currently running
    /// - Biome stop operation fails
    /// - Purge operation fails (if requested)
    #[must_use = "Result of down_biome should be checked"]
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
                bail!("Biome '{biome_name}' is not running");
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JSON/YAML serialization fails
    /// - Table formatting fails
    #[must_use = "Result of list_biomes should be checked"]
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
            biome_list.retain(|b| {
                matches!(
                    (&b.info.status, filter_status.as_str()),
                    (BiomeStatus::Running, "running")
                        | (BiomeStatus::Stopped, "stopped")
                        | (BiomeStatus::Starting, "starting")
                        | (BiomeStatus::Stopping, "stopping")
                        | (BiomeStatus::Error(_), "error")
                        | (BiomeStatus::Migrating, "migrating")
                )
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
                println!("{json_output}");
            }
            "yaml" => {
                let yaml_output =
                    serde_yaml::to_string(&biome_list.iter().map(|b| &b.info).collect::<Vec<_>>())?;
                println!("{yaml_output}");
            }
            "table" => {
                self.print_biomes_table(&biome_list, resources).await?;
            }
            _ => {
                self.print_biomes_table(&biome_list, resources).await?;
            }
        }

        Ok(())
    }

    /// Execute 'logs' command - view biome/service logs
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Biome or service not found
    /// - Log file cannot be read
    /// - Log filtering/following fails
    #[must_use = "Result of show_logs should be checked"]
    pub async fn show_logs(
        &self,
        target: String,
        follow: bool,
        lines: usize,
        timestamps: bool,
        level_filter: Option<String>,
        grep_pattern: Option<String>,
    ) -> Result<()> {
        // ✅ OPTIMIZED: Parse target without unnecessary allocations
        let (biome_name, service_name) = if let Some((biome, service)) = target.split_once('.') {
            (biome.to_string(), Some(service.to_string()))
        } else {
            (target.clone(), None) // Clone needed since target is used in error below
        };

        // Check if biome exists
        let log_file = {
            let biomes = self.biomes.read().await;
            let biome = biomes
                .get(&biome_name)
                .ok_or_else(|| anyhow::anyhow!("Biome '{biome_name}' not found"))?;

            // ✅ OPTIMIZED: Avoid unnecessary clones by using as_deref
            let log_key = service_name.as_deref().unwrap_or("main");

            biome
                .log_files
                .get(log_key)
                .ok_or_else(|| anyhow::anyhow!("No logs found for {target}"))?
                .clone() // This clone is necessary (PathBuf from HashMap)
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

    // -------------------------------------------------------------------------
    // Primal Discovery Methods (Capability-Based)
    // -------------------------------------------------------------------------

    /// Discover security provider (e.g., BearDog) via capability-based discovery
    ///
    /// **Design**: Query biomeOS registry for Security capability provider.
    /// Falls back to hardcoded localhost if biomeOS unavailable (backward compat).
    #[allow(dead_code)] // TODO: Use in start_biome_internal (Phase 2.3)
    async fn discover_security_provider(
        &self,
    ) -> Result<toadstool::biomeos_integration::PrimalInfo> {
        if let Some(ref client) = self.biomeos_client {
            // Primary: Discover via biomeOS registry
            match client.get_security_provider().await {
                Ok(provider) => {
                    info!("✅ Discovered security provider: {} at {}", provider.name, provider.endpoint);
                    return Ok(provider);
                }
                Err(e) => {
                    warn!("⚠️  Failed to discover security provider via biomeOS: {e}");
                }
            }
        }

        // Fallback: Hardcoded localhost (backward compatibility)
        warn!("📍 Using fallback security provider (localhost:8081)");
        Ok(toadstool::biomeos_integration::PrimalInfo {
            name: "beardog".to_string(),
            endpoint: "http://localhost:8081".to_string(),
            capabilities: vec![],
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Discover discovery/coordination provider (e.g., Songbird) via capability-based discovery
    ///
    /// **Design**: Query biomeOS registry for Discovery capability provider.
    #[allow(dead_code)] // TODO: Use in distributed coordinator setup (Phase 2.3)
    async fn discover_discovery_provider(
        &self,
    ) -> Result<toadstool::biomeos_integration::PrimalInfo> {
        if let Some(ref client) = self.biomeos_client {
            match client.get_discovery_provider().await {
                Ok(provider) => {
                    info!("✅ Discovered discovery provider: {} at {}", provider.name, provider.endpoint);
                    return Ok(provider);
                }
                Err(e) => {
                    warn!("⚠️  Failed to discover discovery provider via biomeOS: {e}");
                }
            }
        }

        // Fallback: Hardcoded localhost
        warn!("📍 Using fallback discovery provider (localhost:8082)");
        Ok(toadstool::biomeos_integration::PrimalInfo {
            name: "songbird".to_string(),
            endpoint: "http://localhost:8082".to_string(),
            capabilities: vec![],
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Discover storage provider (e.g., NestGate) via capability-based discovery
    ///
    /// **Design**: Query biomeOS registry for Storage capability provider.
    #[allow(dead_code)] // TODO: Use in storage backend setup (Phase 2.3)
    async fn discover_storage_provider(
        &self,
    ) -> Result<toadstool::biomeos_integration::PrimalInfo> {
        if let Some(ref client) = self.biomeos_client {
            match client.get_storage_provider().await {
                Ok(provider) => {
                    info!("✅ Discovered storage provider: {} at {}", provider.name, provider.endpoint);
                    return Ok(provider);
                }
                Err(e) => {
                    warn!("⚠️  Failed to discover storage provider via biomeOS: {e}");
                }
            }
        }

        // Fallback: Hardcoded localhost
        warn!("📍 Using fallback storage provider (localhost:8083)");
        Ok(toadstool::biomeos_integration::PrimalInfo {
            name: "nestgate".to_string(),
            endpoint: "http://localhost:8083".to_string(),
            capabilities: vec![],
            metadata: std::collections::HashMap::new(),
        })
    }

    // -------------------------------------------------------------------------
    // Internal implementation methods
    // -------------------------------------------------------------------------

    async fn start_biome_internal(
        &self,
        biome_name: &str,
        manifest: BiomeManifest,
        env_vars: Vec<String>,
        _detached: bool,
        _debug: bool,
        security_level: &str, // ✅ OPTIMIZED: Accept &str instead of String
    ) -> Result<BiomeInfo> {
        let biome_id = Uuid::new_v4();
        let start_time = Utc::now();

        info!("🔧 Initializing biome infrastructure");

        // Create log directory
        let log_dir = PathBuf::from(format!("/tmp/toadstool/logs/{biome_name}"));
        fs::create_dir_all(&log_dir).await?;

        // Parse environment variables
        // ✅ ZERO-COPY: Pre-allocate with known capacity
        let mut environment = HashMap::with_capacity(env_vars.len());
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
                        security_level, // Already a &str
                    )
                    .await?;
                processes.push(process);
                // ✅ OPTIMIZED: Use String literal for constant key
                log_files.insert(String::from("beardog"), log_dir.join("beardog.log"));
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
                    name: name.clone(), // Necessary - owned String needed
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
            pid: Some(1000 + (execution_id.as_u128() % 30000) as u32),
            _started_at: Utc::now(),
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
            pid: Some(2000 + (execution_id.as_u128() % 30000) as u32),
            _started_at: Utc::now(),
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
                bail!("Unsupported workload source: {source:?}");
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
                .ok_or_else(|| anyhow::anyhow!("Biome '{biome_name}' not found"))?
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

        for (_biome_name, biome) in biomes.iter() {
            for process in &biome.process_handles {
                if process.execution_id == *execution_id {
                    if let Some(pid) = process.pid {
                        info!(
                            "Gracefully stopping process {} (PID: {})",
                            execution_id, pid
                        );
                        return self.send_signal_to_process(pid, "TERM");
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

        for (_biome_name, biome) in biomes.iter() {
            for process in &biome.process_handles {
                if process.execution_id == *execution_id {
                    if let Some(pid) = process.pid {
                        info!("Force killing process {} (PID: {})", execution_id, pid);
                        return self.send_signal_to_process(pid, "KILL");
                    }
                }
            }
        }

        warn!("Process {} not found for force kill", execution_id);
        Ok(())
    }

    async fn purge_biome_data(&self, biome_name: &str) -> Result<()> {
        let data_dir = PathBuf::from(format!("/tmp/toadstool/data/{biome_name}"));
        let log_dir = PathBuf::from(format!("/tmp/toadstool/logs/{biome_name}"));

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
        log_management::show_log_file(log_file, lines, timestamps, level_filter, grep_pattern).await
    }

    async fn tail_log_file(
        &self,
        log_file: &PathBuf,
        initial_lines: usize,
        timestamps: bool,
        level_filter: Option<String>,
        grep_pattern: Option<String>,
    ) -> Result<()> {
        log_management::tail_log_file(log_file, initial_lines, timestamps, level_filter, grep_pattern).await
    }

    // Helper methods for improved functionality
    #[allow(dead_code)]
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
        Err(anyhow::anyhow!("Biome not found: {biome_name}"))
    }

    #[allow(dead_code)]
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
            .with_context(|| format!("Failed to read WASM file: {source}"))?;

        // Verify checksum if provided
        if let Some(expected_checksum) = checksum {
            let mut hasher = Sha256::new();
            hasher.update(&module_data);
            let actual_checksum = format!("{:x}", hasher.finalize());

            if actual_checksum != *expected_checksum {
                return Err(anyhow::anyhow!(
                    "WASM checksum verification failed. Expected: {expected_checksum}, Got: {actual_checksum}"
                ));
            }
        }

        Ok(module_data)
    }

    #[allow(dead_code)]
    async fn execute_wasm_module(
        &self,
        biome_name: &str,
        _module_data: Vec<u8>,
        _wasi_config: HashMap<String, String>,
    ) -> Result<()> {
        info!("Executing WASM module for biome: {}", biome_name);

        // For now, we'll just return success
        // This will be implemented when WASM runtime is integrated
        Ok(())
    }

    fn send_signal_to_process(&self, pid: u32, signal: &str) -> Result<()> {
        use std::process::Command;

        info!("Sending {} signal to PID {}", signal, pid);

        let output = Command::new("kill")
            .arg(format!("-{signal}"))
            .arg(pid.to_string())
            .output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to send signal: {error_msg}"));
        }

        Ok(())
    }
}
