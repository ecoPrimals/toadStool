// SPDX-License-Identifier: AGPL-3.0-or-later
//! System management configuration types for legacy systems

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import canonical JobPriority for conversions
use toadstool::JobPriority as CanonicalJobPriority;

/// File transfer types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferType {
    /// Upload to legacy system
    Upload,
    /// Download from legacy system
    Download,
    /// Bidirectional transfer
    Bidirectional,
}

/// System monitoring types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringType {
    /// CPU usage
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
    /// Custom monitoring
    Custom {
        name: String,
        parameters: HashMap<String, String>,
    },
}

/// System administration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdministrationType {
    /// User management
    UserManagement,
    /// File system management
    FileSystemManagement,
    /// Process management
    ProcessManagement,
    /// System configuration
    SystemConfiguration,
    /// Backup and restore
    BackupRestore,
    /// Custom administration
    Custom { name: String },
}

/// Legacy job priorities (for backward compatibility with legacy systems)
///
/// Note: For new code, use `toadstool::JobPriority` (canonical definition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobPriority {
    /// Low priority
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
            CanonicalJobPriority::Low => Self::Low,
            CanonicalJobPriority::Background => Self::Low, // Map Background to Low
        }
    }
}
