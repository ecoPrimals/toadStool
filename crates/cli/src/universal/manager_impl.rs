// SPDX-License-Identifier: AGPL-3.0-only
// UniversalComputeManager Implementation
// Core implementation with all operations delegated to extension traits in operations/ module.

// Import all operation traits
use self::operations::{
    BenchmarkingOps, CapabilityDisplayOps, FederationOps, MigrationOps, PlatformDetectionOps,
    UtilityOps,
};

impl UniversalComputeManager {
    /// Create and initialize the universal compute manager
    #[expect(clippy::unused_async, reason = "async signature required by trait/interface")] // API consistency; may add async init in future
    pub async fn new() -> Result<Self> {
        info!("🌍 Initializing Universal Compute Manager");

        let detector = SubstrateDetector::new();

        Ok(Self {
            detector,
            platforms: HashMap::new(),
            benchmarks: HashMap::new(),
            federation_peers: HashMap::new(),
        })
    }

    /// Detect all available compute substrates
    pub async fn detect_platforms(
        &mut self,
        categories: Vec<String>,
        test_platforms: bool,
        output_file: Option<PathBuf>,
    ) -> Result<()> {
        info!("🔍 Detecting universal compute substrates");

        // Determine detection categories
        let detection_categories = if categories.is_empty() {
            vec![
                "traditional".to_string(),
                "container".to_string(),
                "language".to_string(),
                "gpu".to_string(),
                "quantum".to_string(),
                "edge".to_string(),
            ]
        } else {
            categories
        };

        // Perform detection
        let mut total_detected = 0;
        for category in &detection_categories {
            info!("🔍 Detecting {} platforms", category);

            let platforms = match category.as_str() {
                "traditional" => self.detector.detect_traditional_platforms().await?,
                "container" => self.detector.detect_container_platforms().await?,
                "language" => self.detector.detect_language_runtimes().await?,
                "gpu" => self.detector.detect_gpu_platforms().await?,
                "quantum" => self.detector.detect_quantum_platforms().await?,
                "edge" => self.detector.detect_edge_platforms().await?,
                "biological" => self.detector.detect_biological_platforms().await?,
                "neuromorphic" => self.detector.detect_neuromorphic_platforms().await?,
                _ => {
                    warn!("⚠️  Unknown platform category: {}", category);
                    vec![]
                }
            };

            for platform in platforms {
                let platform_id = format!("{}_{}", category, self.get_platform_id(&platform));
                let detected_platform = DetectedPlatform {
                    platform_type: platform.clone(),
                    capabilities: SubstrateCapabilities {
                        traditional_platforms: vec![platform.clone()],
                        container_platforms: Vec::new(),
                        language_runtimes: Vec::new(),
                        gpu_platforms: Vec::new(),
                        specialized_platforms: Vec::new(),
                        experimental_platforms: Vec::new(),
                    },
                    status: PlatformStatus::Available,
                    performance_score: None,
                    last_tested: None,
                    metadata: self.get_platform_metadata(&platform),
                };

                self.platforms
                    .insert(platform_id.clone(), detected_platform);
                total_detected += 1;

                info!("✅ Detected: {}", platform_id);
            }
        }

        info!("🎯 Detection complete: {} platforms found", total_detected);

        // Test platforms if requested
        if test_platforms {
            info!("🧪 Testing detected platforms");
            let platform_ids: Vec<String> = self.platforms.keys().cloned().collect();
            for platform_id in platform_ids {
                if let Some(platform) = self.platforms.get(&platform_id) {
                    match self
                        .test_platform_capabilities(&platform_id, platform)
                        .await
                    {
                        Ok(test_result) => {
                            if let Some(platform) = self.platforms.get_mut(&platform_id) {
                                platform.status = if test_result {
                                    PlatformStatus::Available
                                } else {
                                    PlatformStatus::Degraded
                                };
                                platform.last_tested = Some(std::time::SystemTime::now());
                            }
                            info!(
                                "✅ Tested: {} - {}",
                                platform_id,
                                if test_result { "PASS" } else { "DEGRADED" }
                            );
                        }
                        Err(e) => {
                            if let Some(platform) = self.platforms.get_mut(&platform_id) {
                                platform.status = PlatformStatus::Error(e.to_string());
                                platform.last_tested = Some(std::time::SystemTime::now());
                            }
                            warn!("❌ Test failed: {} - {}", platform_id, e);
                        }
                    }
                }
            }
        }

        // Save results to file if requested
        if let Some(output_path) = output_file {
            let detection_results = DetectionResults {
                timestamp: std::time::SystemTime::now(),
                total_platforms: self.platforms.len(),
                platforms: self.platforms.clone(),
                categories: detection_categories,
            };

            let json_content = serde_json::to_string_pretty(&detection_results)?;
            fs::write(&output_path, json_content).await?;
            info!("💾 Results saved to: {}", output_path.display());
        }

        // Print summary
        self.print_detection_summary().await?;

        Ok(())
    }

    /// Run comprehensive benchmarks
    pub async fn run_benchmarks(
        &mut self,
        suite: String,
        target_platforms: Vec<String>,
        output_format: String,
    ) -> Result<()> {
        info!("🏃 Running benchmark suite: {}", suite);

        // Determine platforms to benchmark
        let platforms_to_test = if target_platforms.is_empty() {
            self.platforms.keys().cloned().collect()
        } else {
            target_platforms
        };

        // Run benchmarks
        for platform_id in platforms_to_test {
            // Set platform status to testing
            if let Some(platform) = self.platforms.get_mut(&platform_id) {
                info!("📊 Benchmarking: {}", platform_id);
                platform.status = PlatformStatus::Testing;
            }

            // Run benchmark (separate from mutable borrow)
            match self.run_platform_benchmark(&platform_id, &suite).await {
                Ok(result) => {
                    // Update platform with results
                    if let Some(platform) = self.platforms.get_mut(&platform_id) {
                        platform.performance_score = Some(result.overall_score);
                        platform.status = PlatformStatus::Available;
                    }
                    self.benchmarks.insert(platform_id.clone(), result.clone());

                    info!(
                        "✅ Benchmark complete: {} - Score: {:.2}",
                        platform_id, result.overall_score
                    );
                }
                Err(e) => {
                    // Update platform with error
                    if let Some(platform) = self.platforms.get_mut(&platform_id) {
                        platform.status = PlatformStatus::Error(e.to_string());
                    }
                    error!("❌ Benchmark failed: {} - {}", platform_id, e);
                }
            }
        }

        // Output results
        match output_format.as_str() {
            "json" => {
                let json_output = serde_json::to_string_pretty(&self.benchmarks)?;
                println!("{json_output}");
            }
            "table" => {
                self.print_benchmark_table().await?;
            }
            _ => {
                self.print_benchmark_table().await?;
            }
        }

        Ok(())
    }

    /// Migrate workload between substrates
    pub async fn migrate_workload(
        &mut self,
        source: String,
        target: String,
        pause_source: bool,
        verify_migration: bool,
    ) -> Result<()> {
        info!("🚚 Migrating workload: {} → {}", source, target);

        // Validate source and target platforms
        if !self.platforms.contains_key(&source) {
            return Err(crate::CliError::Other(format!("Source platform not found: {source}")));
        }
        if !self.platforms.contains_key(&target) {
            return Err(crate::CliError::Other(format!("Target platform not found: {target}")));
        }

        // Create migration plan
        let migration_plan = self.create_migration_plan(&source, &target).await?;

        info!("📋 Migration Plan:");
        info!("   Type: {:?}", migration_plan.migration_type);
        info!(
            "   Estimated Duration: {:?}",
            migration_plan.estimated_duration
        );
        info!("   Risks: {}", migration_plan.risks.join(", "));
        info!(
            "   Requirements: {}",
            migration_plan.requirements.join(", ")
        );

        // Execute migration
        match migration_plan.migration_type {
            MigrationType::LiveMigration => {
                self.execute_live_migration(&migration_plan).await?;
            }
            MigrationType::ColdMigration => {
                if pause_source {
                    info!("⏸️  Pausing source workload");
                    self.pause_workload(&source).await?;
                }
                self.execute_cold_migration(&migration_plan).await?;
            }
            MigrationType::HotMigration => {
                self.execute_hot_migration(&migration_plan).await?;
            }
            MigrationType::CloneMigration => {
                self.execute_clone_migration(&migration_plan).await?;
            }
        }

        // Verify migration if requested
        if verify_migration {
            info!("🔍 Verifying migration");
            if self.verify_migration_success(&migration_plan).await? {
                info!("✅ Migration verification successful");
            } else {
                error!("❌ Migration verification failed");
                return Err(crate::CliError::Other("Migration verification failed".to_string()));
            }
        }

        info!("✅ Migration completed successfully");
        Ok(())
    }

    /// Federate with other `ToadStool` instances
    pub async fn establish_federation(
        &mut self,
        endpoint: String,
        _mode: String,
        shared_resources: Vec<String>,
    ) -> Result<()> {
        info!("🤝 Establishing federation with: {}", endpoint);

        // Parse endpoint
        let peer_addr: SocketAddr = endpoint
            .parse()
            .context(format!("Invalid federation endpoint: {endpoint}"))?;

        // Create federation request
        let federation_request = operations::federation::FederationRequest {
            peer_id: uuid::Uuid::new_v4(),
            mode: std::sync::Arc::from("standard"),
            capabilities: self.get_local_capabilities(),
            shared_resources: shared_resources
                .iter()
                .map(|s| std::sync::Arc::from(s.as_str()))
                .collect(),
            protocol_version: std::sync::Arc::from("1.0"),
        };

        // Attempt connection
        match self.connect_to_peer(&peer_addr, &federation_request).await {
            Ok(response) => {
                info!("✅ Federation established");
                info!("   Remote Peer ID: {}", response.peer_id);
                info!("   Protocol Version: {}", response.protocol_version);
                info!(
                    "   Shared Resources: {}",
                    response.accepted_resources.join(", ")
                );

                // Store peer connection
                let peer = FederationPeer {
                    peer_id: response.peer_id,
                    endpoint: peer_addr,
                    capabilities: response.capabilities.clone(), // Arc<str> clone is cheap
                    shared_resources: response.accepted_resources.clone(), // Arc<str> clone is cheap
                    status: FederationStatus::Connected,
                    last_heartbeat: std::time::SystemTime::now(),
                    trust_level: TrustLevel::Verified,
                };

                self.federation_peers.insert(peer_addr.to_string(), peer);

                // Start heartbeat monitoring
                self.start_peer_monitoring(&peer_addr).await?;

                Ok(())
            }
            Err(e) => {
                error!("❌ Federation failed: {}", e);
                Err(e)
            }
        }
    }

    /// Show universal compute capabilities
    pub async fn show_capabilities(&self, format: &str, detailed: bool) -> Result<()> {
        match format {
            "json" => {
                let capabilities = UniversalCapabilities {
                    platforms: self.platforms.clone(),
                    benchmarks: self.benchmarks.clone(),
                    federation_peers: self.federation_peers.clone(),
                    total_platforms: self.platforms.len(),
                    available_platforms: self
                        .platforms
                        .values()
                        .filter(|p| matches!(p.status, PlatformStatus::Available))
                        .count(),
                };
                println!("{}", serde_json::to_string_pretty(&capabilities)?);
            }
            "yaml" => {
                let capabilities = UniversalCapabilities {
                    platforms: self.platforms.clone(),
                    benchmarks: self.benchmarks.clone(),
                    federation_peers: self.federation_peers.clone(),
                    total_platforms: self.platforms.len(),
                    available_platforms: self
                        .platforms
                        .values()
                        .filter(|p| matches!(p.status, PlatformStatus::Available))
                        .count(),
                };
                println!("{}", serde_yaml_ng::to_string(&capabilities)?);
            }
            "table" => {
                self.print_capabilities_table(detailed).await?;
            }
            _ => {
                self.print_capabilities_table(detailed).await?;
            }
        }

        Ok(())
    }
}

// Helper structs for serialization
#[derive(Debug, Serialize)]
struct DetectionResults {
    #[serde(with = "toadstool_common::system_time_serde")]
    timestamp: std::time::SystemTime,
    total_platforms: usize,
    platforms: HashMap<String, DetectedPlatform>,
    categories: Vec<String>,
}

#[derive(Debug, Serialize)]
struct UniversalCapabilities {
    platforms: HashMap<String, DetectedPlatform>,
    benchmarks: HashMap<String, BenchmarkResult>,
    federation_peers: HashMap<String, FederationPeer>,
    total_platforms: usize,
    available_platforms: usize,
}
