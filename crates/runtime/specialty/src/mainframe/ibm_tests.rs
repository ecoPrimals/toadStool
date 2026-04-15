// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::types::{MainframeJob, Terminal3270Attributes, Terminal3270Key};
use super::IBMMainframeAdapter;
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

fn minimal_mainframe_config(system: LegacySystemType) -> MainframeConfig {
    MainframeConfig {
        system_type: system,
        connection: ConnectionSettings {
            host: LOCALHOST_IPV4.to_string(),
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
    let c2: MainframeConfig = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
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
