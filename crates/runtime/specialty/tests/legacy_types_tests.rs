// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive Tests for Legacy Runtime Types
//!
//! Tests cover:
//! - Job types and execution
//! - Platform emulation
//! - Cross-compilation targets
//! - Legacy system integration

use toadstool_runtime_legacy::types::jobs::*;
use toadstool_runtime_legacy::types::configs::*;
use std::time::Duration;
use uuid::Uuid;

#[test]
fn test_legacy_job_creation() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Mainframe,
        job_type: LegacyJobType::BatchProcessing,
        source_code: "PRINT 'Hello World'".to_string(),
        language: ProgrammingLanguage::COBOL,
        priority: JobPriority::Normal,
        timeout: Some(Duration::from_secs(300)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.platform, PlatformType::Mainframe);
    assert_eq!(job.language, ProgrammingLanguage::COBOL);
}

#[test]
fn test_legacy_job_type_variants() {
    assert_ne!(LegacyJobType::BatchProcessing, LegacyJobType::InteractiveSession);
    assert_ne!(LegacyJobType::InteractiveSession, LegacyJobType::Compilation);
    assert_ne!(LegacyJobType::Compilation, LegacyJobType::CrossCompilation);
}

#[test]
fn test_programming_language_variants() {
    assert_ne!(ProgrammingLanguage::COBOL, ProgrammingLanguage::Fortran);
    assert_ne!(ProgrammingLanguage::Fortran, ProgrammingLanguage::Assembly6502);
    assert_ne!(ProgrammingLanguage::Assembly6502, ProgrammingLanguage::C);
}

#[test]
fn test_job_priority_ordering() {
    // Test that priorities are properly distinct
    let low = JobPriority::Low;
    let normal = JobPriority::Normal;
    let high = JobPriority::High;
    let critical = JobPriority::Critical;
    
    assert_ne!(low, normal);
    assert_ne!(normal, high);
    assert_ne!(high, critical);
}

#[test]
fn test_legacy_job_result_success() {
    let result = LegacyJobResult {
        job_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        output: Some("Program executed successfully".to_string()),
        error: None,
        execution_time: Duration::from_secs(10),
        resources_used: ResourceUsage {
            cpu_time_ms: 5000,
            memory_peak_kb: 1024,
            io_operations: 100,
        },
    };
    
    assert_eq!(result.status, ExecutionStatus::Completed);
    assert!(result.output.is_some());
    assert!(result.error.is_none());
}

#[test]
fn test_legacy_job_result_failure() {
    let result = LegacyJobResult {
        job_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed,
        output: None,
        error: Some("Compilation error: syntax error".to_string()),
        execution_time: Duration::from_secs(1),
        resources_used: ResourceUsage {
            cpu_time_ms: 100,
            memory_peak_kb: 256,
            io_operations: 10,
        },
    };
    
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert!(result.error.is_some());
    assert!(result.output.is_none());
}

#[test]
fn test_execution_status_variants() {
    assert_ne!(ExecutionStatus::Queued, ExecutionStatus::Running);
    assert_ne!(ExecutionStatus::Running, ExecutionStatus::Completed);
    assert_ne!(ExecutionStatus::Completed, ExecutionStatus::Failed);
    assert_ne!(ExecutionStatus::Failed, ExecutionStatus::Cancelled);
}

#[test]
fn test_resource_usage_tracking() {
    let usage = ResourceUsage {
        cpu_time_ms: 1000,
        memory_peak_kb: 2048,
        io_operations: 500,
    };
    
    assert_eq!(usage.cpu_time_ms, 1000);
    assert_eq!(usage.memory_peak_kb, 2048);
    assert_eq!(usage.io_operations, 500);
}

#[test]
fn test_mainframe_job_jcl() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Mainframe,
        job_type: LegacyJobType::BatchProcessing,
        source_code: "//MYJOB JOB CLASS=A\n//STEP1 EXEC PGM=IEFBR14".to_string(),
        language: ProgrammingLanguage::JCL,
        priority: JobPriority::Normal,
        timeout: None,
        metadata: Default::default(),
    };
    
    assert_eq!(job.language, ProgrammingLanguage::JCL);
    assert!(job.source_code.contains("//MYJOB"));
}

#[test]
fn test_embedded_job_assembly() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Embedded,
        job_type: LegacyJobType::Compilation,
        source_code: "LDA #$FF\nSTA $0200".to_string(),
        language: ProgrammingLanguage::Assembly6502,
        priority: JobPriority::High,
        timeout: Some(Duration::from_secs(30)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.platform, PlatformType::Embedded);
    assert_eq!(job.language, ProgrammingLanguage::Assembly6502);
}

#[test]
fn test_realtime_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Realtime,
        job_type: LegacyJobType::InteractiveSession,
        source_code: "void task() { /* real-time code */ }".to_string(),
        language: ProgrammingLanguage::C,
        priority: JobPriority::Critical,
        timeout: Some(Duration::from_millis(100)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.priority, JobPriority::Critical);
    assert!(job.timeout.unwrap() < Duration::from_secs(1));
}

#[test]
fn test_industrial_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Industrial,
        job_type: LegacyJobType::Monitoring,
        source_code: "MONITOR PLC_STATUS".to_string(),
        language: ProgrammingLanguage::Ladder,
        priority: JobPriority::High,
        timeout: None,
        metadata: Default::default(),
    };
    
    assert_eq!(job.platform, PlatformType::Industrial);
    assert_eq!(job.language, ProgrammingLanguage::Ladder);
}

#[test]
fn test_cross_compilation_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Embedded,
        job_type: LegacyJobType::CrossCompilation,
        source_code: "int main() { return 0; }".to_string(),
        language: ProgrammingLanguage::C,
        priority: JobPriority::Normal,
        timeout: Some(Duration::from_secs(60)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.job_type, LegacyJobType::CrossCompilation);
}

#[test]
fn test_job_metadata() {
    let mut job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Mainframe,
        job_type: LegacyJobType::BatchProcessing,
        source_code: "PRINT 'Test'".to_string(),
        language: ProgrammingLanguage::COBOL,
        priority: JobPriority::Normal,
        timeout: None,
        metadata: Default::default(),
    };
    
    job.metadata.insert("author".to_string(), "test".to_string());
    job.metadata.insert("version".to_string(), "1.0".to_string());
    
    assert_eq!(job.metadata.len(), 2);
    assert_eq!(job.metadata.get("author"), Some(&"test".to_string()));
}

#[test]
fn test_legacy_job_clone() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Mainframe,
        job_type: LegacyJobType::BatchProcessing,
        source_code: "TEST".to_string(),
        language: ProgrammingLanguage::COBOL,
        priority: JobPriority::Normal,
        timeout: None,
        metadata: Default::default(),
    };
    
    let cloned = job.clone();
    assert_eq!(job.id, cloned.id);
    assert_eq!(job.platform, cloned.platform);
}

#[test]
fn test_legacy_job_result_clone() {
    let result = LegacyJobResult {
        job_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        output: Some("Success".to_string()),
        error: None,
        execution_time: Duration::from_secs(1),
        resources_used: ResourceUsage {
            cpu_time_ms: 1000,
            memory_peak_kb: 512,
            io_operations: 50,
        },
    };
    
    let cloned = result.clone();
    assert_eq!(result.job_id, cloned.job_id);
    assert_eq!(result.status, cloned.status);
}

#[test]
fn test_resource_usage_clone() {
    let usage = ResourceUsage {
        cpu_time_ms: 1500,
        memory_peak_kb: 4096,
        io_operations: 200,
    };
    
    let cloned = usage.clone();
    assert_eq!(usage.cpu_time_ms, cloned.cpu_time_ms);
    assert_eq!(usage.memory_peak_kb, cloned.memory_peak_kb);
}

#[test]
fn test_job_with_high_timeout() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Mainframe,
        job_type: LegacyJobType::BatchProcessing,
        source_code: "LONG RUNNING JOB".to_string(),
        language: ProgrammingLanguage::COBOL,
        priority: JobPriority::Low,
        timeout: Some(Duration::from_secs(3600)), // 1 hour
        metadata: Default::default(),
    };
    
    assert!(job.timeout.unwrap() > Duration::from_secs(1000));
}

#[test]
fn test_job_with_no_timeout() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Mainframe,
        job_type: LegacyJobType::InteractiveSession,
        source_code: "INTERACTIVE".to_string(),
        language: ProgrammingLanguage::REXX,
        priority: JobPriority::Normal,
        timeout: None,
        metadata: Default::default(),
    };
    
    assert!(job.timeout.is_none());
}

#[test]
fn test_programming_language_display() {
    // Test that languages can be displayed
    let cobol = ProgrammingLanguage::COBOL;
    let fortran = ProgrammingLanguage::Fortran;
    
    format!("{:?}", cobol);
    format!("{:?}", fortran);
}

#[test]
fn test_platform_type_display() {
    let mainframe = PlatformType::Mainframe;
    let embedded = PlatformType::Embedded;
    
    format!("{:?}", mainframe);
    format!("{:?}", embedded);
}

#[test]
fn test_job_priority_display() {
    let low = JobPriority::Low;
    let high = JobPriority::High;
    
    format!("{:?}", low);
    format!("{:?}", high);
}

#[test]
fn test_execution_status_display() {
    let running = ExecutionStatus::Running;
    let completed = ExecutionStatus::Completed;
    
    format!("{:?}", running);
    format!("{:?}", completed);
}

#[test]
fn test_fortran_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Legacy,
        job_type: LegacyJobType::Compilation,
        source_code: "      PROGRAM HELLO\n      PRINT *, 'Hello'\n      END".to_string(),
        language: ProgrammingLanguage::Fortran,
        priority: JobPriority::Normal,
        timeout: Some(Duration::from_secs(30)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.language, ProgrammingLanguage::Fortran);
    assert!(job.source_code.contains("PROGRAM"));
}

#[test]
fn test_pascal_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Legacy,
        job_type: LegacyJobType::Compilation,
        source_code: "program Hello;\nbegin\n  writeln('Hello');\nend.".to_string(),
        language: ProgrammingLanguage::Pascal,
        priority: JobPriority::Normal,
        timeout: Some(Duration::from_secs(30)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.language, ProgrammingLanguage::Pascal);
}

#[test]
fn test_basic_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Legacy,
        job_type: LegacyJobType::InteractiveSession,
        source_code: "10 PRINT \"HELLO\"\n20 END".to_string(),
        language: ProgrammingLanguage::BASIC,
        priority: JobPriority::Normal,
        timeout: None,
        metadata: Default::default(),
    };
    
    assert_eq!(job.language, ProgrammingLanguage::BASIC);
    assert!(job.source_code.starts_with("10"));
}

#[test]
fn test_job_result_with_high_resource_usage() {
    let result = LegacyJobResult {
        job_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        output: Some("Processed large dataset".to_string()),
        error: None,
        execution_time: Duration::from_secs(300),
        resources_used: ResourceUsage {
            cpu_time_ms: 250_000,
            memory_peak_kb: 1024 * 1024, // 1GB
            io_operations: 10_000,
        },
    };
    
    assert!(result.resources_used.memory_peak_kb > 1_000_000);
    assert!(result.resources_used.io_operations > 5000);
}

#[test]
fn test_job_result_with_low_resource_usage() {
    let result = LegacyJobResult {
        job_id: Uuid::new_v4(),
        status: ExecutionStatus::Completed,
        output: Some("Quick task".to_string()),
        error: None,
        execution_time: Duration::from_millis(100),
        resources_used: ResourceUsage {
            cpu_time_ms: 50,
            memory_peak_kb: 128,
            io_operations: 5,
        },
    };
    
    assert!(result.execution_time < Duration::from_secs(1));
    assert!(result.resources_used.cpu_time_ms < 100);
}

#[test]
fn test_cancelled_job_result() {
    let result = LegacyJobResult {
        job_id: Uuid::new_v4(),
        status: ExecutionStatus::Cancelled,
        output: None,
        error: Some("Job cancelled by user".to_string()),
        execution_time: Duration::from_secs(5),
        resources_used: ResourceUsage {
            cpu_time_ms: 2000,
            memory_peak_kb: 512,
            io_operations: 20,
        },
    };
    
    assert_eq!(result.status, ExecutionStatus::Cancelled);
    assert!(result.error.is_some());
}

#[test]
fn test_timeout_job_result() {
    let result = LegacyJobResult {
        job_id: Uuid::new_v4(),
        status: ExecutionStatus::Failed,
        output: None,
        error: Some("Job exceeded timeout limit".to_string()),
        execution_time: Duration::from_secs(300),
        resources_used: ResourceUsage {
            cpu_time_ms: 299_000,
            memory_peak_kb: 2048,
            io_operations: 1000,
        },
    };
    
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert!(result.error.unwrap().contains("timeout"));
}

#[test]
fn test_z80_assembly_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Embedded,
        job_type: LegacyJobType::Compilation,
        source_code: "LD A, 42\nHALT".to_string(),
        language: ProgrammingLanguage::AssemblyZ80,
        priority: JobPriority::Normal,
        timeout: Some(Duration::from_secs(10)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.language, ProgrammingLanguage::AssemblyZ80);
}

#[test]
fn test_68000_assembly_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Embedded,
        job_type: LegacyJobType::Compilation,
        source_code: "MOVE.L #0,D0\nRTS".to_string(),
        language: ProgrammingLanguage::Assembly68000,
        priority: JobPriority::Normal,
        timeout: Some(Duration::from_secs(10)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.language, ProgrammingLanguage::Assembly68000);
}

#[test]
fn test_ada_job() {
    let job = LegacyJob {
        id: Uuid::new_v4(),
        platform: PlatformType::Realtime,
        job_type: LegacyJobType::Compilation,
        source_code: "procedure Main is\nbegin\n  null;\nend Main;".to_string(),
        language: ProgrammingLanguage::Ada,
        priority: JobPriority::High,
        timeout: Some(Duration::from_secs(60)),
        metadata: Default::default(),
    };
    
    assert_eq!(job.language, ProgrammingLanguage::Ada);
    assert_eq!(job.platform, PlatformType::Realtime);
}

