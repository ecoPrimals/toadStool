// SPDX-License-Identifier: AGPL-3.0-only
//! Unit tests for specialty runtime engine

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use uuid::Uuid;

    use crate::config::SpecialtyRuntimeConfig;
    use crate::engine::SpecialtyRuntimeEngine;
    use toadstool::RuntimeEngine;
    use crate::types::configs::CommunicationSettings;
    use crate::types::jobs::{LegacyJob, LegacyJobSource, LegacyJobType, LegacyLanguage, TargetFormat};
    use crate::types::requirements::{
        CommunicationRequirements, CompilationRequirements, CompilerType, CpuRequirements,
        LegacyRuntimeRequirements, MemoryModel, MemoryRequirements, MemoryType,
        NetworkRequirements, StorageRequirements, StorageType, TimingRequirements,
    };
    use crate::types::systems::{LegacyArchitecture, LegacySystemType};
    use toadstool::WorkloadType;

    #[tokio::test]
    async fn test_specialty_runtime_engine_creation() {
        let config = SpecialtyRuntimeConfig::default();
        let engine = SpecialtyRuntimeEngine::new(config);

        let caps = engine.get_capabilities();
        assert!(caps.supported_workloads.contains(&WorkloadType::Native));
    }

    #[tokio::test]
    async fn test_legacy_system_types() {
        let systems = vec![
            LegacySystemType::IbmSystem360,
            LegacySystemType::VaxVms,
            LegacySystemType::AS400,
            LegacySystemType::PDP11,
            LegacySystemType::Intel8080,
            LegacySystemType::MOS6502,
            LegacySystemType::VxWorks,
        ];

        for system in systems {
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
                language: LegacyLanguage::Ckr,
                target_format: TargetFormat::Executable,
            },
            source: LegacyJobSource::SourceCode {
                language: LegacyLanguage::Ckr,
                code: "int main() { return 0; }".to_string(),
            },
            compilation_requirements: CompilationRequirements {
                compiler: CompilerType::MicrosoftC60,
                flags: vec![],
                include_paths: vec![],
                library_paths: vec![],
                libraries: vec![],
                memory_model: MemoryModel::Flat,
                optimization: crate::types::requirements::OptimizationLevel::None,
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
                    file_system: crate::types::requirements::FileSystemType::DOS,
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
            priority: toadstool::JobPriority::Normal,
            created_at: std::time::SystemTime::now(),
            timeout: Duration::from_secs(3600),
        };

        let serialized = serde_json::to_string(&job).unwrap();
        let deserialized: LegacyJob = serde_json::from_str(&serialized).unwrap();
        assert_eq!(job.job_id, deserialized.job_id);
        assert_eq!(job.target_system, deserialized.target_system);
    }
}
