// SPDX-License-Identifier: AGPL-3.0-or-later
//! VAX/VMS System Adapter

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::info;
use uuid::Uuid;

use super::types::{
    DCLProcessor, MainframeJob, VAXFortranCompiler, VAXTerminal, VAXTerminalAttributes,
    VMSFileSystem,
};
use crate::{JobOutput, JobStatus, SpecialtyRuntimeConfig};
use crate::{
    LegacyAdapter, LegacyJob, LegacySystemType, MainframeConfig, SystemInfo, ToadStoolError,
    ToadStoolResult,
};

/// VAX/VMS Adapter
#[derive(Debug)]
pub struct VAXVMSAdapter {
    /// Adapter configuration
    config: Option<MainframeConfig>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, MainframeJob>>>,
    /// DCL command processor
    _dcl_processor: Arc<DCLProcessor>,
    /// VAX FORTRAN compiler
    _fortran_compiler: Arc<VAXFortranCompiler>,
    /// Terminal interface
    _terminal_interface: Arc<Mutex<Option<VAXTerminal>>>,
    /// File system manager
    _file_system: Arc<VMSFileSystem>,
    /// Connection status
    connected: Arc<Mutex<bool>>,
}

impl Default for VAXVMSAdapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            _dcl_processor: Arc::new(DCLProcessor::new()),
            _fortran_compiler: Arc::new(VAXFortranCompiler::new()),
            _terminal_interface: Arc::new(Mutex::new(None)),
            _file_system: Arc::new(VMSFileSystem::new()),
            connected: Arc::new(Mutex::new(false)),
        }
    }
}

impl VAXVMSAdapter {
    /// Create a new VAX/VMS adapter
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for VAXVMSAdapter {
    fn name(&self) -> &'static str {
        "VAX/VMS Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::VaxVms]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing VAX/VMS adapter");

        // Find VAX/VMS configuration
        for (name, mainframe_config) in &config.mainframe_configs {
            if mainframe_config.system_type == LegacySystemType::VaxVms {
                self.config = Some(mainframe_config.clone());
                info!("Found VAX/VMS configuration: {}", name);
                break;
            }
        }

        if self.config.is_none() {
            return Err(ToadStoolError::runtime("No VAX/VMS configuration found"));
        }

        *self.connected.lock().await = true;

        info!("VAX/VMS adapter initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down VAX/VMS adapter");

        *self.connected.lock().await = false;

        info!("VAX/VMS adapter shutdown complete");
        Ok(())
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )] // label uses low bits of UUID only
    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting job to VAX/VMS: {:?}", job.job_id);

        // Create mainframe job
        let mainframe_job = MainframeJob {
            job_id: job.job_id,
            job_name: format!("VAXJOB{:08X}", job.job_id.as_u128() as u32),
            job_class: "A".to_string(),
            priority: job.priority,
            jcl_content: "! VAX/VMS DCL Job".to_string(),
            status: JobStatus::Queued,
            start_time: None,
            end_time: None,
            output_datasets: vec![],
            return_code: None,
            job_log: String::new(),
        };

        self.active_jobs
            .write()
            .await
            .insert(job.job_id, mainframe_job);

        info!("Job submitted to VAX/VMS: {}", job.job_id);
        Ok(job.job_id)
    }

    async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        let jobs = self.active_jobs.read().await;
        jobs.get(&job_id).map_or_else(
            || {
                Err(ToadStoolError::runtime(format!(
                    "Job not found: {}",
                    job_id
                )))
            },
            |job| Ok(job.status.clone()),
        )
    }

    async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        let mut jobs = self.active_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Cancelled;
            info!("Cancelled VAX/VMS job: {}", job_id);
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
        jobs.get(&job_id).map_or_else(
            || {
                Err(ToadStoolError::runtime(format!(
                    "Job not found: {}",
                    job_id
                )))
            },
            |job| {
                Ok(JobOutput {
                    stdout: job.job_log.clone(),
                    stderr: String::new(),
                    return_code: job.return_code,
                    output_files: vec![],
                    binary_output: None,
                })
            },
        )
    }

    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo> {
        // In a real implementation, this would query the VAX/VMS system
        Ok(SystemInfo {
            system_name: "VAX/VMS".to_string(),
            system_type: LegacySystemType::VaxVms,
            version: "7.3".to_string(),
            architecture: crate::LegacyArchitecture::VAX,
            cpu_info: crate::CpuInfo {
                model: "VAX-11/780".to_string(),
                speed: 5_000_000, // 5 MHz
                cores: 1,
                features: vec!["VAX instruction set".to_string()],
                usage: 15.0,
            },
            memory_info: crate::MemoryInfo {
                total: 8 * 1024 * 1024,     // 8 MB
                available: 4 * 1024 * 1024, // 4 MB
                used: 4 * 1024 * 1024,      // 4 MB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 300 * 1024 * 1024,     // 300 MB
                available: 150 * 1024 * 1024, // 150 MB
                used: 150 * 1024 * 1024,      // 150 MB
                storage_type: crate::StorageType::HardDisk,
            },
            network_info: crate::NetworkInfo {
                interfaces: vec![],
                protocols: vec![crate::NetworkProtocol::DECnet],
                status: crate::NetworkStatus::Online,
            },
            status: crate::SystemStatus::Online,
        })
    }

    async fn test_connectivity(&self) -> ToadStoolResult<bool> {
        let connected = self.connected.lock().await;
        Ok(*connected)
    }
}

// ── VAX/VMS supporting component implementations ────────────────────────────

/// PATH-based compiler lookup (no hardcoded `/usr/bin` paths).
fn find_compiler_in_path(name: &str) -> std::path::PathBuf {
    std::env::var_os("PATH")
        .and_then(|path_var| {
            std::env::split_paths(&path_var)
                .map(|dir| dir.join(name))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| std::path::PathBuf::from(name))
}

impl Default for DCLProcessor {
    fn default() -> Self {
        Self {
            templates: HashMap::new(),
            environment: HashMap::new(),
            current_directory: std::path::PathBuf::from("SYS$LOGIN:"),
        }
    }
}

impl DCLProcessor {
    /// Creates a new DCL processor for VAX/VMS.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for VAXFortranCompiler {
    fn default() -> Self {
        Self {
            compiler_path: find_compiler_in_path("f77"),
            compiler_options: vec![],
            library_paths: vec![],
        }
    }
}

impl VAXFortranCompiler {
    /// Creates a new VAX FORTRAN compiler interface.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for VAXTerminal {
    fn default() -> Self {
        Self {
            terminal_type: "VT100".to_string(),
            attributes: VAXTerminalAttributes {
                width: 80,
                height: 24,
                capabilities: vec!["cursor_addressing".to_string()],
            },
            session: None,
        }
    }
}

impl VAXTerminal {
    /// Creates a new VAX terminal interface.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for VMSFileSystem {
    fn default() -> Self {
        Self {
            file_specs: HashMap::new(),
            directory_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl VMSFileSystem {
    /// Creates a new VMS file system interface.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{MainframeJob, VAXTerminalAttributes, VMSFileSpec};
    use super::VAXVMSAdapter;
    use crate::{
        AuthenticationSettings, AuthenticationType, COBOLSettings, CommunicationRequirements,
        CommunicationSettings, CompilationRequirements, CompilerType, ConnectionSettings,
        CpuRequirements, FileSystemType, JCLSettings, LegacyAdapter, LegacyArchitecture, LegacyJob,
        LegacyJobType, LegacyLanguage, LegacyRuntimeRequirements, LegacySystemType,
        MainframeConfig, MainframeConnectionType, MemoryModel, MemoryRequirements, MemoryType,
        NetworkRequirements, SpecialtyRuntimeConfig, StorageRequirements, StorageType,
        TimingRequirements,
    };
    use std::collections::HashMap;
    use std::time::Duration;
    use toadstool::JobPriority;
    use toadstool_common::constants::network::LOCALHOST_IPV4;
    use uuid::Uuid;

    fn minimal_mainframe_config() -> MainframeConfig {
        MainframeConfig {
            system_type: LegacySystemType::VaxVms,
            connection: ConnectionSettings {
                host: LOCALHOST_IPV4.to_string(),
                port: 23,
                connection_type: MainframeConnectionType::IBM3270,
                authentication: AuthenticationSettings {
                    auth_type: AuthenticationType::None,
                    username: None,
                    password: None,
                    key_file: None,
                    certificate: None,
                },
            },
            datasets: HashMap::new(),
            jcl_settings: JCLSettings {
                job_class: "A".to_string(),
                message_class: "A".to_string(),
                priority: 1,
                time_limit: Duration::from_secs(3600),
                region_size: 1024 * 1024,
            },
            cobol_settings: COBOLSettings {
                compiler: "VAX".to_string(),
                compile_options: vec![],
                link_options: vec![],
                runtime_options: vec![],
            },
        }
    }

    fn minimal_legacy_job() -> LegacyJob {
        LegacyJob {
            job_id: Uuid::new_v4(),
            target_system: LegacySystemType::VaxVms,
            target_architecture: LegacyArchitecture::VAX,
            job_type: LegacyJobType::Compilation {
                language: LegacyLanguage::Fortran77,
                target_format: crate::TargetFormat::Executable,
            },
            source: crate::LegacyJobSource::SourceCode {
                language: LegacyLanguage::Fortran77,
                code: "C TEST".to_string(),
            },
            compilation_requirements: CompilationRequirements {
                compiler: CompilerType::VaxFortran,
                flags: vec![],
                include_paths: vec![],
                library_paths: vec![],
                libraries: vec![],
                memory_model: MemoryModel::Flat,
                optimization: crate::OptimizationLevel::None,
                debug_info: false,
            },
            runtime_requirements: LegacyRuntimeRequirements {
                memory: MemoryRequirements {
                    min_memory: 64 * 1024,
                    max_memory: 1024 * 1024,
                    memory_type: MemoryType::RAM,
                    memory_model: MemoryModel::Flat,
                },
                cpu: CpuRequirements {
                    architecture: LegacyArchitecture::VAX,
                    min_speed: 1,
                    required_features: vec![],
                    fpu_required: false,
                },
                storage: StorageRequirements {
                    min_storage: 1024,
                    storage_type: StorageType::HardDisk,
                    file_system: FileSystemType::VMS,
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
                    max_response_time: Duration::from_secs(60),
                    min_cycle_time: Duration::from_millis(1),
                    timing_accuracy: Duration::from_millis(1),
                },
                special_hardware: vec![],
            },
            communication_settings: CommunicationSettings::default(),
            priority: JobPriority::Normal,
            created_at: std::time::SystemTime::UNIX_EPOCH,
            timeout: Duration::from_secs(300),
        }
    }

    #[test]
    fn vax_adapter_default_new_and_debug() {
        let a = VAXVMSAdapter::default();
        let b = VAXVMSAdapter::new();
        let s = format!("{:?}", a);
        assert!(s.contains("VAXVMSAdapter"), "{s}");
        let _ = b;
    }

    #[test]
    fn vax_serde_roundtrips() {
        let job = MainframeJob {
            job_id: Uuid::new_v4(),
            job_name: "V".to_string(),
            job_class: "B".to_string(),
            priority: JobPriority::Normal,
            jcl_content: "!X".to_string(),
            status: crate::JobStatus::Running,
            start_time: None,
            end_time: None,
            output_datasets: vec![],
            return_code: None,
            job_log: String::new(),
        };
        let j2: MainframeJob = serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        assert_eq!(job.job_id, j2.job_id);

        let spec = VMSFileSpec {
            device: "DKA0".to_string(),
            directory: vec!["SYS".to_string()],
            filename: "FOO".to_string(),
            file_type: "TXT".to_string(),
            version: Some(2),
        };
        let s2: VMSFileSpec = serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
        assert_eq!(spec.filename, s2.filename);

        let attrs = VAXTerminalAttributes {
            width: 132,
            height: 24,
            capabilities: vec!["wrap".to_string()],
        };
        let a2: VAXTerminalAttributes =
            serde_json::from_str(&serde_json::to_string(&attrs).unwrap()).unwrap();
        assert_eq!(attrs.width, a2.width);
    }

    #[test]
    fn mainframe_config_serde_roundtrip() {
        let c = minimal_mainframe_config();
        let c2: MainframeConfig =
            serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(c.system_type, c2.system_type);
    }

    #[tokio::test]
    async fn vax_initialize_errors_without_config() {
        let mut adapter = VAXVMSAdapter::new();
        let err = adapter
            .initialize(&SpecialtyRuntimeConfig::default())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("VAX"));
    }

    #[tokio::test]
    async fn vax_lifecycle_and_job_errors() {
        let mut adapter = VAXVMSAdapter::new();
        let mut cfg = SpecialtyRuntimeConfig::default();
        cfg.mainframe_configs
            .insert("vax".to_string(), minimal_mainframe_config());
        adapter.initialize(&cfg).await.unwrap();

        let missing = Uuid::nil();
        adapter.get_job_status(missing).await.unwrap_err();
        adapter.cancel_job(missing).await.unwrap_err();
        adapter.get_job_output(missing).await.unwrap_err();

        let job = minimal_legacy_job();
        let id = adapter.submit_job(job).await.unwrap();
        assert_eq!(
            adapter.get_job_status(id).await.unwrap(),
            crate::JobStatus::Queued
        );
        adapter.cancel_job(id).await.unwrap();

        let info = adapter.get_system_info().await.unwrap();
        assert_eq!(info.system_type, LegacySystemType::VaxVms);
        assert!(adapter.test_connectivity().await.unwrap());
        adapter.shutdown().await.unwrap();
        assert!(!adapter.test_connectivity().await.unwrap());
    }
}

// Implementation for AS/400 Adapter
