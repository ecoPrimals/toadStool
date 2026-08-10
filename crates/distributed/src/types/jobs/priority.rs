// SPDX-License-Identifier: AGPL-3.0-or-later
//! Job priority levels for distributed scheduling.

use serde::{Deserialize, Serialize};

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

#[cfg(feature = "runtime")]
impl From<JobPriority> for toadstool::JobPriority {
    fn from(priority: JobPriority) -> Self {
        match priority {
            JobPriority::Emergency => Self::Emergency,
            JobPriority::Critical => Self::Critical,
            JobPriority::High => Self::High,
            JobPriority::Normal => Self::Normal,
            JobPriority::Low => Self::Low,
            JobPriority::Background => Self::Background,
        }
    }
}

#[cfg(feature = "runtime")]
impl From<toadstool::JobPriority> for JobPriority {
    fn from(priority: toadstool::JobPriority) -> Self {
        match priority {
            toadstool::JobPriority::Emergency => Self::Emergency,
            toadstool::JobPriority::Critical => Self::Critical,
            toadstool::JobPriority::High => Self::High,
            toadstool::JobPriority::Normal => Self::Normal,
            toadstool::JobPriority::Low => Self::Low,
            toadstool::JobPriority::Background => Self::Background,
        }
    }
}
