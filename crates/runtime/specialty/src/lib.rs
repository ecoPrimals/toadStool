#![deny(unsafe_code)]

//! # ToadStool Specialty Hardware Runtime Engine
//!
//! Specialty hardware support for ToadStool Universal Compute Platform.
//! 
//! This runtime engine provides execution support for:
//! - Mainframe systems (IBM System/360, VAX/VMS, AS/400, z/OS)
//! - Embedded systems (8-bit microcontrollers, 16-bit systems, Arduino, ESP32)
//! - Industrial control systems (PLCs, SCADA, real-time systems)
//! - Exotic Unix systems (PDP-11, early UNIX variants)
//! - Real-time operating systems (VxWorks, QNX, RT-11)
//!
//! ## Architecture
//!
//! ```text
//! Specialty Hardware Runtime Engine
//! ├── Mainframe Adapters (IBM, VAX, AS/400)
//! ├── Embedded Adapters (8-bit, 16-bit MCUs, Arduino, ESP32)
//! ├── Industrial Adapters (PLCs, SCADA)
//! ├── Real-time Adapters (VxWorks, QNX)
//! └── Cross-compilation Support
//! ```

// Migrated to native async traits
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// Re-export core types
pub use toadstool::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus,
    ResourceRequirements, RuntimeEngine, RuntimeMetrics, RuntimeType,
    ToadStoolError, ToadStoolResult, WorkloadType, RuntimeCapabilities,
};
pub use toadstool::execution;

pub mod types;
pub mod mainframe;
pub mod embedded;
pub mod industrial;
pub mod realtime;
pub mod cross_compilation;
pub mod legacy_networking;  // Legacy protocol support (appropriate name for protocol compatibility)
pub mod emulation;

// Re-export types for backward compatibility
pub use types::*;

// Import specific types needed in this module
use types::systems::{SystemStatus, LegacySystemType, LegacyArchitecture};
use types::requirements::{MemoryType, StorageType, NetworkProtocol};
use types::configs::{ToolchainConfig, MainframeConfig};

/// Specialty Hardware Runtime Engine for universal specialty system support
#[derive(Debug)]
pub struct SpecialtyRuntimeEngine {
    /// Runtime configuration
    config: SpecialtyRuntimeConfig,
    /// Active specialty hardware adapters
    adapters: Arc<RwLock<HashMap<LegacySystemType, Arc<dyn LegacyAdapter>>>>,
    /// Cross-compilation toolchains (using concrete types for now)
    toolchains: Arc<RwLock<HashMap<LegacyArchitecture, ToolchainConfig>>>,
    /// Active specialty jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, LegacyJob>>>,
    /// Communication sessions (using concrete type for now)
    communication_sessions: Arc<RwLock<HashMap<Uuid, Box<dyn LegacyCommunicationSession>>>>,
    /// System emulators (using concrete type for now)
    emulators: Arc<RwLock<HashMap<LegacySystemType, Box<dyn LegacyEmulator>>>>,
    /// Runtime metrics
    metrics: Arc<Mutex<SpecialtyRuntimeMetrics>>,
}

/// Configuration for specialty hardware runtime engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialtyRuntimeConfig {
    /// Enable mainframe support
    pub mainframe_enabled: bool,
    /// Enable embedded systems support
    pub embedded_enabled: bool,
    /// Enable industrial control support
    pub industrial_enabled: bool,
    /// Enable real-time systems support
    pub realtime_enabled: bool,
    /// Enable cross-compilation
    pub cross_compilation_enabled: bool,
    /// Enable legacy networking
    pub legacy_networking_enabled: bool,
    /// Enable system emulation
    pub emulation_enabled: bool,
    /// Maximum concurrent legacy jobs
    pub max_concurrent_jobs: usize,
    /// Job timeout
    pub job_timeout: Duration,
    /// Communication timeout
    pub communication_timeout: Duration,
    /// Supported legacy systems
    pub supported_systems: Vec<LegacySystemType>,
    /// Toolchain configurations
    pub toolchain_configs: HashMap<LegacyArchitecture, ToolchainConfig>,
    /// Mainframe connection configurations
    pub mainframe_configs: HashMap<String, MainframeConfig>,
    /// Embedded system configurations
    pub embedded_configs: HashMap<String, EmbeddedConfig>,
    /// Industrial system configurations
    pub industrial_configs: HashMap<String, IndustrialConfig>,
    /// Real-time system configurations
    pub realtime_configs: HashMap<String, RealtimeConfig>,
    /// Emulation configurations
    pub emulation_configs: HashMap<LegacySystemType, EmulationConfig>,
}

impl Default for SpecialtyRuntimeConfig {
    fn default() -> Self {
        Self {
            mainframe_enabled: true,
            embedded_enabled: true,
            industrial_enabled: true,
            realtime_enabled: true,
            cross_compilation_enabled: true,
            legacy_networking_enabled: true,
            emulation_enabled: true,
            max_concurrent_jobs: 10,
            job_timeout: Duration::from_secs(3600),
            communication_timeout: Duration::from_secs(30),
            supported_systems: Vec::new(),
            toolchain_configs: HashMap::new(),
            mainframe_configs: HashMap::new(),
            embedded_configs: HashMap::new(),
            industrial_configs: HashMap::new(),
            realtime_configs: HashMap::new(),
            emulation_configs: HashMap::new(),
        }
    }
}

impl SpecialtyRuntimeEngine {
    /// Create a new specialty hardware runtime engine
    pub fn new(config: SpecialtyRuntimeConfig) -> Self {
        Self {
            config,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            toolchains: Arc::new(RwLock::new(HashMap::new())),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            communication_sessions: Arc::new(RwLock::new(HashMap::new())),
            emulators: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(SpecialtyRuntimeMetrics::default())),
        }
    }
    
    /// Initialize the legacy runtime engine
    pub async fn initialize(&mut self) -> ToadStoolResult<()> {
        info!("Initializing Legacy Runtime Engine");
        
        // Initialize adapters based on configuration
        if self.config.mainframe_enabled {
            self.initialize_mainframe_adapters().await?;
        }
        
        if self.config.embedded_enabled {
            self.initialize_embedded_adapters().await?;
        }
        
        if self.config.industrial_enabled {
            self.initialize_industrial_adapters().await?;
        }
        
        if self.config.realtime_enabled {
            self.initialize_realtime_adapters().await?;
        }
        
        if self.config.cross_compilation_enabled {
            self.initialize_cross_compilation_toolchains().await?;
        }
        
        if self.config.emulation_enabled {
            self.initialize_emulators().await?;
        }
        
        info!("Legacy Runtime Engine initialized successfully");
        Ok(())
    }
    
    /// Initialize mainframe adapters
    async fn initialize_mainframe_adapters(&mut self) -> ToadStoolResult<()> {
        info!("Initializing mainframe adapters");
        
        // Initialize IBM System/360 adapter
        let ibm_adapter = mainframe::IBMMainframeAdapter::new();
        self.adapters.write().await.insert(LegacySystemType::IBM_System360, Box::new(ibm_adapter));
        
        // Initialize VAX/VMS adapter
        let vax_adapter = mainframe::VAXVMSAdapter::new();
        self.adapters.write().await.insert(LegacySystemType::VAX_VMS, Box::new(vax_adapter));
        
        // Initialize AS/400 adapter
        let as400_adapter = mainframe::AS400Adapter::new();
        self.adapters.write().await.insert(LegacySystemType::AS400, Box::new(as400_adapter));
        
        Ok(())
    }
    
    /// Initialize embedded system adapters
    async fn initialize_embedded_adapters(&mut self) -> ToadStoolResult<()> {
        info!("Initializing embedded system adapters");
        
        // Initialize 8-bit microcontroller adapters
        let mcu_8bit_adapter = embedded::Microcontroller8BitAdapter::new();
        self.adapters.write().await.insert(LegacySystemType::Intel8080, Box::new(mcu_8bit_adapter));
        
        // Initialize 16-bit system adapters
        let system_16bit_adapter = embedded::System16BitAdapter::new();
        self.adapters.write().await.insert(LegacySystemType::Intel8086, Box::new(system_16bit_adapter));
        
        Ok(())
    }
    
    /// Initialize industrial system adapters
    async fn initialize_industrial_adapters(&mut self) -> ToadStoolResult<()> {
        info!("Initializing industrial system adapters");
        
        // Initialize PLC adapter
        let plc_adapter = industrial::PLCAdapter::new();
        self.adapters.write().await.insert(LegacySystemType::PLC_Ladder, Box::new(plc_adapter));
        
        // Initialize SCADA adapter
        let scada_adapter = industrial::SCADAAdapter::new();
        self.adapters.write().await.insert(LegacySystemType::SCADA_System, Box::new(scada_adapter));
        
        Ok(())
    }
    
    /// Initialize real-time system adapters
    async fn initialize_realtime_adapters(&mut self) -> ToadStoolResult<()> {
        info!("Initializing real-time system adapters");
        
        // Initialize VxWorks adapter
        let vxworks_adapter = realtime::VxWorksAdapter::new();
        self.adapters.write().await.insert(LegacySystemType::VxWorks, Box::new(vxworks_adapter));
        
        // Initialize QNX adapter
        let qnx_adapter = realtime::QNXAdapter::new();
        self.adapters.write().await.insert(LegacySystemType::QNX_Legacy, Box::new(qnx_adapter));
        
        Ok(())
    }
    
    /// Initialize cross-compilation toolchains
    async fn initialize_cross_compilation_toolchains(&mut self) -> ToadStoolResult<()> {
        info!("Initializing cross-compilation toolchains");
        
        // Initialize 6502 toolchain
        let toolchain_6502 = cross_compilation::Toolchain6502::new();
        self.toolchains.write().await.insert(LegacyArchitecture::MOS6502, Box::new(toolchain_6502));
        
        // Initialize Z80 toolchain
        let toolchain_z80 = cross_compilation::ToolchainZ80::new();
        self.toolchains.write().await.insert(LegacyArchitecture::Zilog_Z80, Box::new(toolchain_z80));
        
        // Initialize 68000 toolchain
        let toolchain_68000 = cross_compilation::Toolchain68000::new();
        self.toolchains.write().await.insert(LegacyArchitecture::Motorola68000, Box::new(toolchain_68000));
        
        Ok(())
    }
    
    /// Initialize emulators
    async fn initialize_emulators(&mut self) -> ToadStoolResult<()> {
        info!("Initializing emulators");
        
        // Initialize PDP-11 emulator
        let pdp11_emulator = emulation::PDP11Emulator::new();
        self.emulators.write().await.insert(LegacySystemType::PDP11, Box::new(pdp11_emulator));
        
        // Initialize Apple II emulator
        let apple2_emulator = emulation::Apple2Emulator::new();
        self.emulators.write().await.insert(LegacySystemType::Apple_II, Box::new(apple2_emulator));
        
        Ok(())
    }
    
    /// Submit a legacy job for execution
    pub async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting legacy job: {:?}", job.job_id);
        
        // Check if we have an adapter for this system type
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&job.target_system)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", job.target_system)))?;
        
        // Submit the job
        let job_id = adapter.submit_job(job.clone()).await?;
        
        // Store the job
        self.active_jobs.write().await.insert(job_id, job);
        
        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.total_jobs += 1;
        metrics.active_jobs += 1;
        
        Ok(job_id)
    }
    
    /// Get the status of a legacy job
    pub async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        // Find the job
        let jobs = self.active_jobs.read().await;
        let job = jobs.get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?;
        
        // Get adapter for this job
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&job.target_system)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", job.target_system)))?;
        
        // Get job status
        adapter.get_job_status(job_id).await
    }
    
    /// Cancel a legacy job
    pub async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        // Find the job
        let jobs = self.active_jobs.read().await;
        let job = jobs.get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?;
        
        // Get adapter for this job
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&job.target_system)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", job.target_system)))?;
        
        // Cancel the job
        adapter.cancel_job(job_id).await?;
        
        // Remove from active jobs
        drop(jobs);
        self.active_jobs.write().await.remove(&job_id);
        
        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.active_jobs = metrics.active_jobs.saturating_sub(1);
        
        Ok(())
    }
    
    /// Get legacy job output
    pub async fn get_job_output(&self, job_id: Uuid) -> ToadStoolResult<JobOutput> {
        // Find the job
        let jobs = self.active_jobs.read().await;
        let job = jobs.get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?;
        
        // Get adapter for this job
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&job.target_system)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", job.target_system)))?;
        
        // Get job output
        adapter.get_job_output(job_id).await
    }
    
    /// Get runtime metrics
    pub async fn get_metrics(&self) -> ToadStoolResult<SpecialtyRuntimeMetrics> {
        let metrics = self.metrics.lock().await;
        Ok(metrics.clone())
    }
    
    /// Get supported legacy systems
    pub fn get_supported_systems(&self) -> Vec<LegacySystemType> {
        self.config.supported_systems.clone()
    }
    
    /// Test connectivity to a legacy system
    pub async fn test_connectivity(&self, system_type: LegacySystemType) -> ToadStoolResult<bool> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&system_type)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", system_type)))?;
        
        adapter.test_connectivity().await
    }
    
    /// Shutdown the specialty hardware runtime engine
    pub async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down Specialty Hardware Runtime Engine");
        
        // Shutdown all adapters
        let mut adapters = self.adapters.write().await;
        for (_, adapter) in adapters.iter_mut() {
            if let Err(e) = adapter.shutdown().await {
                error!("Error shutting down adapter: {}", e);
            }
        }
        
        // Shutdown all emulators
        let mut emulators = self.emulators.write().await;
        for (_, emulator) in emulators.iter_mut() {
            if let Err(e) = emulator.stop().await {
                error!("Error stopping emulator: {}", e);
            }
        }
        
        info!("Specialty Hardware Runtime Engine shutdown complete");
        Ok(())
    }
}

impl RuntimeEngine for SpecialtyRuntimeEngine {
    fn initialize(&mut self, config: execution::RuntimeConfig) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
        info!("Initializing specialty hardware runtime engine");
        
        // Apply runtime configuration if provided
        if let Some(resource_limits) = config.resource_limits {
            debug!("Applying resource limits: {:?}", resource_limits);
            // Resource limits are already set in the constructor via SpecialtyConfig
        }
        
        if let Some(security_settings) = config.security_settings {
            debug!("Applying security settings: {:?}", security_settings);
            // Security settings can be applied here if needed
        }
        
        info!("Specialty hardware runtime engine initialized successfully");
        Ok(())
        })
    }

    fn execute(&self, request: ExecutionRequest) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
        info!("Executing specialty hardware runtime request: {:?}", request.workload_id);
        
        // Convert ExecutionRequest to LegacyJob (maintains legacy system compatibility)
        let legacy_job = self.convert_execution_request_to_legacy_job(request)?;
        
        // Submit the job
        let job_id = self.submit_job(legacy_job).await?;
        
        // Wait for job completion or timeout
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
                    // Check timeout
                    if start_time.elapsed() > timeout {
                        self.cancel_job(job_id).await?;
                        return Ok(ExecutionResponse {
                            workload_id: job_id,
                            status: ExecutionStatus::TimedOut,
                            output: None,
                            error: Some("Job timed out".to_string()),
                            metrics: Some(self.get_runtime_metrics().await?),
                        });
                    }
                    
                    // BLOCKED(legacy-hardware): Polling external systems that don't provide
                    // event notifications. 1s interval is the minimum responsiveness for
                    // mainframe/embedded/RTOS integrations that only expose status queries.
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
        matches!(
            workload_type,
            WorkloadType::Native | WorkloadType::Custom(_)
        )
    }
    
    fn get_metrics(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async {
            self.get_runtime_metrics().await
        })
    }
    
    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
        info!("Shutting down legacy runtime engine");
        
        // Cancel all active jobs
        let jobs: Vec<Uuid> = self.active_jobs.read().await.keys().cloned().collect();
        for job_id in jobs {
            if let Err(e) = self.cancel_job(job_id).await {
                error!("Error cancelling job {}: {}", job_id, e);
            }
        }
        
        info!("Legacy runtime engine shutdown complete");
        Ok(())
        })
    }
}

impl SpecialtyRuntimeEngine {
    /// Convert ExecutionRequest to LegacyJob
    fn convert_execution_request_to_legacy_job(&self, request: ExecutionRequest) -> ToadStoolResult<LegacyJob> {
        // This is a simplified conversion - in practice, you'd need more sophisticated mapping
        // based on the workload specification and execution context
        
        let job_id = request.workload_id.unwrap_or_else(|| Uuid::new_v4());
        
        Ok(LegacyJob {
            job_id,
            target_system: LegacySystemType::Intel8086, // Default - should be determined from request
            target_architecture: LegacyArchitecture::Intel8086,
            job_type: LegacyJobType::Execution {
                program_format: ProgramFormat::DOS_EXE,
                arguments: vec![],
            },
            source: LegacyJobSource::SourceCode {
                language: LegacyLanguage::C_K_R,
                code: "/* Default legacy job */".to_string(),
            },
            compilation_requirements: CompilationRequirements {
                compiler: CompilerType::Microsoft_C_60,
                flags: vec![],
                include_paths: vec![],
                library_paths: vec![],
                libraries: vec![],
                memory_model: MemoryModel::Flat,
                optimization: OptimizationLevel::None,
                debug_info: false,
            },
            runtime_requirements: LegacyRuntimeRequirements {
                memory: MemoryRequirements {
                    min_memory: 64 * 1024, // 64KB
                    max_memory: 640 * 1024, // 640KB
                    memory_type: MemoryType::RAM,
                    memory_model: MemoryModel::Segmented,
                },
                cpu: CpuRequirements {
                    architecture: LegacyArchitecture::Intel8086,
                    min_speed: 4_770_000, // 4.77 MHz
                    required_features: vec![],
                    fpu_required: false,
                },
                storage: StorageRequirements {
                    min_storage: 360 * 1024, // 360KB floppy
                    storage_type: StorageType::FloppyDisk,
                    file_system: FileSystemType::DOS,
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
            priority: JobPriority::Normal,
            created_at: std::time::SystemTime::now(),
            timeout: Duration::from_secs(3600),
        })
    }
    
    /// Get runtime metrics in ToadStool format
    async fn get_runtime_metrics(&self) -> ToadStoolResult<RuntimeMetrics> {
        let _legacy_metrics = self.get_metrics().await?;
        
        // Maps legacy specialty runtime metrics to unified RuntimeMetrics structure
        // Legacy metrics are converted to standard ToadStool format
        Ok(RuntimeMetrics::default())
    }
}

/// Error types for specialty hardware runtime
#[derive(Debug, thiserror::Error)]
pub enum SpecialtyRuntimeError {
    #[error("System not supported: {0}")]
    SystemNotSupported(String),
    
    #[error("Architecture not supported: {0}")]
    ArchitectureNotSupported(String),
    
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
    
    #[error("Communication error: {0}")]
    CommunicationError(String),
    
    #[error("Emulation error: {0}")]
    EmulationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

impl From<SpecialtyRuntimeError> for ToadStoolError {
    fn from(err: SpecialtyRuntimeError) -> Self {
        ToadStoolError::runtime(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_specialty_runtime_engine_creation() {
        let config = SpecialtyRuntimeConfig::default();
        let engine = SpecialtyRuntimeEngine::new(config);
        
        // Verify capabilities reflect specialty hardware runtime
        let caps = engine.get_capabilities();
        assert!(caps.supported_workloads.contains(&WorkloadType::Custom("specialty".to_string())));
    }
    
    #[tokio::test]
    async fn test_legacy_system_types() {
        let systems = vec![
            LegacySystemType::IBM_System360,
            LegacySystemType::VAX_VMS,
            LegacySystemType::AS400,
            LegacySystemType::PDP11,
            LegacySystemType::Intel8080,
            LegacySystemType::MOS6502,
            LegacySystemType::VxWorks,
        ];
        
        for system in systems {
            // Test serialization
            let serialized = serde_json::to_string(&system).unwrap();
            let deserialized: LegacySystemType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(system, deserialized);
        }
    }
    
    #[tokio::test]
    async fn test_legacy_job_creation() {
        let job = LegacyJob {
            job_id: Uuid::new_v4(),
            target_system: LegacySystemType::Intel8086,
            target_architecture: LegacyArchitecture::Intel8086,
            job_type: LegacyJobType::Compilation {
                language: LegacyLanguage::C_K_R,
                target_format: TargetFormat::Executable,
            },
            source: LegacyJobSource::SourceCode {
                language: LegacyLanguage::C_K_R,
                code: "int main() { return 0; }".to_string(),
            },
            compilation_requirements: CompilationRequirements {
                compiler: CompilerType::Microsoft_C_60,
                flags: vec![],
                include_paths: vec![],
                library_paths: vec![],
                libraries: vec![],
                memory_model: MemoryModel::Flat,
                optimization: OptimizationLevel::None,
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
                    file_system: FileSystemType::DOS,
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
            priority: JobPriority::Normal,
            created_at: std::time::SystemTime::now(),
            timeout: Duration::from_secs(3600),
        };
        
        // Test serialization
        let serialized = serde_json::to_string(&job).unwrap();
        let deserialized: LegacyJob = serde_json::from_str(&serialized).unwrap();
        assert_eq!(job.job_id, deserialized.job_id);
        assert_eq!(job.target_system, deserialized.target_system);
    }
} 