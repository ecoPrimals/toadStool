// SPDX-License-Identifier: AGPL-3.0-only
//! IBM Mainframe Adapter (System/360, System/370, z/Series)

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::info;
use uuid::Uuid;

use super::types::{COBOLCompiler, DatasetManager, JCLGenerator, MainframeJob, Terminal3270};
use crate::{JobOutput, JobStatus, SpecialtyRuntimeConfig};
use crate::{
    LegacyAdapter, LegacyJob, LegacySystemType, MainframeConfig, SystemInfo, ToadStoolError,
    ToadStoolResult,
};
use toadstool::JobPriority;

/// IBM Mainframe Adapter for System/360, System/370, z/Series
#[derive(Debug)]
pub struct IBMMainframeAdapter {
    /// Adapter configuration
    config: Option<MainframeConfig>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, MainframeJob>>>,
    /// JCL generator
    jcl_generator: Arc<Mutex<JCLGenerator>>,
    /// COBOL compiler interface
    cobol_compiler: Arc<Mutex<COBOLCompiler>>,
    /// 3270 terminal emulator
    terminal_emulator: Arc<Mutex<Option<Terminal3270>>>,
    /// Dataset manager
    dataset_manager: Arc<Mutex<DatasetManager>>,
    /// Connection status
    connected: Arc<Mutex<bool>>,
}

impl Default for IBMMainframeAdapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            jcl_generator: Arc::new(Mutex::new(JCLGenerator::new())),
            cobol_compiler: Arc::new(Mutex::new(COBOLCompiler::new())),
            terminal_emulator: Arc::new(Mutex::new(None)),
            dataset_manager: Arc::new(Mutex::new(DatasetManager::new())),
            connected: Arc::new(Mutex::new(false)),
        }
    }
}

impl IBMMainframeAdapter {
    /// Create a new IBM Mainframe adapter
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate JCL for a job
    async fn generate_jcl(&self, job: &LegacyJob) -> ToadStoolResult<String> {
        self.jcl_generator.lock().await.generate_jcl(job).await
    }

    /// Submit JCL job
    #[allow(clippy::cast_possible_truncation)] // label uses low bits of UUID only
    async fn submit_jcl_job(&self, jcl: &str) -> ToadStoolResult<Uuid> {
        let job_id = Uuid::new_v4();
        let job_name = format!("JOB{:08X}", job_id.as_u128() as u32);

        let mainframe_job = MainframeJob {
            job_id,
            job_name: job_name.clone(),
            job_class: "A".to_string(),
            priority: JobPriority::Normal,
            jcl_content: jcl.to_string(),
            status: JobStatus::Queued,
            start_time: None,
            end_time: None,
            output_datasets: vec![],
            return_code: None,
            job_log: String::new(),
        };

        self.active_jobs.write().await.insert(job_id, mainframe_job);

        // In a real implementation, this would submit to the mainframe job queue
        info!("Submitted JCL job {} to mainframe", job_name);

        Ok(job_id)
    }

    /// Connect to mainframe via 3270 terminal
    async fn connect_3270(&self) -> ToadStoolResult<()> {
        if let Some(ref config) = self.config {
            let mut term_3270 = Terminal3270::new();
            term_3270.connect(&config.connection).await?;
            *self.terminal_emulator.lock().await = Some(term_3270);

            *self.connected.lock().await = true;

            info!("Connected to IBM mainframe via 3270 terminal");
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for IBMMainframeAdapter {
    fn name(&self) -> &'static str {
        "IBM Mainframe Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![
            LegacySystemType::IbmSystem360,
            LegacySystemType::IbmSystem370,
            LegacySystemType::IbmZSeries,
        ]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing IBM Mainframe adapter");

        // Find mainframe configuration
        for (name, mainframe_config) in &config.mainframe_configs {
            if mainframe_config.system_type == LegacySystemType::IbmSystem360
                || mainframe_config.system_type == LegacySystemType::IbmSystem370
                || mainframe_config.system_type == LegacySystemType::IbmZSeries
            {
                self.config = Some(mainframe_config.clone());
                info!("Found IBM mainframe configuration: {}", name);
                break;
            }
        }

        if self.config.is_none() {
            return Err(ToadStoolError::runtime(
                "No IBM mainframe configuration found",
            ));
        }

        // Initialize components - config must be initialized before these calls
        let config = self.config.as_ref().ok_or_else(|| {
            ToadStoolError::configuration("Mainframe adapter config not initialized")
        })?;

        self.jcl_generator
            .lock()
            .await
            .initialize(&config.jcl_settings)
            .await?;
        self.cobol_compiler
            .lock()
            .await
            .initialize(&config.cobol_settings)
            .await?;
        self.dataset_manager
            .lock()
            .await
            .initialize(&config.datasets)
            .await?;

        // Connect to mainframe
        self.connect_3270().await?;

        info!("IBM Mainframe adapter initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down IBM Mainframe adapter");

        // Disconnect terminal
        let mut terminal = self.terminal_emulator.lock().await;
        if let Some(ref mut term) = *terminal {
            term.disconnect().await?;
        }
        *terminal = None;
        drop(terminal);

        *self.connected.lock().await = false;

        info!("IBM Mainframe adapter shutdown complete");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting job to IBM mainframe: {:?}", job.job_id);

        // Generate JCL for the job
        let jcl = self.generate_jcl(&job).await?;

        // Submit JCL job
        let job_id = self.submit_jcl_job(&jcl).await?;

        info!("Job submitted to IBM mainframe: {}", job_id);
        Ok(job_id)
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
            info!("Cancelled IBM mainframe job: {}", job_id);
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
        // In a real implementation, this would query the mainframe system
        Ok(SystemInfo {
            system_name: "IBM z/OS".to_string(),
            system_type: LegacySystemType::IbmZSeries,
            version: "2.4".to_string(),
            architecture: crate::LegacyArchitecture::IbmSystem360,
            cpu_info: crate::CpuInfo {
                model: "IBM z14".to_string(),
                speed: 5_200_000_000, // 5.2 GHz
                cores: 32,
                features: vec!["z/Architecture".to_string()],
                usage: 25.0,
            },
            memory_info: crate::MemoryInfo {
                total: 1024 * 1024 * 1024 * 1024,    // 1 TB
                available: 512 * 1024 * 1024 * 1024, // 512 GB
                used: 512 * 1024 * 1024 * 1024,      // 512 GB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 100 * 1024 * 1024 * 1024 * 1024,    // 100 TB
                available: 50 * 1024 * 1024 * 1024 * 1024, // 50 TB
                used: 50 * 1024 * 1024 * 1024 * 1024,      // 50 TB
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
    use super::super::types::{MainframeJob, Terminal3270Attributes, Terminal3270Key};
    use super::IBMMainframeAdapter;
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
    use uuid::Uuid;

    fn minimal_mainframe_config(system: LegacySystemType) -> MainframeConfig {
        MainframeConfig {
            system_type: system,
            connection: ConnectionSettings {
                host: "127.0.0.1".to_string(),
                port: 3270,
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
                compiler: "IGYCRCTL".to_string(),
                compile_options: vec![],
                link_options: vec![],
                runtime_options: vec![],
            },
        }
    }

    fn minimal_legacy_job(system: LegacySystemType, arch: LegacyArchitecture) -> LegacyJob {
        LegacyJob {
            job_id: Uuid::new_v4(),
            target_system: system,
            target_architecture: arch.clone(),
            job_type: LegacyJobType::Compilation {
                language: LegacyLanguage::COBOL,
                target_format: crate::TargetFormat::LoadModule,
            },
            source: crate::LegacyJobSource::SourceCode {
                language: LegacyLanguage::COBOL,
                code: "IDENTIFICATION DIVISION.".to_string(),
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
                    architecture: arch,
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
    fn ibm_adapter_default_new_and_debug() {
        let a = IBMMainframeAdapter::default();
        let b = IBMMainframeAdapter::new();
        let sa = format!("{:?}", a);
        let sb = format!("{:?}", b);
        assert!(sa.contains("IBMMainframeAdapter"), "{sa}");
        assert!(sb.contains("IBMMainframeAdapter"), "{sb}");
    }

    #[test]
    fn mainframe_job_terminal_attrs_keys_serde_roundtrip() {
        let job = MainframeJob {
            job_id: Uuid::nil(),
            job_name: "T".to_string(),
            job_class: "A".to_string(),
            priority: JobPriority::High,
            jcl_content: "//X".to_string(),
            status: crate::JobStatus::Queued,
            start_time: None,
            end_time: None,
            output_datasets: vec!["OUT".to_string()],
            return_code: Some(0),
            job_log: "log".to_string(),
        };
        let js = serde_json::to_string(&job).unwrap();
        let job2: MainframeJob = serde_json::from_str(&js).unwrap();
        assert_eq!(job.job_id, job2.job_id);
        assert_eq!(job.status, job2.status);

        let attrs = Terminal3270Attributes {
            width: 80,
            height: 24,
            color_support: true,
            extended_attributes: false,
        };
        let a2: Terminal3270Attributes =
            serde_json::from_str(&serde_json::to_string(&attrs).unwrap()).unwrap();
        assert_eq!(attrs.width, a2.width);

        for key in [
            Terminal3270Key::Enter,
            Terminal3270Key::PF(3),
            Terminal3270Key::String("x".to_string()),
        ] {
            let k2: Terminal3270Key =
                serde_json::from_str(&serde_json::to_string(&key).unwrap()).unwrap();
            assert_eq!(format!("{:?}", key), format!("{:?}", k2));
        }
    }

    #[test]
    fn mainframe_config_serde_roundtrip() {
        let c = minimal_mainframe_config(LegacySystemType::IbmZSeries);
        let c2: MainframeConfig =
            serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(c.system_type, c2.system_type);
        assert_eq!(c.jcl_settings.job_class, c2.jcl_settings.job_class);
    }

    #[tokio::test]
    async fn ibm_initialize_errors_without_mainframe_config() {
        let mut adapter = IBMMainframeAdapter::new();
        let err = adapter
            .initialize(&SpecialtyRuntimeConfig::default())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("IBM") || err.to_string().contains("configuration"));
    }

    #[tokio::test]
    async fn ibm_submit_fails_without_initialized_templates() {
        let adapter = IBMMainframeAdapter::new();
        let job = minimal_legacy_job(
            LegacySystemType::IbmZSeries,
            LegacyArchitecture::IbmSystem360,
        );
        let err = adapter.submit_job(job).await.unwrap_err();
        assert!(err.to_string().contains("JCL") || err.to_string().contains("template"));
    }

    #[tokio::test]
    async fn ibm_lifecycle_job_not_found_errors() {
        let mut adapter = IBMMainframeAdapter::new();
        let mut cfg = SpecialtyRuntimeConfig::default();
        cfg.mainframe_configs.insert(
            "ibm".to_string(),
            minimal_mainframe_config(LegacySystemType::IbmZSeries),
        );
        adapter.initialize(&cfg).await.unwrap();

        let missing = Uuid::nil();
        adapter.get_job_status(missing).await.unwrap_err();
        adapter.cancel_job(missing).await.unwrap_err();
        adapter.get_job_output(missing).await.unwrap_err();

        let job = minimal_legacy_job(
            LegacySystemType::IbmZSeries,
            LegacyArchitecture::IbmSystem360,
        );
        let id = adapter.submit_job(job).await.unwrap();
        assert_eq!(
            adapter.get_job_status(id).await.unwrap(),
            crate::JobStatus::Queued
        );
        adapter.cancel_job(id).await.unwrap();
        adapter.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn ibm_system_info_and_connectivity() {
        let mut adapter = IBMMainframeAdapter::new();
        assert!(!adapter.test_connectivity().await.unwrap());

        let mut cfg = SpecialtyRuntimeConfig::default();
        cfg.mainframe_configs.insert(
            "ibm".to_string(),
            minimal_mainframe_config(LegacySystemType::IbmSystem370),
        );
        adapter.initialize(&cfg).await.unwrap();
        assert!(adapter.test_connectivity().await.unwrap());

        let info = adapter.get_system_info().await.unwrap();
        assert_eq!(info.system_type, LegacySystemType::IbmZSeries);
        adapter.shutdown().await.unwrap();
    }
}

// Implementation for VAX/VMS Adapter
