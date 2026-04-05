// SPDX-License-Identifier: AGPL-3.0-or-later
//! AS/400 System Adapter

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::info;
use uuid::Uuid;

use super::types::{IFSManager, MainframeJob, RPGCompiler, Terminal5250};

mod compiler;
mod connection;
mod jobs;
mod terminal;

use crate::{JobOutput, JobStatus, SpecialtyRuntimeConfig};
use crate::{
    LegacyAdapter, LegacyJob, LegacySystemType, MainframeConfig, SystemInfo, ToadStoolError,
    ToadStoolResult,
};

/// AS/400 Adapter
#[derive(Debug)]
pub struct AS400Adapter {
    /// Adapter configuration
    config: Option<MainframeConfig>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, MainframeJob>>>,
    /// RPG compiler
    _rpg_compiler: Arc<RPGCompiler>,
    /// 5250 terminal emulator
    _terminal_emulator: Arc<Mutex<Option<Terminal5250>>>,
    /// IFS (Integrated File System) manager
    _ifs_manager: Arc<IFSManager>,
    /// Connection status
    connected: Arc<Mutex<bool>>,
}

impl Default for AS400Adapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            _rpg_compiler: Arc::new(RPGCompiler::new()),
            _terminal_emulator: Arc::new(Mutex::new(None)),
            _ifs_manager: Arc::new(IFSManager::new()),
            connected: Arc::new(Mutex::new(false)),
        }
    }
}

impl AS400Adapter {
    /// Create a new AS/400 adapter
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for AS400Adapter {
    fn name(&self) -> &'static str {
        "AS/400 Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::AS400]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing AS/400 adapter");

        // Find AS/400 configuration
        for (name, mainframe_config) in &config.mainframe_configs {
            if mainframe_config.system_type == LegacySystemType::AS400 {
                self.config = Some(mainframe_config.clone());
                info!("Found AS/400 configuration: {}", name);
                break;
            }
        }

        if self.config.is_none() {
            return Err(ToadStoolError::runtime("No AS/400 configuration found"));
        }

        *self.connected.lock().await = true;

        info!("AS/400 adapter initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down AS/400 adapter");

        *self.connected.lock().await = false;

        info!("AS/400 adapter shutdown complete");
        Ok(())
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )] // label uses low bits of UUID only
    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting job to AS/400: {:?}", job.job_id);

        // Create mainframe job
        let mainframe_job = MainframeJob {
            job_id: job.job_id,
            job_name: format!("AS400JOB{:08X}", job.job_id.as_u128() as u32),
            job_class: "A".to_string(),
            priority: job.priority,
            jcl_content: "// AS/400 Job".to_string(),
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

        info!("Job submitted to AS/400: {}", job.job_id);
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
            info!("Cancelled AS/400 job: {}", job_id);
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
        // In a real implementation, this would query the AS/400 system
        Ok(SystemInfo {
            system_name: "IBM AS/400".to_string(),
            system_type: LegacySystemType::AS400,
            version: "V7R4".to_string(),
            architecture: crate::LegacyArchitecture::PowerPc601,
            cpu_info: crate::CpuInfo {
                model: "POWER8".to_string(),
                speed: 3_000_000_000, // 3 GHz
                cores: 8,
                features: vec!["POWER8 instruction set".to_string()],
                usage: 30.0,
            },
            memory_info: crate::MemoryInfo {
                total: 16 * 1024 * 1024 * 1024,    // 16 GB
                available: 8 * 1024 * 1024 * 1024, // 8 GB
                used: 8 * 1024 * 1024 * 1024,      // 8 GB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 1024 * 1024 * 1024 * 1024,    // 1 TB
                available: 512 * 1024 * 1024 * 1024, // 512 GB
                used: 512 * 1024 * 1024 * 1024,      // 512 GB
                storage_type: crate::StorageType::HardDisk,
            },
            network_info: crate::NetworkInfo {
                interfaces: vec![],
                protocols: vec![crate::NetworkProtocol::TCPIP],
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

#[cfg(test)]
mod tests {
    use super::super::types::{
        COBOLCompiler, DCLProcessor, DatasetManager, Field5250, Field5250Attributes, Field5250Type,
        IFSFile, IFSFileAttributes, IFSManager, JCLGenerator, MainframeJob, RPGCompiler,
        Terminal3270, Terminal3270Attributes, Terminal5250, VAXFortranCompiler, VAXTerminal,
        VAXTerminalAttributes, VMSFileSpec, VMSFileSystem,
    };
    use super::AS400Adapter;
    use super::compiler::find_compiler_in_path;
    use crate::{
        AuthenticationSettings, AuthenticationType, COBOLSettings, CommunicationRequirements,
        CommunicationSettings, CompilationRequirements, CompilerType, ConnectionSettings,
        CpuRequirements, DatasetConfig, DatasetType, FileSystemType, JCLSettings, LegacyAdapter,
        LegacyArchitecture, LegacyJob, LegacyJobType, LegacyLanguage, LegacyRuntimeRequirements,
        LegacySystemType, MainframeConfig, MainframeConnectionType, MemoryModel,
        MemoryRequirements, MemoryType, NetworkRequirements, RecordFormat, SpaceAllocation,
        SpaceUnit, SpecialtyRuntimeConfig, StorageRequirements, StorageType, TimingRequirements,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;
    use toadstool::JobPriority;
    use uuid::Uuid;

    fn minimal_mainframe_config() -> MainframeConfig {
        MainframeConfig {
            system_type: LegacySystemType::AS400,
            connection: ConnectionSettings {
                host: "127.0.0.1".to_string(),
                port: 5250,
                connection_type: MainframeConnectionType::IBM5250,
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
                compiler: "CRTRPGPGM".to_string(),
                compile_options: vec![],
                link_options: vec![],
                runtime_options: vec![],
            },
        }
    }

    fn minimal_legacy_job() -> LegacyJob {
        LegacyJob {
            job_id: Uuid::new_v4(),
            target_system: LegacySystemType::AS400,
            target_architecture: LegacyArchitecture::PowerPc601,
            job_type: LegacyJobType::Compilation {
                language: LegacyLanguage::RPG,
                target_format: crate::TargetFormat::Executable,
            },
            source: crate::LegacyJobSource::SourceCode {
                language: LegacyLanguage::RPG,
                code: "H SPEC".to_string(),
            },
            compilation_requirements: CompilationRequirements {
                compiler: CompilerType::IbmCobol,
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
                    architecture: LegacyArchitecture::PowerPc601,
                    min_speed: 1,
                    required_features: vec![],
                    fpu_required: false,
                },
                storage: StorageRequirements {
                    min_storage: 1024,
                    storage_type: StorageType::HardDisk,
                    file_system: FileSystemType::MvsDataset,
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
    fn find_compiler_in_path_returns_pathbuf() {
        let p = find_compiler_in_path("nonexistent-binary-xyz999");
        assert_eq!(p, PathBuf::from("nonexistent-binary-xyz999"));
    }

    #[test]
    fn as400_adapter_default_new_debug() {
        let a = AS400Adapter::default();
        let b = AS400Adapter::new();
        let s = format!("{:?}", a);
        assert!(s.contains("AS400Adapter"), "{s}");
        let _ = b;
    }

    #[test]
    fn jcl_generator_default_new_initialize_generate() {
        let mut jcl_gen = JCLGenerator::new();
        let _ = format!("{:?}", jcl_gen);
        JCLGenerator::default();
        let settings = minimal_mainframe_config().jcl_settings;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            jcl_gen.initialize(&settings).await.unwrap();
            let job = minimal_legacy_job();
            let jcl = jcl_gen.generate_jcl(&job).await.unwrap();
            assert!(jcl.contains("JOB"));
        });
    }

    #[tokio::test]
    async fn jcl_generator_generate_errors_without_template() {
        let jcl_gen = JCLGenerator::new();
        let job = minimal_legacy_job();
        let err = jcl_gen.generate_jcl(&job).await.unwrap_err();
        assert!(err.to_string().contains("template"));
    }

    #[test]
    fn cobol_compiler_default_new_initialize() {
        let mut c = COBOLCompiler::new();
        let _ = COBOLCompiler::default();
        let _ = format!("{:?}", c);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            c.initialize(&minimal_mainframe_config().cobol_settings)
                .await
                .unwrap();
        });
    }

    #[tokio::test]
    async fn terminal3270_connect_disconnect() {
        let mut t = Terminal3270::new();
        let _ = Terminal3270::default();
        let _ = format!("{:?}", t);
        let conn = minimal_mainframe_config().connection.clone();
        t.connect(&conn).await.unwrap();
        t.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn dataset_manager_initialize() {
        let mut m = DatasetManager::new();
        let _ = DatasetManager::default();
        let _ = format!("{:?}", m);
        m.initialize(&HashMap::new()).await.unwrap();
    }

    #[test]
    fn dcl_processor_vax_fortran_vax_terminal_vms_fs_rpg_defaults() {
        let d = DCLProcessor::new();
        let _ = DCLProcessor::default();
        let _ = format!("{:?}", d);

        let f = VAXFortranCompiler::new();
        let _ = VAXFortranCompiler::default();
        let _ = format!("{:?}", f);

        let vt = VAXTerminal::new();
        let _ = VAXTerminal::default();
        let _ = format!("{:?}", vt);

        let vfs = VMSFileSystem::new();
        let _ = VMSFileSystem::default();
        let _ = format!("{:?}", vfs);

        let r = RPGCompiler::new();
        let _ = RPGCompiler::default();
        let _ = format!("{:?}", r);
    }

    #[test]
    fn terminal5250_ifs_manager_defaults() {
        let t = Terminal5250::new();
        let _ = Terminal5250::default();
        let _ = format!("{:?}", t);

        let i = IFSManager::new();
        let _ = IFSManager::default();
        let _ = format!("{:?}", i);
    }

    #[test]
    fn serde_roundtrips_and_clone() {
        let job = MainframeJob {
            job_id: Uuid::nil(),
            job_name: "J".to_string(),
            job_class: "A".to_string(),
            priority: JobPriority::High,
            jcl_content: "//".to_string(),
            status: crate::JobStatus::Queued,
            start_time: None,
            end_time: None,
            output_datasets: vec![],
            return_code: None,
            job_log: String::new(),
        };
        let j2: MainframeJob = serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        assert_eq!(job.job_id, j2.job_id);
        assert_eq!(job.job_name, j2.job_name);

        let attrs = Terminal3270Attributes {
            width: 80,
            height: 24,
            color_support: false,
            extended_attributes: true,
        };
        assert_eq!(attrs, attrs.clone());

        let vax_a = VAXTerminalAttributes {
            width: 80,
            height: 24,
            capabilities: vec![],
        };
        let v2: VAXTerminalAttributes =
            serde_json::from_str(&serde_json::to_string(&vax_a).unwrap()).unwrap();
        assert_eq!(vax_a, v2);

        let spec = VMSFileSpec {
            device: "D".to_string(),
            directory: vec![],
            filename: "a".to_string(),
            file_type: "b".to_string(),
            version: None,
        };
        let s2: VMSFileSpec = serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
        assert_eq!(spec.filename, s2.filename);

        let field = Field5250 {
            name: "a".to_string(),
            position: (1, 2),
            length: 10,
            field_type: Field5250Type::Input,
            attributes: Field5250Attributes {
                color: None,
                highlighting: None,
                protected: false,
                intensity: None,
            },
            value: String::new(),
        };
        let f2: Field5250 = serde_json::from_str(&serde_json::to_string(&field).unwrap()).unwrap();
        assert_eq!(field.name, f2.name);

        let ifs = IFSFile {
            path: PathBuf::from("/tmp/x"),
            size: 1,
            file_type: "f".to_string(),
            attributes: IFSFileAttributes {
                permissions: "rw".to_string(),
                owner: "o".to_string(),
                group: "g".to_string(),
                ccsid: Some(37),
            },
            last_modified: std::time::SystemTime::UNIX_EPOCH,
        };
        let i2: IFSFile = serde_json::from_str(&serde_json::to_string(&ifs).unwrap()).unwrap();
        assert_eq!(ifs.size, i2.size);

        let cfg = minimal_mainframe_config();
        let c2: MainframeConfig =
            serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(cfg.system_type, c2.system_type);
    }

    #[tokio::test]
    async fn as400_initialize_missing_config_error() {
        let mut adapter = AS400Adapter::new();
        let err = adapter
            .initialize(&SpecialtyRuntimeConfig::default())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("AS/400"));
    }

    #[tokio::test]
    async fn as400_lifecycle_and_job_errors() {
        let mut adapter = AS400Adapter::new();
        let mut cfg = SpecialtyRuntimeConfig::default();
        cfg.mainframe_configs
            .insert("as400".to_string(), minimal_mainframe_config());
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
        assert_eq!(info.system_type, LegacySystemType::AS400);
        assert!(adapter.test_connectivity().await.unwrap());
        adapter.shutdown().await.unwrap();
        assert!(!adapter.test_connectivity().await.unwrap());
    }

    #[tokio::test]
    async fn dataset_manager_with_config() {
        let mut m = DatasetManager::new();
        let mut map = HashMap::new();
        map.insert(
            "d".to_string(),
            DatasetConfig {
                name: "d".to_string(),
                dataset_type: DatasetType::Sequential,
                record_format: RecordFormat::Fixed,
                record_length: 80,
                block_size: 800,
                space_allocation: SpaceAllocation {
                    primary: 1,
                    secondary: 1,
                    unit: SpaceUnit::Tracks,
                },
            },
        );
        m.initialize(&map).await.unwrap();
    }
}
