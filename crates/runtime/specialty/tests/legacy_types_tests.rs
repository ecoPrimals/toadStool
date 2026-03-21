// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for legacy runtime types
//!
//! Tests cover types from `toadstool_runtime_specialty`:
//! - LegacyJob creation and serialization
//! - LegacyJobType, LegacyLanguage, LegacyJobSource variants
//! - LegacySystemType, LegacyArchitecture
//! - JobStatus, JobOutput
//! - CompilationRequirements, LegacyRuntimeRequirements

use std::path::PathBuf;
use std::time::Duration;
use toadstool_runtime_specialty::LegacyArchitecture;
use toadstool_runtime_specialty::LegacySystemType;
use toadstool_runtime_specialty::types::configs::CommunicationSettings;
use toadstool_runtime_specialty::types::jobs::*;
use toadstool_runtime_specialty::types::requirements::{
    self, CompilationRequirements, LegacyRuntimeRequirements, *,
};
use toadstool_runtime_specialty::types::systems::*;
use toadstool_runtime_specialty::types::traits::*;
use uuid::Uuid;

fn minimal_compilation_requirements() -> CompilationRequirements {
    CompilationRequirements {
        compiler: CompilerType::MicrosoftC60,
        flags: vec![],
        include_paths: vec![],
        library_paths: vec![],
        libraries: vec![],
        memory_model: MemoryModel::Flat,
        optimization: requirements::OptimizationLevel::None,
        debug_info: false,
    }
}

fn minimal_runtime_requirements() -> LegacyRuntimeRequirements {
    LegacyRuntimeRequirements {
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
    }
}

#[test]
fn test_legacy_job_creation_compilation() {
    let job = LegacyJob {
        job_id: Uuid::new_v4(),
        target_system: LegacySystemType::Intel8086,
        target_architecture: LegacyArchitecture::Intel8086,
        job_type: LegacyJobType::Compilation {
            language: LegacyLanguage::Ckr,
            target_format: TargetFormat::Executable,
        },
        source: LegacyJobSource::SourceCode {
            language: LegacyLanguage::Ckr,
            code: "int main() { return 0; }".to_string(),
        },
        compilation_requirements: minimal_compilation_requirements(),
        runtime_requirements: minimal_runtime_requirements(),
        communication_settings: CommunicationSettings::default(),
        priority: toadstool::JobPriority::Normal,
        created_at: std::time::SystemTime::now(),
        timeout: Duration::from_secs(3600),
    };
    assert!(matches!(job.job_type, LegacyJobType::Compilation { .. }));
    assert!(matches!(job.source, LegacyJobSource::SourceCode { .. }));
}

#[test]
fn test_legacy_job_serialization_roundtrip() {
    let job = LegacyJob {
        job_id: Uuid::new_v4(),
        target_system: LegacySystemType::IbmSystem360,
        target_architecture: LegacyArchitecture::IbmSystem360,
        job_type: LegacyJobType::Compilation {
            language: LegacyLanguage::COBOL,
            target_format: TargetFormat::Executable,
        },
        source: LegacyJobSource::SourceCode {
            language: LegacyLanguage::COBOL,
            code: "IDENTIFICATION DIVISION.".to_string(),
        },
        compilation_requirements: minimal_compilation_requirements(),
        runtime_requirements: minimal_runtime_requirements(),
        communication_settings: CommunicationSettings::default(),
        priority: toadstool::JobPriority::Normal,
        created_at: std::time::SystemTime::now(),
        timeout: Duration::from_secs(300),
    };
    let json = serde_json::to_string(&job).unwrap();
    let deserialized: LegacyJob = serde_json::from_str(&json).unwrap();
    assert_eq!(job.job_id, deserialized.job_id);
    assert_eq!(job.target_system, deserialized.target_system);
}

#[test]
fn test_legacy_job_type_variants() {
    let compilation = LegacyJobType::Compilation {
        language: LegacyLanguage::Ckr,
        target_format: TargetFormat::Executable,
    };
    assert!(matches!(compilation, LegacyJobType::Compilation { .. }));

    let execution = LegacyJobType::Execution {
        program_format: ProgramFormat::DosExe,
        arguments: vec!["arg1".to_string()],
    };
    assert!(matches!(execution, LegacyJobType::Execution { .. }));

    let file_transfer = LegacyJobType::FileTransfer {
        transfer_type: TransferType::Upload,
        source_path: PathBuf::from("/src"),
        destination_path: PathBuf::from("/dst"),
    };
    assert!(matches!(file_transfer, LegacyJobType::FileTransfer { .. }));
}

#[test]
fn test_legacy_language_variants() {
    let languages = [
        LegacyLanguage::COBOL,
        LegacyLanguage::Fortran77,
        LegacyLanguage::Assembly6502,
        LegacyLanguage::Ckr,
        LegacyLanguage::JCL,
    ];
    for lang in languages {
        let json = serde_json::to_string(&lang).unwrap();
        let _round: LegacyLanguage = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_legacy_job_source_variants() {
    let source_code = LegacyJobSource::SourceCode {
        language: LegacyLanguage::BASIC,
        code: "10 PRINT \"HELLO\"".to_string(),
    };
    let json = serde_json::to_string(&source_code).unwrap();
    let _round: LegacyJobSource = serde_json::from_str(&json).unwrap();

    let source_file = LegacyJobSource::SourceFile {
        language: LegacyLanguage::Ckr,
        file_path: PathBuf::from("/tmp/main.c"),
    };
    let json = serde_json::to_string(&source_file).unwrap();
    let _round: LegacyJobSource = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_legacy_system_type_serialization() {
    let systems = [
        LegacySystemType::IbmSystem360,
        LegacySystemType::VaxVms,
        LegacySystemType::PDP11,
        LegacySystemType::Intel8080,
        LegacySystemType::MOS6502,
        LegacySystemType::VxWorks,
    ];
    for sys in systems {
        let json = serde_json::to_string(&sys).unwrap();
        let round: LegacySystemType = serde_json::from_str(&json).unwrap();
        assert_eq!(sys, round);
    }
}

#[test]
fn test_legacy_architecture_serialization() {
    let archs = [
        LegacyArchitecture::Intel8086,
        LegacyArchitecture::MOS6502,
        LegacyArchitecture::PDP11,
        LegacyArchitecture::VAX,
    ];
    for arch in archs {
        let json = serde_json::to_string(&arch).unwrap();
        let round: LegacyArchitecture = serde_json::from_str(&json).unwrap();
        assert_eq!(arch, round);
    }
}

#[test]
fn test_system_status() {
    assert_eq!(SystemStatus::Online, SystemStatus::Online);
    assert_eq!(SystemStatus::Offline, SystemStatus::Offline);
    let default: SystemStatus = Default::default();
    assert!(matches!(default, SystemStatus::Unknown));
}

#[test]
fn test_job_status_variants() {
    let queued = JobStatus::Queued;
    assert!(matches!(queued, JobStatus::Queued));

    let running = JobStatus::Running;
    assert!(matches!(running, JobStatus::Running));

    let completed = JobStatus::Completed;
    assert!(matches!(completed, JobStatus::Completed));

    let failed = JobStatus::Failed {
        error: "test error".to_string(),
    };
    assert!(matches!(failed, JobStatus::Failed { .. }));

    let json = serde_json::to_string(&failed).unwrap();
    let _round: JobStatus = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_job_output() {
    let output = JobOutput {
        stdout: "Hello".to_string(),
        stderr: "".to_string(),
        return_code: Some(0),
        output_files: vec![],
        binary_output: None,
    };
    let json = serde_json::to_string(&output).unwrap();
    let deserialized: JobOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(output.stdout, deserialized.stdout);
    assert_eq!(output.return_code, deserialized.return_code);
}

#[test]
fn test_compilation_requirements_roundtrip() {
    let req = minimal_compilation_requirements();
    let json = serde_json::to_string(&req).unwrap();
    let _round: CompilationRequirements = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_compiler_type_variants() {
    let compiler = CompilerType::MicrosoftC60;
    let json = serde_json::to_string(&compiler).unwrap();
    let _round: CompilerType = serde_json::from_str(&json).unwrap();

    let cross = CompilerType::CrossCompiler {
        host_arch: "x86_64".to_string(),
        target_arch: LegacyArchitecture::Intel8086,
    };
    let json = serde_json::to_string(&cross).unwrap();
    let _round: CompilerType = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_program_format_variants() {
    let formats = [
        ProgramFormat::Binary,
        ProgramFormat::DosExe,
        ProgramFormat::IntelHex,
        ProgramFormat::IbmLoadModule,
    ];
    for f in formats {
        let json = serde_json::to_string(&f).unwrap();
        let _round: ProgramFormat = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_legacy_job_jcl_source() {
    let job = LegacyJob {
        job_id: Uuid::new_v4(),
        target_system: LegacySystemType::IbmSystem360,
        target_architecture: LegacyArchitecture::IbmSystem360,
        job_type: LegacyJobType::Compilation {
            language: LegacyLanguage::JCL,
            target_format: TargetFormat::LoadModule,
        },
        source: LegacyJobSource::JCL {
            jcl_text: "//MYJOB JOB CLASS=A".to_string(),
            datasets: std::collections::HashMap::new(),
        },
        compilation_requirements: minimal_compilation_requirements(),
        runtime_requirements: minimal_runtime_requirements(),
        communication_settings: CommunicationSettings::default(),
        priority: toadstool::JobPriority::Normal,
        created_at: std::time::SystemTime::now(),
        timeout: Duration::from_secs(600),
    };
    assert!(matches!(job.source, LegacyJobSource::JCL { .. }));
}

#[test]
fn test_legacy_job_interactive_session() {
    use toadstool_runtime_specialty::types::configs::SessionConfig;
    use toadstool_runtime_specialty::types::configs::{CharacterEncoding, FlowControl, LineEnding};
    let job = LegacyJob {
        job_id: Uuid::new_v4(),
        target_system: LegacySystemType::PDP11,
        target_architecture: LegacyArchitecture::PDP11,
        job_type: LegacyJobType::InteractiveSession {
            terminal_type: TerminalType::VT100,
            session_config: SessionConfig {
                width: 80,
                height: 24,
                line_ending: LineEnding::Unix,
                encoding: CharacterEncoding::ASCII,
                flow_control: FlowControl::None,
            },
        },
        source: LegacyJobSource::SourceCode {
            language: LegacyLanguage::ShellBourne,
            code: "echo hello".to_string(),
        },
        compilation_requirements: minimal_compilation_requirements(),
        runtime_requirements: minimal_runtime_requirements(),
        communication_settings: CommunicationSettings::default(),
        priority: toadstool::JobPriority::Normal,
        created_at: std::time::SystemTime::now(),
        timeout: Duration::from_secs(30),
    };
    assert!(matches!(
        job.job_type,
        LegacyJobType::InteractiveSession { .. }
    ));
}

#[test]
fn test_specialty_runtime_metrics_default() {
    let metrics = SpecialtyRuntimeMetrics::default();
    assert_eq!(metrics.total_jobs, 0);
    assert_eq!(metrics.successful_jobs, 0);
    assert_eq!(metrics.failed_jobs, 0);
}
