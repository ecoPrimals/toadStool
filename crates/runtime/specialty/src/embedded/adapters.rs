// SPDX-License-Identifier: AGPL-3.0-or-later
//! Adapter implementations for embedded systems
//!
//! This module contains the main adapter structs for 8-bit and 16-bit embedded systems.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    EmbeddedConfig, JobOutput, JobStatus, LegacyAdapter, LegacyJob, LegacySystemType,
    SpecialtyRuntimeConfig, SystemInfo, ToadStoolError, ToadStoolResult,
    LegacyArchitecture,
};

use super::types::*;
use super::managers::{MemoryLayoutManager, PeripheralManager};
use super::dos::DOSInterface;
use super::toolchains::*;
use super::emulators::*;
use super::programmers::*;

/// 8-bit Microcontroller Adapter
#[derive(Debug)]
pub struct Microcontroller8BitAdapter {
    /// Adapter configuration
    config: Option<EmbeddedConfig>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, EmbeddedJob>>>,
    /// Cross-compilation toolchains
    toolchains: Arc<RwLock<HashMap<LegacyArchitecture, Box<dyn EmbeddedToolchain>>>>,
    /// Programming interfaces
    programmers: Arc<RwLock<HashMap<String, Box<dyn ProgrammerInterface>>>>,
    /// Emulators
    emulators: Arc<RwLock<HashMap<LegacyArchitecture, Box<dyn EmbeddedEmulator>>>>,
    /// Memory layout manager
    memory_manager: Arc<MemoryLayoutManager>,
    /// Peripheral manager
    peripheral_manager: Arc<PeripheralManager>,
}

/// 16-bit System Adapter
#[derive(Debug)]
pub struct System16BitAdapter {
    /// Adapter configuration
    config: Option<EmbeddedConfig>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, EmbeddedJob>>>,
    /// Cross-compilation toolchains
    toolchains: Arc<RwLock<HashMap<LegacyArchitecture, Box<dyn EmbeddedToolchain>>>>,
    /// System emulators
    emulators: Arc<RwLock<HashMap<LegacyArchitecture, Box<dyn EmbeddedEmulator>>>>,
    /// Memory layout manager
    memory_manager: Arc<MemoryLayoutManager>,
    /// DOS interface (for 8086 systems)
    dos_interface: Arc<Mutex<Option<DOSInterface>>>,
}

// Implementation for 8-bit Microcontroller Adapter
impl Microcontroller8BitAdapter {
    /// Create a new 8-bit microcontroller adapter
    pub fn new() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            toolchains: Arc::new(RwLock::new(HashMap::new())),
            programmers: Arc::new(RwLock::new(HashMap::new())),
            emulators: Arc::new(RwLock::new(HashMap::new())),
            memory_manager: Arc::new(MemoryLayoutManager::new()),
            peripheral_manager: Arc::new(PeripheralManager::new()),
        }
    }

    /// Initialize toolchains for supported architectures
    async fn initialize_toolchains(&self) -> ToadStoolResult<()> {
        let mut toolchains = self.toolchains.write().await;

        // Initialize 6502 toolchain
        let toolchain_6502 = Box::new(Toolchain6502::new());
        toolchains.insert(LegacyArchitecture::MOS6502, toolchain_6502);

        // Initialize Z80 toolchain
        let toolchain_z80 = Box::new(ToolchainZ80::new());
        toolchains.insert(LegacyArchitecture::Zilog_Z80, toolchain_z80);

        // Initialize 8080 toolchain
        let toolchain_8080 = Box::new(Toolchain8080::new());
        toolchains.insert(LegacyArchitecture::Intel8080, toolchain_8080);

        // Initialize 8051 toolchain
        let toolchain_8051 = Box::new(Toolchain8051::new());
        toolchains.insert(LegacyArchitecture::Intel_8051, toolchain_8051);

        info!("Initialized toolchains for 8-bit microcontrollers");
        Ok(())
    }

    /// Initialize programmers
    async fn initialize_programmers(&self) -> ToadStoolResult<()> {
        let mut programmers = self.programmers.write().await;

        // Initialize generic programmer
        let generic_programmer = Box::new(GenericProgrammer::new());
        programmers.insert("generic".to_string(), generic_programmer);

        // Initialize EPROM programmer
        let eprom_programmer = Box::new(EPROMProgrammer::new());
        programmers.insert("eprom".to_string(), eprom_programmer);

        info!("Initialized programmers for 8-bit microcontrollers");
        Ok(())
    }

    /// Initialize emulators
    async fn initialize_emulators(&self) -> ToadStoolResult<()> {
        let mut emulators = self.emulators.write().await;

        // Initialize 6502 emulator
        let emulator_6502 = Box::new(Emulator6502::new());
        emulators.insert(LegacyArchitecture::MOS6502, emulator_6502);

        // Initialize Z80 emulator
        let emulator_z80 = Box::new(EmulatorZ80::new());
        emulators.insert(LegacyArchitecture::Zilog_Z80, emulator_z80);

        info!("Initialized emulators for 8-bit microcontrollers");
        Ok(())
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for Microcontroller8BitAdapter {
    fn name(&self) -> &str {
        "8-bit Microcontroller Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![
            LegacySystemType::Intel8080,
            LegacySystemType::MOS6502,
            LegacySystemType::Zilog_Z80,
            LegacySystemType::Intel8051,
        ]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing 8-bit microcontroller adapter");

        // Find embedded configuration
        for (name, embedded_config) in &config.embedded_configs {
            if matches!(
                embedded_config.architecture,
                LegacyArchitecture::Intel8080
                    | LegacyArchitecture::MOS6502
                    | LegacyArchitecture::Zilog_Z80
                    | LegacyArchitecture::Intel_8051
            ) {
                self.config = Some(embedded_config.clone());
                info!("Found 8-bit microcontroller configuration: {}", name);
                break;
            }
        }

        if self.config.is_none() {
            return Err(ToadStoolError::runtime(
                "No 8-bit microcontroller configuration found",
            ));
        }

        // Initialize components
        self.initialize_toolchains().await?;
        self.initialize_programmers().await?;
        self.initialize_emulators().await?;

        info!("8-bit microcontroller adapter initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down 8-bit microcontroller adapter");

        // Shutdown all components
        self.toolchains.write().await.clear();
        self.programmers.write().await.clear();
        self.emulators.write().await.clear();

        info!("8-bit microcontroller adapter shutdown complete");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting job to 8-bit microcontroller: {:?}", job.job_id);

        // Create embedded job - config must be initialized
        let config = self.config.as_ref().ok_or_else(|| {
            ToadStoolError::configuration("8-bit microcontroller adapter config not initialized")
        })?;

        let embedded_job = EmbeddedJob {
            job_id: job.job_id,
            target_architecture: LegacyArchitecture::MOS6502, // Default, should be determined from job
            job_type: EmbeddedJobType::Compilation {
                language: EmbeddedLanguage::Assembly,
                optimization: OptimizationLevel::Size,
                debug_info: false,
            },
            source_files: vec![],
            memory_layout: config.memory_layout.clone(),
            programming_interface: config.programming_interface.clone(),
            status: JobStatus::Queued,
            output_files: vec![],
            compilation_log: String::new(),
            programming_log: String::new(),
            start_time: None,
            end_time: None,
        };

        self.active_jobs
            .write()
            .await
            .insert(job.job_id, embedded_job);

        info!("Job submitted to 8-bit microcontroller: {}", job.job_id);
        Ok(job.job_id)
    }

    async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        let jobs = self.active_jobs.read().await;
        if let Some(job) = jobs.get(&job_id) {
            Ok(job.status.clone())
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        let mut jobs = self.active_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Cancelled;
            info!("Cancelled 8-bit microcontroller job: {}", job_id);
            Ok(())
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn get_job_output(&self, job_id: Uuid) -> ToadStoolResult<JobOutput> {
        let jobs = self.active_jobs.read().await;
        if let Some(job) = jobs.get(&job_id) {
            Ok(JobOutput {
                stdout: job.compilation_log.clone(),
                stderr: job.programming_log.clone(),
                return_code: Some(0),
                output_files: vec![],
                binary_output: None,
            })
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo> {
        Ok(SystemInfo {
            system_name: "8-bit Microcontroller".to_string(),
            system_type: LegacySystemType::MOS6502,
            version: "1.0".to_string(),
            architecture: LegacyArchitecture::MOS6502,
            cpu_info: crate::CpuInfo {
                model: "MOS 6502".to_string(),
                speed: 1_000_000, // 1 MHz
                cores: 1,
                features: vec!["8-bit".to_string()],
                usage: 0.0,
            },
            memory_info: crate::MemoryInfo {
                total: 64 * 1024,      // 64KB
                available: 32 * 1024,  // 32KB
                used: 32 * 1024,       // 32KB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 32 * 1024, // 32KB ROM
                available: 0,
                used: 32 * 1024,
                storage_type: crate::StorageType::Cartridge,
            },
            network_info: crate::NetworkInfo {
                interfaces: vec![],
                protocols: vec![],
                status: crate::NetworkStatus::Offline,
            },
            status: crate::SystemStatus::Online,
        })
    }

    async fn test_connectivity(&self) -> ToadStoolResult<bool> {
        Ok(true)
    }
}

// Implementation for 16-bit System Adapter
impl System16BitAdapter {
    /// Create a new 16-bit system adapter
    pub fn new() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            toolchains: Arc::new(RwLock::new(HashMap::new())),
            emulators: Arc::new(RwLock::new(HashMap::new())),
            memory_manager: Arc::new(MemoryLayoutManager::new()),
            dos_interface: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize toolchains for 16-bit systems
    async fn initialize_toolchains(&self) -> ToadStoolResult<()> {
        let mut toolchains = self.toolchains.write().await;

        // Initialize 8086 toolchain
        let toolchain_8086 = Box::new(Toolchain8086::new());
        toolchains.insert(LegacyArchitecture::Intel8086, toolchain_8086);

        // Initialize 68000 toolchain
        let toolchain_68000 = Box::new(Toolchain68000::new());
        toolchains.insert(LegacyArchitecture::Motorola68000, toolchain_68000);

        info!("Initialized toolchains for 16-bit systems");
        Ok(())
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for System16BitAdapter {
    fn name(&self) -> &str {
        "16-bit System Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![
            LegacySystemType::Intel8086,
            LegacySystemType::Motorola68000,
            LegacySystemType::DOS_16bit,
        ]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing 16-bit system adapter");

        // Find embedded configuration
        for (name, embedded_config) in &config.embedded_configs {
            if matches!(
                embedded_config.architecture,
                LegacyArchitecture::Intel8086 | LegacyArchitecture::Motorola68000
            ) {
                self.config = Some(embedded_config.clone());
                info!("Found 16-bit system configuration: {}", name);
                break;
            }
        }

        if self.config.is_none() {
            return Err(ToadStoolError::runtime(
                "No 16-bit system configuration found",
            ));
        }

        // Initialize components
        self.initialize_toolchains().await?;

        // Initialize DOS interface if needed - config must be initialized
        let config = self.config.as_ref().ok_or_else(|| {
            ToadStoolError::configuration("16-bit system adapter config not initialized")
        })?;

        if config.architecture == LegacyArchitecture::Intel8086 {
            let dos_interface = DOSInterface::new();
            *self.dos_interface.lock().await = Some(dos_interface);
        }

        info!("16-bit system adapter initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down 16-bit system adapter");

        // Shutdown all components
        self.toolchains.write().await.clear();
        self.emulators.write().await.clear();
        *self.dos_interface.lock().await = None;

        info!("16-bit system adapter shutdown complete");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting job to 16-bit system: {:?}", job.job_id);

        // Create embedded job - config must be initialized
        let config = self.config.as_ref().ok_or_else(|| {
            ToadStoolError::configuration("16-bit system adapter config not initialized")
        })?;

        let embedded_job = EmbeddedJob {
            job_id: job.job_id,
            target_architecture: LegacyArchitecture::Intel8086, // Default
            job_type: EmbeddedJobType::Compilation {
                language: EmbeddedLanguage::C,
                optimization: OptimizationLevel::Size,
                debug_info: false,
            },
            source_files: vec![],
            memory_layout: config.memory_layout.clone(),
            programming_interface: config.programming_interface.clone(),
            status: JobStatus::Queued,
            output_files: vec![],
            compilation_log: String::new(),
            programming_log: String::new(),
            start_time: None,
            end_time: None,
        };

        self.active_jobs
            .write()
            .await
            .insert(job.job_id, embedded_job);

        info!("Job submitted to 16-bit system: {}", job.job_id);
        Ok(job.job_id)
    }

    async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        let jobs = self.active_jobs.read().await;
        if let Some(job) = jobs.get(&job_id) {
            Ok(job.status.clone())
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        let mut jobs = self.active_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Cancelled;
            info!("Cancelled 16-bit system job: {}", job_id);
            Ok(())
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn get_job_output(&self, job_id: Uuid) -> ToadStoolResult<JobOutput> {
        let jobs = self.active_jobs.read().await;
        if let Some(job) = jobs.get(&job_id) {
            Ok(JobOutput {
                stdout: job.compilation_log.clone(),
                stderr: job.programming_log.clone(),
                return_code: Some(0),
                output_files: vec![],
                binary_output: None,
            })
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo> {
        Ok(SystemInfo {
            system_name: "16-bit System".to_string(),
            system_type: LegacySystemType::Intel8086,
            version: "1.0".to_string(),
            architecture: LegacyArchitecture::Intel8086,
            cpu_info: crate::CpuInfo {
                model: "Intel 8086".to_string(),
                speed: 4_770_000, // 4.77 MHz
                cores: 1,
                features: vec!["16-bit".to_string()],
                usage: 0.0,
            },
            memory_info: crate::MemoryInfo {
                total: 640 * 1024,     // 640KB
                available: 320 * 1024, // 320KB
                used: 320 * 1024,      // 320KB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 360 * 1024,     // 360KB floppy
                available: 100 * 1024, // 100KB
                used: 260 * 1024,      // 260KB
                storage_type: crate::StorageType::FloppyDisk,
            },
            network_info: crate::NetworkInfo {
                interfaces: vec![],
                protocols: vec![],
                status: crate::NetworkStatus::Offline,
            },
            status: crate::SystemStatus::Online,
        })
    }

    async fn test_connectivity(&self) -> ToadStoolResult<bool> {
        Ok(true)
    }
}

