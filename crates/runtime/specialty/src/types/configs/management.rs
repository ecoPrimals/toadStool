// SPDX-License-Identifier: AGPL-3.0-only
//! System management configuration types for legacy systems

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import canonical JobPriority for conversions
use toadstool::JobPriority as CanonicalJobPriority;

/// File transfer types
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum TransferType {
    /// Upload to legacy system
    #[default]
    Upload,
    /// Download from legacy system
    Download,
    /// Bidirectional transfer
    Bidirectional,
}

/// System monitoring types
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum MonitoringType {
    /// CPU usage
    #[default]
    CPU,
    /// Memory usage
    Memory,
    /// Storage usage
    Storage,
    /// Network traffic
    Network,
    /// System performance
    Performance,
    /// Process monitoring
    Process,
    /// Custom monitoring type.
    Custom {
        /// Monitoring type name.
        name: String,
        /// Configuration parameters.
        parameters: HashMap<String, String>,
    },
}

/// System administration types
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum AdministrationType {
    /// User management
    #[default]
    UserManagement,
    /// File system management
    FileSystemManagement,
    /// Process management
    ProcessManagement,
    /// System configuration
    SystemConfiguration,
    /// Backup and restore
    BackupRestore,
    /// Custom administration type.
    Custom {
        /// Administration type name.
        name: String,
    },
}

/// Legacy job priorities (for backward compatibility with legacy systems)
///
/// Note: For new code, use `toadstool::JobPriority` (canonical definition)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum JobPriority {
    /// Low priority
    #[default]
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
    /// Real-time priority (maps to Emergency in canonical)
    RealTime,
}

impl From<JobPriority> for CanonicalJobPriority {
    fn from(legacy: JobPriority) -> Self {
        match legacy {
            JobPriority::Low => Self::Low,
            JobPriority::Normal => Self::Normal,
            JobPriority::High => Self::High,
            JobPriority::Critical => Self::Critical,
            JobPriority::RealTime => Self::Emergency,
        }
    }
}

impl From<CanonicalJobPriority> for JobPriority {
    fn from(canonical: CanonicalJobPriority) -> Self {
        match canonical {
            CanonicalJobPriority::Emergency => Self::RealTime,
            CanonicalJobPriority::Critical => Self::Critical,
            CanonicalJobPriority::High => Self::High,
            CanonicalJobPriority::Normal => Self::Normal,
            CanonicalJobPriority::Low | CanonicalJobPriority::Background => Self::Low, // Background → Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn assert_serde_json_stable<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).expect("serde_json serialize");
        let back: T = serde_json::from_str(&json).expect("serde_json deserialize");
        let json_again = serde_json::to_string(&back).expect("serde_json re-serialize");
        assert_eq!(
            json, json_again,
            "serde round-trip must preserve JSON representation"
        );
    }

    #[test]
    fn transfer_type_default_clone_debug_serde() {
        assert!(matches!(TransferType::default(), TransferType::Upload));
        for t in [
            TransferType::Upload,
            TransferType::Download,
            TransferType::Bidirectional,
        ] {
            assert_serde_json_stable(&t);
            assert_serde_json_stable(&t.clone());
        }
        let dbg = format!("{:?}", TransferType::Download);
        assert!(dbg.contains("Download"));
    }

    #[test]
    fn monitoring_type_default_clone_debug_serde() {
        assert!(matches!(MonitoringType::default(), MonitoringType::CPU));
        let custom = MonitoringType::Custom {
            name: "metrics".to_string(),
            parameters: HashMap::from([("k".to_string(), "v".to_string())]),
        };
        assert_serde_json_stable(&custom);
        assert_serde_json_stable(&custom);
        for m in [
            MonitoringType::Memory,
            MonitoringType::Storage,
            MonitoringType::Network,
            MonitoringType::Performance,
            MonitoringType::Process,
        ] {
            assert_serde_json_stable(&m);
        }
        let dbg = format!("{:?}", MonitoringType::Network);
        assert!(dbg.contains("Network"));
    }

    #[test]
    fn administration_type_default_clone_debug_serde() {
        assert!(matches!(
            AdministrationType::default(),
            AdministrationType::UserManagement
        ));
        let custom = AdministrationType::Custom {
            name: "ldap".to_string(),
        };
        assert_serde_json_stable(&custom);
        assert_serde_json_stable(&custom);
        for a in [
            AdministrationType::FileSystemManagement,
            AdministrationType::ProcessManagement,
            AdministrationType::SystemConfiguration,
            AdministrationType::BackupRestore,
        ] {
            assert_serde_json_stable(&a);
        }
        let dbg = format!("{:?}", AdministrationType::BackupRestore);
        assert!(dbg.contains("BackupRestore"));
    }

    #[test]
    fn job_priority_default_clone_debug_serde() {
        assert!(matches!(JobPriority::default(), JobPriority::Low));
        for p in [
            JobPriority::Normal,
            JobPriority::High,
            JobPriority::Critical,
            JobPriority::RealTime,
        ] {
            assert_serde_json_stable(&p);
            assert_serde_json_stable(&p.clone());
        }
        let dbg = format!("{:?}", JobPriority::Critical);
        assert!(dbg.contains("Critical"));
    }

    #[test]
    fn job_priority_from_legacy_to_canonical() {
        let cases = [
            (JobPriority::Low, CanonicalJobPriority::Low),
            (JobPriority::Normal, CanonicalJobPriority::Normal),
            (JobPriority::High, CanonicalJobPriority::High),
            (JobPriority::Critical, CanonicalJobPriority::Critical),
            (JobPriority::RealTime, CanonicalJobPriority::Emergency),
        ];
        for (legacy, canonical) in cases {
            let c: CanonicalJobPriority = legacy.into();
            assert_eq!(c, canonical);
        }
    }

    #[test]
    fn job_priority_from_canonical_to_legacy() {
        let cases = [
            (CanonicalJobPriority::Low, JobPriority::Low),
            (CanonicalJobPriority::Normal, JobPriority::Normal),
            (CanonicalJobPriority::High, JobPriority::High),
            (CanonicalJobPriority::Critical, JobPriority::Critical),
            (CanonicalJobPriority::Emergency, JobPriority::RealTime),
            (CanonicalJobPriority::Background, JobPriority::Low),
        ];
        for (canonical, legacy) in cases {
            let l: JobPriority = canonical.into();
            assert_eq!(std::mem::discriminant(&l), std::mem::discriminant(&legacy));
            let json_l = serde_json::to_string(&l).unwrap();
            let json_e = serde_json::to_string(&legacy).unwrap();
            assert_eq!(json_l, json_e);
        }
    }

    #[test]
    fn job_priority_roundtrip_through_canonical() {
        for p in [
            JobPriority::Low,
            JobPriority::Normal,
            JobPriority::High,
            JobPriority::Critical,
            JobPriority::RealTime,
        ] {
            let c: CanonicalJobPriority = p.clone().into();
            let back: JobPriority = c.into();
            assert_serde_json_stable(&p);
            assert_eq!(
                serde_json::to_string(&p).unwrap(),
                serde_json::to_string(&back).unwrap()
            );
        }
    }
}
