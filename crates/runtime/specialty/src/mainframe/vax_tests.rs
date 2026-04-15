// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::types::{MainframeJob, VAXTerminalAttributes, VMSFileSpec};
use super::VAXVMSAdapter;
use crate::{
    AuthenticationSettings, AuthenticationType, COBOLSettings, CommunicationRequirements,
    CommunicationSettings, CompilationRequirements, CompilerType, ConnectionSettings,
    CpuRequirements, FileSystemType, JCLSettings, LegacyAdapter, LegacyArchitecture, LegacyJob,
    LegacyJobType, LegacyLanguage, LegacyRuntimeRequirements, LegacySystemType, MainframeConfig,
    MainframeConnectionType, MemoryModel, MemoryRequirements, MemoryType, NetworkRequirements,
    SpecialtyRuntimeConfig, StorageRequirements, StorageType, TimingRequirements,
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
    let c2: MainframeConfig = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
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
