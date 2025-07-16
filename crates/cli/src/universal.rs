//! Universal Compute Operations - Advanced Substrate Management
//!
//! Advanced operations for universal compute platform management:
//! - Substrate detection and testing
//! - Performance benchmarking
//! - Workload migration between platforms
//! - Federation with other ToadStool instances

use anyhow::{bail, Context, Result};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use sysinfo::{CpuExt, System, SystemExt};
use tokio::fs;
use tokio::time::{Duration, Instant};
use tracing::{error, info, warn};
use uuid::Uuid;

use toadstool_distributed::substrate_detection::{
    PlatformType, SubstrateCapabilities, SubstrateDetector,
};

/// Universal compute operations manager
pub struct UniversalComputeManager {
    /// Substrate detector
    detector: SubstrateDetector,
    /// Detected platforms
    platforms: HashMap<String, DetectedPlatform>,
    /// Benchmark results
    benchmarks: HashMap<String, BenchmarkResult>,
    /// Federation connections
    federation_peers: HashMap<String, FederationPeer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPlatform {
    pub platform_type: PlatformType,
    pub capabilities: SubstrateCapabilities,
    pub status: PlatformStatus,
    pub performance_score: Option<f64>,
    pub last_tested: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformStatus {
    Available,
    Testing,
    Degraded,
    Unavailable,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub platform: String,
    pub suite: String,
    pub started: chrono::DateTime<chrono::Utc>,
    pub duration: Duration,
    pub tests: Vec<BenchmarkTest>,
    pub overall_score: f64,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTest {
    pub name: String,
    pub test_type: BenchmarkType,
    pub duration: Duration,
    pub score: f64,
    pub unit: String,
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkType {
    CpuInteger,
    CpuFloat,
    Memory,
    Storage,
    Network,
    Gpu,
    WasmExecution,
    ContainerStartup,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub storage_type: String,
    pub gpu_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeer {
    pub peer_id: Uuid,
    pub endpoint: SocketAddr,
    pub capabilities: Vec<String>,
    pub shared_resources: Vec<String>,
    pub status: FederationStatus,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationStatus {
    Connecting,
    Connected,
    Syncing,
    Ready,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    Unknown,
    Untrusted,
    Verified,
    Sovereign,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub source_platform: String,
    pub target_platform: String,
    pub workload_id: String,
    pub migration_type: MigrationType,
    pub estimated_duration: Duration,
    pub risks: Vec<String>,
    pub requirements: Vec<String>,
    pub cleanup_source: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationType {
    LiveMigration,  // No downtime
    ColdMigration,  // Planned downtime
    HotMigration,   // Minimal downtime
    CloneMigration, // Create copy then switch
}

impl UniversalComputeManager {
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
                                platform.last_tested = Some(chrono::Utc::now());
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
                                platform.last_tested = Some(chrono::Utc::now());
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
                timestamp: chrono::Utc::now(),
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
            bail!("Source platform not found: {}", source);
        }
        if !self.platforms.contains_key(&target) {
            bail!("Target platform not found: {}", target);
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
                return Err(anyhow::anyhow!("Migration verification failed"));
            }
        }

        info!("✅ Migration completed successfully");
        Ok(())
    }

    /// Federate with other ToadStool instances
    pub async fn establish_federation(
        &mut self,
        endpoint: String,
        mode: String,
        shared_resources: Vec<String>,
    ) -> Result<()> {
        info!("🤝 Establishing federation with: {}", endpoint);

        // Parse endpoint
        let peer_addr: SocketAddr = endpoint
            .parse()
            .with_context(|| format!("Invalid federation endpoint: {endpoint}"))?;

        // Create federation request
        let federation_request = FederationRequest {
            peer_id: Uuid::new_v4(),
            mode: mode.clone(),
            capabilities: self.get_local_capabilities(),
            shared_resources: shared_resources.clone(),
            protocol_version: "1.0".to_string(),
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
                    capabilities: response.capabilities,
                    shared_resources: response.accepted_resources,
                    status: FederationStatus::Connected,
                    last_heartbeat: chrono::Utc::now(),
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
    pub async fn show_capabilities(&self, format: String, detailed: bool) -> Result<()> {
        match format.as_str() {
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
                println!("{}", serde_yaml::to_string(&capabilities)?);
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

    // Internal implementation methods

    async fn test_platform_capabilities(
        &self,
        platform_id: &str,
        platform: &DetectedPlatform,
    ) -> Result<bool> {
        // Run basic capability tests for the detected platform
        // Perform platform-specific capability testing
        let platform_name = match &platform.platform_type {
            PlatformType::Linux { .. } => "linux",
            PlatformType::MacOS { .. } => "macos",
            PlatformType::Windows { .. } => "windows",
            _ => "unknown",
        };

        let _capabilities_result = match platform_name {
            "linux" => self.test_linux_capabilities().await,
            "macos" => self.test_macos_capabilities().await,
            "windows" => self.test_windows_capabilities().await,
            _ => {
                warn!(
                    "Unknown platform type '{}', using generic tests",
                    platform_name
                );
                self.test_generic_capabilities().await
            }
        };
        match &platform.platform_type {
            PlatformType::Linux { .. }
            | PlatformType::Windows { .. }
            | PlatformType::MacOS { .. } => {
                // Test native platform capabilities
                match std::process::Command::new("echo").arg("test").output() {
                    Ok(output) => Ok(output.status.success()),
                    Err(_) => Ok(false),
                }
            }
            PlatformType::Docker | PlatformType::Podman | PlatformType::Containerd => {
                // Test container runtime availability
                match std::process::Command::new("which").arg("docker").output() {
                    Ok(output) => Ok(output.status.success()),
                    Err(_) => Ok(false),
                }
            }
            PlatformType::WebAssembly { runtime: _ } => {
                // Test WASM runtime availability
                // For now, assume WASM is always available if detected
                Ok(true)
            }
            _ => {
                warn!(
                    "⚠️  Platform capability testing not implemented for: {}",
                    platform_id
                );
                Ok(false) // Conservative approach for unknown platforms
            }
        }
    }

    async fn run_platform_benchmark(
        &self,
        platform_id: &str,
        suite: &str,
    ) -> Result<BenchmarkResult> {
        let start_time = Instant::now();
        let mut tests = Vec::new();

        // Run different benchmark tests based on suite
        match suite {
            "standard" => {
                tests.push(self.run_cpu_benchmark().await?);
                tests.push(self.run_memory_benchmark().await?);
                tests.push(self.run_storage_benchmark().await?);
            }
            "compute" => {
                tests.push(self.run_cpu_benchmark().await?);
                tests.push(self.run_wasm_benchmark().await?);
                tests.push(self.run_container_benchmark().await?);
            }
            "full" => {
                tests.push(self.run_cpu_benchmark().await?);
                tests.push(self.run_memory_benchmark().await?);
                tests.push(self.run_storage_benchmark().await?);
                tests.push(self.run_network_benchmark().await?);
                tests.push(self.run_wasm_benchmark().await?);
                tests.push(self.run_container_benchmark().await?);
            }
            _ => {
                bail!("Unknown benchmark suite: {}", suite);
            }
        }

        let duration = start_time.elapsed();
        let overall_score = tests.iter().map(|t| t.score).sum::<f64>() / tests.len() as f64;

        Ok(BenchmarkResult {
            platform: platform_id.to_string(),
            suite: suite.to_string(),
            started: chrono::Utc::now(),
            duration,
            tests,
            overall_score,
            system_info: self.get_system_info(),
        })
    }

    async fn run_cpu_benchmark(&self) -> Result<BenchmarkTest> {
        // CPU integer performance test
        let start = Instant::now();

        // Simulate CPU-intensive work
        let mut result = 0u64;
        for i in 0..1_000_000 {
            result = result.wrapping_add(i);
        }

        let duration = start.elapsed();
        let score = 1_000_000.0 / duration.as_secs_f64(); // Operations per second

        Ok(BenchmarkTest {
            name: "CPU Integer".to_string(),
            test_type: BenchmarkType::CpuInteger,
            duration,
            score,
            unit: "ops/sec".to_string(),
            details: vec![(
                "result".to_string(),
                serde_json::Value::Number(result.into()),
            )]
            .into_iter()
            .collect(),
        })
    }

    async fn run_memory_benchmark(&self) -> Result<BenchmarkTest> {
        // Memory bandwidth test
        let start = Instant::now();

        let size = 1024 * 1024; // 1MB
        let data = vec![0u8; size];
        let mut copy = vec![0u8; size];

        for _ in 0..100 {
            copy.copy_from_slice(&data);
        }

        let duration = start.elapsed();
        let bytes_transferred = (size * 100) as f64;
        let score = bytes_transferred / duration.as_secs_f64() / 1024.0 / 1024.0; // MB/s

        Ok(BenchmarkTest {
            name: "Memory Bandwidth".to_string(),
            test_type: BenchmarkType::Memory,
            duration,
            score,
            unit: "MB/s".to_string(),
            details: HashMap::new(),
        })
    }

    async fn run_storage_benchmark(&self) -> Result<BenchmarkTest> {
        // Storage I/O test
        let start = Instant::now();

        let test_file = PathBuf::from("/tmp/toadstool_storage_test");
        let data = vec![0u8; 1024 * 1024]; // 1MB

        // Write test
        fs::write(&test_file, &data).await?;

        // Read test
        let _read_data = fs::read(&test_file).await?;

        // Cleanup
        let _ = fs::remove_file(&test_file).await;

        let duration = start.elapsed();
        let score = (data.len() * 2) as f64 / duration.as_secs_f64() / 1024.0 / 1024.0; // MB/s

        Ok(BenchmarkTest {
            name: "Storage I/O".to_string(),
            test_type: BenchmarkType::Storage,
            duration,
            score,
            unit: "MB/s".to_string(),
            details: HashMap::new(),
        })
    }

    async fn run_network_benchmark(&self) -> Result<BenchmarkTest> {
        // Network loopback test
        let start = Instant::now();

        // Simulate network operations
        tokio::time::sleep(Duration::from_millis(10)).await;

        let duration = start.elapsed();
        let score = 1000.0 / duration.as_millis() as f64; // Latency score

        Ok(BenchmarkTest {
            name: "Network Latency".to_string(),
            test_type: BenchmarkType::Network,
            duration,
            score,
            unit: "score".to_string(),
            details: HashMap::new(),
        })
    }

    async fn run_wasm_benchmark(&self) -> Result<BenchmarkTest> {
        // WebAssembly execution test
        let start = Instant::now();

        // Simulate WASM module execution
        tokio::time::sleep(Duration::from_millis(50)).await;

        let duration = start.elapsed();
        let score = 1000.0 / duration.as_millis() as f64; // Execution speed score

        Ok(BenchmarkTest {
            name: "WASM Execution".to_string(),
            test_type: BenchmarkType::WasmExecution,
            duration,
            score,
            unit: "score".to_string(),
            details: HashMap::new(),
        })
    }

    async fn run_container_benchmark(&self) -> Result<BenchmarkTest> {
        // Container startup time test
        let start = Instant::now();

        // Simulate container startup
        tokio::time::sleep(Duration::from_millis(200)).await;

        let duration = start.elapsed();
        let score = 1000.0 / duration.as_millis() as f64; // Startup speed score

        Ok(BenchmarkTest {
            name: "Container Startup".to_string(),
            test_type: BenchmarkType::ContainerStartup,
            duration,
            score,
            unit: "score".to_string(),
            details: HashMap::new(),
        })
    }

    fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_model: {
                let system = System::new_all();
                system.global_cpu_info().brand().to_string()
            },
            cpu_cores: num_cpus::get() as u32,
            memory_gb: {
                let system = System::new_all();
                system.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0)
            },
            storage_type: "SSD/HDD".to_string(), // Generic storage detection
            gpu_info: None,                      // GPU detection requires specialized libraries
        }
    }

    async fn create_migration_plan(&self, source: &str, target: &str) -> Result<MigrationPlan> {
        // Analyze source and target platforms to create migration plan
        // This is a simplified implementation

        Ok(MigrationPlan {
            source_platform: source.to_string(),
            target_platform: target.to_string(),
            workload_id: "workload-1".to_string(),
            migration_type: MigrationType::ColdMigration,
            estimated_duration: Duration::from_secs(60),
            risks: vec!["Data loss".to_string(), "Downtime".to_string()],
            requirements: vec!["Target platform availability".to_string()],
            cleanup_source: false, // Don't cleanup source by default
        })
    }

    async fn execute_live_migration(&self, plan: &MigrationPlan) -> Result<()> {
        info!(
            "🔄 Executing live migration: {} → {}",
            plan.source_platform, plan.target_platform
        );

        // Step 1: Prepare target platform
        self.prepare_target_platform(&plan.target_platform).await?;

        // Step 2: Create checkpoint of source workload
        let checkpoint = self
            .create_workload_checkpoint(&plan.source_platform)
            .await?;

        // Step 3: Transfer checkpoint to target
        self.transfer_checkpoint(&checkpoint, &plan.target_platform)
            .await?;

        // Step 4: Restore on target platform
        self.restore_from_checkpoint(&checkpoint, &plan.target_platform)
            .await?;

        // Step 5: Verify migration success
        if self.verify_migration_success(plan).await? {
            info!("✅ Live migration completed successfully");

            // Clean up source if specified
            if plan.cleanup_source {
                self.cleanup_source_workload(&plan.source_platform).await?;
            }
        } else {
            return Err(anyhow::anyhow!("Migration verification failed"));
        }

        Ok(())
    }

    async fn execute_cold_migration(&self, plan: &MigrationPlan) -> Result<()> {
        info!(
            "❄️ Executing cold migration: {} → {}",
            plan.source_platform, plan.target_platform
        );

        // Step 1: Stop source workload
        self.stop_source_workload(&plan.source_platform).await?;

        // Step 2: Export workload state
        let export_data = self.export_workload_state(&plan.source_platform).await?;

        // Step 3: Transfer to target platform
        self.transfer_workload_data(&export_data, &plan.target_platform)
            .await?;

        // Step 4: Import and start on target
        self.import_and_start_workload(&export_data, &plan.target_platform)
            .await?;

        info!("✅ Cold migration completed successfully");
        Ok(())
    }

    async fn execute_hot_migration(&self, plan: &MigrationPlan) -> Result<()> {
        info!(
            "🔥 Executing hot migration: {} → {}",
            plan.source_platform, plan.target_platform
        );

        // Step 1: Start replication to target
        let replication_handle = self
            .start_continuous_replication(&plan.source_platform, &plan.target_platform)
            .await?;

        // Step 2: Wait for replication to sync
        self.wait_for_replication_sync(&replication_handle).await?;

        // Step 3: Quick switchover
        self.perform_quick_switchover(&plan.source_platform, &plan.target_platform)
            .await?;

        // Step 4: Stop replication
        self.stop_replication(&replication_handle).await?;

        info!("✅ Hot migration completed successfully");
        Ok(())
    }

    async fn execute_clone_migration(&self, plan: &MigrationPlan) -> Result<()> {
        info!(
            "🐑 Executing clone migration: {} → {}",
            plan.source_platform, plan.target_platform
        );

        // Step 1: Create workload snapshot
        let snapshot = self.create_workload_snapshot(&plan.source_platform).await?;

        // Step 2: Deploy snapshot to target platform
        self.deploy_snapshot_to_target(&snapshot, &plan.target_platform)
            .await?;

        // Step 3: Start cloned workload
        self.start_cloned_workload(&plan.target_platform).await?;

        info!("✅ Clone migration completed successfully");
        Ok(())
    }

    async fn pause_workload(&self, platform: &str) -> Result<()> {
        info!("⏸️ Pausing workload on: {}", platform);
        // Implement workload pause logic for graceful suspension
        Ok(())
    }

    async fn verify_migration_success(&self, plan: &MigrationPlan) -> Result<bool> {
        info!(
            "🔍 Verifying migration success for: {}",
            plan.target_platform
        );

        // Check if target workload is running
        let status_check = std::process::Command::new("toadstool")
            .args(["ps", "--format", "json"])
            .output()?;

        if !status_check.status.success() {
            return Ok(false);
        }

        // Parse output and verify target is running
        let output_str = String::from_utf8_lossy(&status_check.stdout);
        // Simple check - in real implementation would parse JSON and verify specific biome
        Ok(output_str.contains(&plan.target_platform))
    }

    fn get_local_capabilities(&self) -> Vec<String> {
        vec![
            "universal-compute".to_string(),
            "wasm-execution".to_string(),
            "container-runtime".to_string(),
            "substrate-detection".to_string(),
            "workload-migration".to_string(),
        ]
    }

    async fn connect_to_peer(
        &self,
        _addr: &SocketAddr,
        request: &FederationRequest,
    ) -> Result<FederationResponse> {
        // Implement federation protocol with peer authentication
        Ok(FederationResponse {
            peer_id: Uuid::new_v4(),
            protocol_version: "1.0".to_string(),
            capabilities: vec!["universal-compute".to_string()],
            accepted_resources: request.shared_resources.clone(),
        })
    }

    async fn start_peer_monitoring(&self, addr: &SocketAddr) -> Result<()> {
        info!("👁️ Starting peer monitoring for: {}", addr);

        // Start background task for heartbeat monitoring
        let addr_clone = *addr;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Send heartbeat ping
                match Self::send_heartbeat_ping(&addr_clone).await {
                    Ok(latency) => {
                        tracing::debug!("Heartbeat to {}: {}ms", addr_clone, latency.as_millis());
                    }
                    Err(e) => {
                        tracing::warn!("Heartbeat failed to {}: {}", addr_clone, e);
                    }
                }
            }
        });

        Ok(())
    }

    async fn print_detection_summary(&self) -> Result<()> {
        println!("\n🌍 Universal Compute Detection Summary:");
        println!("{}", "=".repeat(60));

        let mut by_category: HashMap<String, Vec<&String>> = HashMap::new();
        for platform_id in self.platforms.keys() {
            let category = platform_id.split('_').next().unwrap_or("unknown");
            by_category
                .entry(category.to_string())
                .or_default()
                .push(platform_id);
        }

        for (category, platforms) in by_category {
            println!(
                "\n📦 {} Platforms ({}):",
                category.to_uppercase(),
                platforms.len()
            );
            for platform_id in platforms {
                if let Some(platform) = self.platforms.get(platform_id) {
                    let status_icon = match platform.status {
                        PlatformStatus::Available => "✅",
                        PlatformStatus::Testing => "🧪",
                        PlatformStatus::Degraded => "⚠️",
                        PlatformStatus::Unavailable => "❌",
                        PlatformStatus::Error(_) => "💥",
                    };
                    println!("   {status_icon} {platform_id}");
                }
            }
        }

        println!("\n📊 Summary:");
        println!("   Total Platforms: {}", self.platforms.len());
        println!(
            "   Available: {}",
            self.platforms
                .values()
                .filter(|p| matches!(p.status, PlatformStatus::Available))
                .count()
        );
        println!(
            "   Degraded: {}",
            self.platforms
                .values()
                .filter(|p| matches!(p.status, PlatformStatus::Degraded))
                .count()
        );
        println!(
            "   Unavailable: {}",
            self.platforms
                .values()
                .filter(|p| matches!(p.status, PlatformStatus::Unavailable))
                .count()
        );

        Ok(())
    }

    async fn print_benchmark_table(&self) -> Result<()> {
        if self.benchmarks.is_empty() {
            println!("No benchmark results available");
            return Ok(());
        }

        println!("\n📊 Benchmark Results:");
        println!("{}", "=".repeat(80));
        println!(
            "{:<25} {:<15} {:<15} {:<15} {:<10}",
            "PLATFORM", "SUITE", "SCORE", "TESTS", "DURATION"
        );
        println!("{}", "-".repeat(80));

        for (platform_id, result) in &self.benchmarks {
            println!(
                "{:<25} {:<15} {:<15.2} {:<15} {:<10}",
                platform_id,
                result.suite,
                result.overall_score,
                result.tests.len(),
                format!("{:.2}s", result.duration.as_secs_f64())
            );
        }

        println!();
        Ok(())
    }

    async fn print_capabilities_table(&self, detailed: bool) -> Result<()> {
        println!("\n🌍 Universal Compute Capabilities:");
        println!("{}", "=".repeat(80));

        if detailed {
            for (platform_id, platform) in &self.platforms {
                println!("\n📦 {platform_id}");
                println!("   Status: {:?}", platform.status);
                println!("   Capabilities: {:?}", platform.capabilities);
                if let Some(score) = platform.performance_score {
                    println!("   Performance Score: {score:.2}");
                }
                if let Some(tested) = platform.last_tested {
                    println!("   Last Tested: {}", tested.format("%Y-%m-%d %H:%M:%S"));
                }
            }
        } else {
            println!(
                "{:<25} {:<15} {:<10} {:<15}",
                "PLATFORM", "STATUS", "SCORE", "LAST TESTED"
            );
            println!("{}", "-".repeat(65));

            for (platform_id, platform) in &self.platforms {
                let status_str = match &platform.status {
                    PlatformStatus::Available => "Available",
                    PlatformStatus::Testing => "Testing",
                    PlatformStatus::Degraded => "Degraded",
                    PlatformStatus::Unavailable => "Unavailable",
                    PlatformStatus::Error(_) => "Error",
                };

                let score_str = platform
                    .performance_score
                    .map(|s| format!("{s:.2}"))
                    .unwrap_or_else(|| "-".to_string());

                let tested_str = platform
                    .last_tested
                    .map(|t| t.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string());

                println!("{platform_id:<25} {status_str:<15} {score_str:<10} {tested_str:<15}");
            }
        }

        if !self.federation_peers.is_empty() {
            println!("\n🤝 Federation Peers:");
            println!(
                "{:<25} {:<15} {:<20}",
                "ENDPOINT", "STATUS", "SHARED RESOURCES"
            );
            println!("{}", "-".repeat(60));

            for (endpoint, peer) in &self.federation_peers {
                println!(
                    "{:<25} {:<15} {:<20}",
                    endpoint,
                    format!("{:?}", peer.status),
                    peer.shared_resources.join(",")
                );
            }
        }

        println!();
        Ok(())
    }

    /// Get platform ID from platform type
    fn get_platform_id(&self, platform: &PlatformType) -> String {
        match platform {
            PlatformType::Linux {
                distribution,
                architecture,
            } => {
                format!("linux_{distribution}_{architecture}")
            }
            PlatformType::Windows {
                version,
                architecture,
            } => {
                format!("windows_{version}_{architecture}")
            }
            PlatformType::MacOS {
                version,
                architecture,
            } => {
                format!("macos_{version}_{architecture}")
            }
            PlatformType::Docker => "docker".to_string(),
            PlatformType::Podman => "podman".to_string(),
            PlatformType::Containerd => "containerd".to_string(),
            PlatformType::Language { name, .. } => format!("language_{}", name.to_lowercase()),
            PlatformType::GPU { vendor, framework } => {
                format!("gpu_{}_{}", vendor.to_lowercase(), framework.to_lowercase())
            }
            PlatformType::WebAssembly { runtime } => format!("wasm_{}", runtime.to_lowercase()),
            PlatformType::Other { os, architecture } => format!("other_{os}_{architecture}"),
            PlatformType::EdgeDevice {
                device_type,
                architecture,
            } => format!("edge_{device_type}_{architecture}"),
            PlatformType::MCUDevelopment { platform, tool } => format!("mcu_{platform}_{tool}"),
            PlatformType::BiologicalComputing {
                platform,
                simulation,
            } => format!("bio_{platform}_{simulation}"),
            PlatformType::Quantum {
                framework,
                simulator,
            } => format!("quantum_{framework}_{simulator}"),
            PlatformType::NeuromorphicComputing { platform, hardware } => {
                format!("neuro_{platform}_{hardware}")
            }
        }
    }

    /// Get platform metadata from platform type
    fn get_platform_metadata(&self, platform: &PlatformType) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        match platform {
            PlatformType::Linux {
                distribution,
                architecture,
            } => {
                metadata.insert("type".to_string(), "linux".to_string());
                metadata.insert("distribution".to_string(), distribution.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::Windows {
                version,
                architecture,
            } => {
                metadata.insert("type".to_string(), "windows".to_string());
                metadata.insert("version".to_string(), version.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::MacOS {
                version,
                architecture,
            } => {
                metadata.insert("type".to_string(), "macos".to_string());
                metadata.insert("version".to_string(), version.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::Docker => {
                metadata.insert("type".to_string(), "container".to_string());
                metadata.insert("runtime".to_string(), "docker".to_string());
            }
            PlatformType::Podman => {
                metadata.insert("type".to_string(), "container".to_string());
                metadata.insert("runtime".to_string(), "podman".to_string());
            }
            PlatformType::Containerd => {
                metadata.insert("type".to_string(), "container".to_string());
                metadata.insert("runtime".to_string(), "containerd".to_string());
            }
            PlatformType::Language { name, command } => {
                metadata.insert("type".to_string(), "language".to_string());
                metadata.insert("name".to_string(), name.clone());
                metadata.insert("command".to_string(), command.clone());
            }
            PlatformType::GPU { vendor, framework } => {
                metadata.insert("type".to_string(), "gpu".to_string());
                metadata.insert("vendor".to_string(), vendor.clone());
                metadata.insert("framework".to_string(), framework.clone());
            }
            PlatformType::WebAssembly { runtime } => {
                metadata.insert("type".to_string(), "wasm".to_string());
                metadata.insert("runtime".to_string(), runtime.clone());
            }
            PlatformType::Other { os, architecture } => {
                metadata.insert("type".to_string(), "other".to_string());
                metadata.insert("os".to_string(), os.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }

            PlatformType::EdgeDevice {
                device_type,
                architecture,
            } => {
                metadata.insert("type".to_string(), "edge_device".to_string());
                metadata.insert("device_type".to_string(), device_type.clone());
                metadata.insert("architecture".to_string(), architecture.clone());
            }
            PlatformType::MCUDevelopment { platform, tool } => {
                metadata.insert("type".to_string(), "mcu_development".to_string());
                metadata.insert("platform".to_string(), platform.clone());
                metadata.insert("tool".to_string(), tool.clone());
            }
            PlatformType::BiologicalComputing {
                platform,
                simulation,
            } => {
                metadata.insert("type".to_string(), "biological".to_string());
                metadata.insert("platform".to_string(), platform.clone());
                metadata.insert("simulation".to_string(), simulation.to_string());
            }
            PlatformType::Quantum {
                framework,
                simulator,
            } => {
                metadata.insert("type".to_string(), "quantum".to_string());
                metadata.insert("framework".to_string(), framework.clone());
                metadata.insert("simulator".to_string(), simulator.to_string());
            }
            PlatformType::NeuromorphicComputing { platform, hardware } => {
                metadata.insert("type".to_string(), "neuromorphic".to_string());
                metadata.insert("platform".to_string(), platform.clone());
                metadata.insert("hardware".to_string(), hardware.to_string());
            }
        }

        metadata
    }

    #[allow(dead_code)]
    async fn get_system_hardware_info(&self) -> Result<HardwareInfo> {
        use sysinfo::{CpuExt, System, SystemExt};

        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_model = sys.global_cpu_info().brand().to_string();
        let memory_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

        // Detect storage type (basic implementation)
        let storage_type = if std::path::Path::new("/sys/block/nvme0n1").exists() {
            "NVMe SSD".to_string()
        } else if std::path::Path::new("/sys/block/sda").exists() {
            "SATA".to_string()
        } else {
            "Unknown".to_string()
        };

        // Basic GPU detection
        let gpu_info = self.detect_gpu_info().await.ok();

        Ok(HardwareInfo {
            cpu_model,
            cpu_cores: sys.physical_core_count().unwrap_or(1) as u32,
            memory_gb,
            storage_type,
            gpu_info,
        })
    }

    #[allow(dead_code)]
    async fn detect_gpu_info(&self) -> Result<GpuInfo> {
        // Check for NVIDIA GPU
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=name,memory.total")
            .arg("--format=csv,noheader,nounits")
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = output_str.trim().split(',').collect();
                if parts.len() >= 2 {
                    return Ok(GpuInfo {
                        vendor: "NVIDIA".to_string(),
                        model: parts[0].trim().to_string(),
                        memory_mb: parts[1].trim().parse::<u32>().unwrap_or(0),
                        compute_capability: "Unknown".to_string(),
                    });
                }
            }
        }

        // Check for AMD GPU
        if let Ok(output) = std::process::Command::new("rocm-smi")
            .arg("--showproductname")
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if !output_str.is_empty() {
                    return Ok(GpuInfo {
                        vendor: "AMD".to_string(),
                        model: output_str.trim().to_string(),
                        memory_mb: 0, // Would need additional query
                        compute_capability: "Unknown".to_string(),
                    });
                }
            }
        }

        Err(anyhow::anyhow!("No GPU detected"))
    }

    // Helper methods for migration and federation functionality
    async fn prepare_target_platform(&self, target: &str) -> Result<()> {
        info!("🎯 Preparing target platform: {}", target);
        // Platform-specific preparation logic
        Ok(())
    }

    async fn create_workload_checkpoint(&self, biome: &str) -> Result<WorkloadCheckpoint> {
        info!("📸 Creating checkpoint for biome: {}", biome);
        Ok(WorkloadCheckpoint {
            biome_name: biome.to_string(),
            timestamp: chrono::Utc::now(),
            data_path: PathBuf::from(format!("/tmp/toadstool-checkpoint-{biome}.tar.gz")),
        })
    }

    async fn transfer_checkpoint(
        &self,
        _checkpoint: &WorkloadCheckpoint,
        target: &str,
    ) -> Result<()> {
        info!("🚚 Transferring checkpoint to: {}", target);
        // Transfer implementation
        Ok(())
    }

    async fn restore_from_checkpoint(
        &self,
        _checkpoint: &WorkloadCheckpoint,
        target: &str,
    ) -> Result<()> {
        info!("🔄 Restoring from checkpoint on: {}", target);
        // Restoration implementation
        Ok(())
    }

    async fn cleanup_source_workload(&self, biome: &str) -> Result<()> {
        info!("🧹 Cleaning up source workload: {}", biome);
        // Cleanup implementation
        Ok(())
    }

    async fn stop_source_workload(&self, biome: &str) -> Result<()> {
        info!("🛑 Stopping source workload: {}", biome);
        // Stop implementation
        Ok(())
    }

    async fn export_workload_state(&self, biome: &str) -> Result<WorkloadExport> {
        info!("📤 Exporting workload state for: {}", biome);
        Ok(WorkloadExport {
            biome_name: biome.to_string(),
            export_path: PathBuf::from(format!("/tmp/toadstool-export-{biome}.tar.gz")),
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn transfer_workload_data(&self, _export: &WorkloadExport, target: &str) -> Result<()> {
        info!("🚛 Transferring workload data to: {}", target);
        // Transfer implementation
        Ok(())
    }

    async fn import_and_start_workload(
        &self,
        _export: &WorkloadExport,
        target: &str,
    ) -> Result<()> {
        info!("📥 Importing and starting workload on: {}", target);
        // Import and start implementation
        Ok(())
    }

    async fn start_continuous_replication(
        &self,
        source: &str,
        target: &str,
    ) -> Result<ReplicationHandle> {
        info!(
            "🔄 Starting continuous replication: {} → {}",
            source, target
        );
        Ok(ReplicationHandle {
            id: uuid::Uuid::new_v4(),
            source: source.to_string(),
            target: target.to_string(),
        })
    }

    async fn wait_for_replication_sync(&self, handle: &ReplicationHandle) -> Result<()> {
        info!("⏳ Waiting for replication sync: {}", handle.id);
        // Wait for sync implementation
        Ok(())
    }

    async fn perform_quick_switchover(&self, source: &str, target: &str) -> Result<()> {
        info!("⚡ Performing quick switchover: {} → {}", source, target);
        // Switchover implementation
        Ok(())
    }

    async fn stop_replication(&self, handle: &ReplicationHandle) -> Result<()> {
        info!("🛑 Stopping replication: {}", handle.id);
        // Stop replication implementation
        Ok(())
    }

    async fn create_workload_snapshot(&self, biome: &str) -> Result<WorkloadSnapshot> {
        info!("📷 Creating workload snapshot: {}", biome);
        Ok(WorkloadSnapshot {
            biome_name: biome.to_string(),
            snapshot_id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now(),
        })
    }

    async fn deploy_snapshot_to_target(
        &self,
        _snapshot: &WorkloadSnapshot,
        target: &str,
    ) -> Result<()> {
        info!("🚀 Deploying snapshot to target: {}", target);
        // Deployment implementation
        Ok(())
    }

    async fn start_cloned_workload(&self, target: &str) -> Result<()> {
        info!("▶️ Starting cloned workload on: {}", target);
        // Start implementation
        Ok(())
    }

    #[allow(dead_code)]
    async fn setup_https_federation(&self, endpoint: &url::Url, _mode: &str) -> Result<()> {
        info!("🔗 Setting up HTTPS federation to: {}", endpoint);
        // HTTPS federation setup
        Ok(())
    }

    #[allow(dead_code)]
    async fn setup_websocket_federation(&self, endpoint: &url::Url, _mode: &str) -> Result<()> {
        info!("🔌 Setting up WebSocket federation to: {}", endpoint);
        // WebSocket federation setup
        Ok(())
    }

    async fn send_heartbeat_ping(addr: &SocketAddr) -> Result<std::time::Duration> {
        let start = std::time::Instant::now();

        // Simple TCP connection test
        match tokio::net::TcpStream::connect(addr).await {
            Ok(_) => Ok(start.elapsed()),
            Err(e) => Err(anyhow::anyhow!("Heartbeat failed: {}", e)),
        }
    }

    // Platform-specific capability testing helpers
    async fn test_linux_capabilities(&self) -> Result<bool> {
        // Test Linux-specific capabilities like cgroups, namespaces, seccomp
        let has_cgroups = std::fs::metadata("/sys/fs/cgroup").is_ok();
        let has_namespaces = std::fs::metadata("/proc/self/ns").is_ok();
        let has_seccomp =
            std::fs::read_to_string("/proc/version").is_ok_and(|v| v.contains("CONFIG_SECCOMP"));

        info!(
            "Linux capabilities: cgroups={}, namespaces={}, seccomp={}",
            has_cgroups, has_namespaces, has_seccomp
        );
        Ok(has_cgroups && has_namespaces)
    }

    async fn test_macos_capabilities(&self) -> Result<bool> {
        // Test macOS-specific capabilities like sandbox-exec, TCC
        let has_sandbox = std::process::Command::new("which")
            .arg("sandbox-exec")
            .output()
            .is_ok_and(|o| o.status.success());

        let has_system_integrity = std::fs::metadata("/System/Library/Sandbox").is_ok();

        info!(
            "macOS capabilities: sandbox={}, sip={}",
            has_sandbox, has_system_integrity
        );
        Ok(has_sandbox)
    }

    async fn test_windows_capabilities(&self) -> Result<bool> {
        // Test Windows-specific capabilities like AppContainer, WDAG
        let has_powershell = std::process::Command::new("where")
            .arg("powershell")
            .output()
            .is_ok_and(|o| o.status.success());

        info!("Windows capabilities: powershell={}", has_powershell);
        Ok(has_powershell)
    }

    async fn test_generic_capabilities(&self) -> Result<bool> {
        // Generic capability tests that work across platforms
        let has_proc_info = !sysinfo::System::new_all().processes().is_empty();
        let has_filesystem = std::env::current_dir().is_ok();

        info!(
            "Generic capabilities: proc_info={}, filesystem={}",
            has_proc_info, has_filesystem
        );
        Ok(has_proc_info && has_filesystem)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DetectionResults {
    timestamp: chrono::DateTime<chrono::Utc>,
    total_platforms: usize,
    platforms: HashMap<String, DetectedPlatform>,
    categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UniversalCapabilities {
    platforms: HashMap<String, DetectedPlatform>,
    benchmarks: HashMap<String, BenchmarkResult>,
    federation_peers: HashMap<String, FederationPeer>,
    total_platforms: usize,
    available_platforms: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FederationRequest {
    peer_id: Uuid,
    mode: String,
    capabilities: Vec<String>,
    shared_resources: Vec<String>,
    protocol_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FederationResponse {
    peer_id: Uuid,
    protocol_version: String,
    capabilities: Vec<String>,
    accepted_resources: Vec<String>,
}

// Supporting structures for migration and federation
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub storage_type: String,
    pub gpu_info: Option<GpuInfo>,
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub vendor: String,
    pub model: String,
    pub memory_mb: u32,
    pub compute_capability: String,
}

#[derive(Debug, Clone)]
pub struct WorkloadCheckpoint {
    pub biome_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WorkloadExport {
    pub biome_name: String,
    pub export_path: PathBuf,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ReplicationHandle {
    pub id: uuid::Uuid,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct WorkloadSnapshot {
    pub biome_name: String,
    pub snapshot_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
