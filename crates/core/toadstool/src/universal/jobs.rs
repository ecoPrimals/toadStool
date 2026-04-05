// SPDX-License-Identifier: AGPL-3.0-or-later
//! Job types and priority management

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use crate::resources::ResourceRequirements;

use super::types::PrimalContext;

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum JobPriority {
    /// Emergency - highest priority (level 0)
    Emergency = 0,
    /// Critical - very high priority (level 1)
    Critical = 1,
    /// High priority (level 2)
    High = 2,
    /// Normal priority (level 3)
    Normal = 3,
    /// Low priority (level 4)
    Low = 4,
    /// Background - lowest priority (level 5)
    Background = 5,
}

/// Universal job types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalJobType {
    /// Native process execution
    Native {
        /// Executable path or name.
        executable: String,
        /// Command-line arguments.
        args: Vec<String>,
        /// Environment variables.
        env: HashMap<String, String>,
    },
    /// WebAssembly execution
    Wasm {
        /// WASM module bytes.
        module: Vec<u8>,
        /// Arguments to pass.
        args: Vec<String>,
        /// Environment variables.
        env: HashMap<String, String>,
    },
    /// Primal delegation
    Primal {
        /// Target primal type.
        primal_type: String,
        /// Endpoint URL.
        endpoint: String,
        /// JSON payload.
        payload: serde_json::Value,
    },
    /// `BiomeOS` orchestration
    BiomeOS {
        /// Biome manifest JSON.
        biome_manifest: serde_json::Value,
        /// Team identifier.
        team_id: String,
    },
}

/// Universal job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalJob {
    /// Job ID
    pub id: Uuid,
    /// Job type
    pub job_type: UniversalJobType,
    /// Job priority
    pub priority: JobPriority,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Execution timeout
    pub timeout: Option<Duration>,
    /// Creation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
    /// Context
    pub context: PrimalContext,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::types::NetworkLocation;
    use crate::universal::types::PrimalContext;
    use crate::universal::types::SecurityLevel;
    use std::collections::HashMap;

    fn sample_primal_context() -> PrimalContext {
        PrimalContext {
            user_id: "user-1".to_string(),
            device_id: "device-1".to_string(),
            session_id: "session-1".to_string(),
            network_location: NetworkLocation {
                ip_address: "127.0.0.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::Emergency < JobPriority::Critical);
        assert!(JobPriority::Critical < JobPriority::High);
        assert!(JobPriority::High < JobPriority::Normal);
        assert!(JobPriority::Normal < JobPriority::Low);
        assert!(JobPriority::Low < JobPriority::Background);
        assert!(JobPriority::Emergency < JobPriority::Background);
    }

    #[test]
    fn test_job_priority_serialization_round_trip() {
        for priority in [
            JobPriority::Emergency,
            JobPriority::Critical,
            JobPriority::High,
            JobPriority::Normal,
            JobPriority::Low,
            JobPriority::Background,
        ] {
            let json = serde_json::to_string(&priority).expect("serialize");
            let parsed: JobPriority = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(priority, parsed);
        }
    }

    #[test]
    fn test_universal_job_type_native_construction() {
        let mut env = HashMap::new();
        env.insert("VAR".to_string(), "value".to_string());
        let job_type = UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            env,
        };
        let json = serde_json::to_string(&job_type).expect("serialize");
        let parsed: UniversalJobType = serde_json::from_str(&json).expect("deserialize");
        if let UniversalJobType::Native {
            executable,
            args,
            env: e,
        } = parsed
        {
            assert_eq!(executable, "/bin/echo");
            assert_eq!(args, vec!["hello".to_string()]);
            assert_eq!(e.get("VAR"), Some(&"value".to_string()));
        } else {
            panic!("expected Native variant");
        }
    }

    #[test]
    fn test_universal_job_type_wasm_construction() {
        let mut env = HashMap::new();
        env.insert("WASM".to_string(), "1".to_string());
        let job_type = UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env,
        };
        let json = serde_json::to_string(&job_type).expect("serialize");
        let parsed: UniversalJobType = serde_json::from_str(&json).expect("deserialize");
        if let UniversalJobType::Wasm { module, .. } = parsed {
            assert_eq!(module, vec![0x00, 0x61, 0x73, 0x6d]);
        } else {
            panic!("expected Wasm variant");
        }
    }

    #[test]
    fn test_universal_job_type_primal_construction() {
        let job_type = UniversalJobType::Primal {
            primal_type: "compute".to_string(),
            endpoint: "http://localhost:8080/run".to_string(),
            payload: serde_json::json!({"task": "inference"}),
        };
        let json = serde_json::to_string(&job_type).expect("serialize");
        let parsed: UniversalJobType = serde_json::from_str(&json).expect("deserialize");
        if let UniversalJobType::Primal {
            primal_type,
            endpoint,
            payload,
        } = parsed
        {
            assert_eq!(primal_type, "compute");
            assert_eq!(endpoint, "http://localhost:8080/run");
            assert_eq!(payload["task"], "inference");
        } else {
            panic!("expected Primal variant");
        }
    }

    #[test]
    fn test_universal_job_type_biome_os_construction() {
        let job_type = UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({"version": "1"}),
            team_id: "team-42".to_string(),
        };
        let json = serde_json::to_string(&job_type).expect("serialize");
        let parsed: UniversalJobType = serde_json::from_str(&json).expect("deserialize");
        if let UniversalJobType::BiomeOS {
            biome_manifest,
            team_id,
        } = parsed
        {
            assert_eq!(biome_manifest["version"], "1");
            assert_eq!(team_id, "team-42");
        } else {
            panic!("expected BiomeOS variant");
        }
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact comparison intended in this context"
    )] // default ResourceRequirements value
    fn test_universal_job_construction_with_all_fields() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "val".to_string());
        let job = UniversalJob {
            id: Uuid::new_v4(),
            job_type: UniversalJobType::Native {
                executable: "true".to_string(),
                args: vec![],
                env: env.clone(),
            },
            priority: JobPriority::High,
            resources: ResourceRequirements::default(),
            timeout: Some(Duration::from_secs(60)),
            created_at: SystemTime::now(),
            context: sample_primal_context(),
        };
        assert_eq!(job.priority, JobPriority::High);
        assert_eq!(job.timeout, Some(Duration::from_secs(60)));
        assert_eq!(job.resources.cpu.min_cores, 1.0);
    }

    #[test]
    fn test_universal_job_with_default_resources() {
        let job = UniversalJob {
            id: Uuid::new_v4(),
            job_type: UniversalJobType::Primal {
                primal_type: "test".to_string(),
                endpoint: "http://test".to_string(),
                payload: serde_json::Value::Null,
            },
            priority: JobPriority::Normal,
            resources: ResourceRequirements::default(),
            timeout: None,
            created_at: SystemTime::now(),
            context: sample_primal_context(),
        };
        assert!(job.timeout.is_none());
        assert_eq!(job.priority, JobPriority::Normal);
    }
}
