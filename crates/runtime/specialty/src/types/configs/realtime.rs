// SPDX-License-Identifier: AGPL-3.0-only
//! Real-time systems configuration types

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Real-time system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConfig {
    /// Real-time OS
    pub rtos: RealtimeOS,
    /// Scheduling policy
    pub scheduling_policy: SchedulingPolicy,
    /// Task configuration
    pub tasks: Vec<TaskConfig>,
    /// Interrupt configuration
    pub interrupts: Vec<InterruptConfig>,
}

/// Real-time operating systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RealtimeOS {
    /// `VxWorks`
    VxWorks,
    /// QNX
    QNX,
    /// RT-11
    RT11,
    /// RTOS-32
    RTOS32,
    /// `FreeRTOS`
    FreeRTOS,
    /// embOS
    EmbOS,
    /// µC/OS
    MicroCOS,
    /// Custom real-time OS.
    Custom {
        /// RTOS name.
        name: String,
    },
}

/// Scheduling policies for real-time systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    /// Preemptive scheduling
    Preemptive,
    /// Cooperative scheduling
    Cooperative,
    /// Round-robin scheduling
    RoundRobin,
    /// Priority-based scheduling
    Priority,
    /// Rate-monotonic scheduling
    RateMonotonic,
    /// Earliest deadline first
    EarliestDeadlineFirst,
    /// Custom scheduling policy.
    Custom {
        /// Policy name.
        name: String,
    },
}

/// Task configuration for real-time systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Task name
    pub name: String,
    /// Task priority
    pub priority: u8,
    /// Stack size
    pub stack_size: u32,
    /// Task period
    pub period: Duration,
    /// Task deadline
    pub deadline: Duration,
    /// Task function
    pub function: String,
}

/// Interrupt configuration for real-time systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptConfig {
    /// Interrupt number
    pub interrupt_number: u8,
    /// Interrupt priority
    pub priority: u8,
    /// Interrupt handler
    pub handler: String,
    /// Interrupt type
    pub interrupt_type: InterruptType,
}

/// Interrupt types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptType {
    /// Hardware interrupt
    Hardware,
    /// Software interrupt
    Software,
    /// Timer interrupt
    Timer,
    /// External interrupt
    External,
    /// Custom interrupt type.
    Custom {
        /// Interrupt type name.
        name: String,
    },
}
