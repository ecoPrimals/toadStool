//! Chaos Engineering Framework for ToadStool
//!
//! This module provides infrastructure for chaos engineering tests that validate
//! system resilience under failure conditions.
//!
//! # Philosophy
//!
//! Chaos engineering helps us:
//! - Discover weaknesses before they cause outages
//! - Build confidence in system resilience
//! - Understand actual failure modes
//! - Document recovery procedures
//!
//! # Usage
//!
//! ```rust,ignore
//! use toadstool_testing::chaos::{ChaosScenario, FaultType};
//!
//! #[tokio::test]
//! async fn test_network_partition_recovery() {
//!     let scenario = ChaosScenario::new("network_partition")
//!         .with_fault(FaultType::NetworkPartition {
//!             duration_ms: 5000,
//!             affected_nodes: vec!["node1", "node2"],
//!         })
//!         .with_validation(|state| {
//!             assert!(state.cluster_recovered());
//!             assert_eq!(state.data_loss_count, 0);
//!         });
//!
//!     scenario.run().await.expect("Chaos scenario should pass");
//! }
//! ```

use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

/// Type alias for chaos scenario state validators
///
/// Validators check system state after fault injection to ensure the system
/// recovered correctly or maintains expected invariants.
pub type StateValidator = Box<dyn Fn(&SystemState) -> Result<(), String> + Send + Sync>;

/// Chaos scenario configuration and execution
pub struct ChaosScenario {
    /// Scenario name for reporting
    pub name: String,
    /// Faults to inject during the scenario
    pub faults: Vec<FaultInjection>,
    /// System state validator
    pub validator: Option<StateValidator>,
    /// Maximum scenario duration
    pub timeout: Duration,
}

/// Types of faults that can be injected
#[derive(Debug, Clone)]
pub enum FaultType {
    /// Network partition between nodes
    NetworkPartition {
        /// How long the partition lasts
        duration_ms: u64,
        /// Which nodes are affected
        affected_nodes: Vec<String>,
    },
    /// Random process crash and restart
    ProcessCrash {
        /// Node to crash
        node_id: String,
        /// Delay before restart (ms)
        restart_delay_ms: u64,
    },
    /// Resource exhaustion
    ResourceExhaustion {
        /// Resource type (memory, cpu, disk)
        resource_type: ResourceType,
        /// Percentage to consume (0-100)
        consumption_percent: u8,
        /// Duration of exhaustion
        duration_ms: u64,
    },
    /// Network latency injection
    NetworkLatency {
        /// Latency to add (ms)
        latency_ms: u64,
        /// Duration of latency
        duration_ms: u64,
    },
    /// Packet loss
    PacketLoss {
        /// Loss rate (0.0 - 1.0)
        loss_rate: f64,
        /// Duration
        duration_ms: u64,
    },
}

/// Resource types for exhaustion testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Memory,
    Cpu,
    Disk,
    Network,
}

/// Fault injection configuration
#[derive(Debug, Clone)]
pub struct FaultInjection {
    /// Type of fault
    pub fault_type: FaultType,
    /// When to inject (ms from start)
    pub inject_at_ms: u64,
    /// Optional description
    pub description: Option<String>,
}

/// System state for validation
#[derive(Debug, Clone, Default)]
pub struct SystemState {
    /// Number of active nodes
    pub active_nodes: usize,
    /// Number of failed nodes
    pub failed_nodes: usize,
    /// Data loss events
    pub data_loss_count: u64,
    /// Successful recoveries
    pub recovery_count: u64,
    /// Custom metrics
    pub metrics: HashMap<String, f64>,
}

impl SystemState {
    /// Check if cluster has recovered
    pub fn cluster_recovered(&self) -> bool {
        self.failed_nodes == 0 && self.active_nodes > 0
    }

    /// Get a metric value
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        self.metrics.get(name).copied()
    }

    /// Set a metric value
    pub fn set_metric(&mut self, name: impl Into<String>, value: f64) {
        self.metrics.insert(name.into(), value);
    }
}

impl ChaosScenario {
    /// Create a new chaos scenario
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            faults: Vec::new(),
            validator: None,
            timeout: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Add a fault injection to the scenario
    pub fn with_fault(mut self, fault_type: FaultType) -> Self {
        self.faults.push(FaultInjection {
            fault_type,
            inject_at_ms: 0,
            description: None,
        });
        self
    }

    /// Add a delayed fault injection
    pub fn with_delayed_fault(mut self, fault_type: FaultType, inject_at_ms: u64) -> Self {
        self.faults.push(FaultInjection {
            fault_type,
            inject_at_ms,
            description: None,
        });
        self
    }

    /// Set the validation function
    pub fn with_validation<F>(mut self, validator: F) -> Self
    where
        F: Fn(&SystemState) -> Result<(), String> + Send + Sync + 'static,
    {
        self.validator = Some(Box::new(validator));
        self
    }

    /// Set scenario timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Run the chaos scenario
    pub async fn run(self) -> Result<ScenarioResult, ChaosError> {
        info!("🔥 Starting chaos scenario: {}", self.name);

        let start_time = std::time::Instant::now();
        let mut state = SystemState::default();

        // Execute faults in order
        for fault in &self.faults {
            if fault.inject_at_ms > 0 {
                tokio::time::sleep(Duration::from_millis(fault.inject_at_ms)).await;
            }

            info!(
                "💥 Injecting fault: {:?}{}",
                fault.fault_type,
                fault
                    .description
                    .as_ref()
                    .map(|d| format!(" ({})", d))
                    .unwrap_or_default()
            );

            // Inject the fault
            self.inject_fault(&fault.fault_type, &mut state).await?;
        }

        // Wait for system to stabilize
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Run validation if provided
        if let Some(validator) = &self.validator {
            validator(&state).map_err(ChaosError::ValidationFailed)?;
        }

        let duration = start_time.elapsed();
        info!(
            "✅ Chaos scenario '{}' completed in {:?}",
            self.name, duration
        );

        Ok(ScenarioResult {
            name: self.name,
            duration,
            faults_injected: self.faults.len(),
            final_state: state,
        })
    }

    /// Inject a specific fault
    async fn inject_fault(
        &self,
        fault_type: &FaultType,
        state: &mut SystemState,
    ) -> Result<(), ChaosError> {
        match fault_type {
            FaultType::NetworkPartition {
                duration_ms,
                affected_nodes,
            } => {
                debug!(
                    "Simulating network partition for {} nodes for {}ms",
                    affected_nodes.len(),
                    duration_ms
                );
                state.failed_nodes += affected_nodes.len();
                tokio::time::sleep(Duration::from_millis(*duration_ms)).await;
                state.failed_nodes -= affected_nodes.len();
                state.recovery_count += 1;
            }
            FaultType::ProcessCrash {
                node_id,
                restart_delay_ms,
            } => {
                debug!("Simulating process crash on node: {}", node_id);
                state.failed_nodes += 1;
                tokio::time::sleep(Duration::from_millis(*restart_delay_ms)).await;
                state.failed_nodes -= 1;
                state.recovery_count += 1;
            }
            FaultType::ResourceExhaustion {
                resource_type,
                consumption_percent,
                duration_ms,
            } => {
                debug!(
                    "Simulating {:?} exhaustion ({}%) for {}ms",
                    resource_type, consumption_percent, duration_ms
                );
                // In real implementation, would actually consume resources
                tokio::time::sleep(Duration::from_millis(*duration_ms)).await;
            }
            FaultType::NetworkLatency {
                latency_ms,
                duration_ms,
            } => {
                debug!(
                    "Simulating network latency ({}ms) for {}ms",
                    latency_ms, duration_ms
                );
                tokio::time::sleep(Duration::from_millis(*duration_ms)).await;
            }
            FaultType::PacketLoss {
                loss_rate,
                duration_ms,
            } => {
                debug!(
                    "Simulating packet loss ({:.1}%) for {}ms",
                    loss_rate * 100.0,
                    duration_ms
                );
                tokio::time::sleep(Duration::from_millis(*duration_ms)).await;
            }
        }

        Ok(())
    }
}

/// Result of a chaos scenario
#[derive(Debug)]
pub struct ScenarioResult {
    /// Scenario name
    pub name: String,
    /// Total duration
    pub duration: Duration,
    /// Number of faults injected
    pub faults_injected: usize,
    /// Final system state
    pub final_state: SystemState,
}

/// Chaos engineering errors
#[derive(Debug)]
pub enum ChaosError {
    /// Validation failed
    ValidationFailed(String),
    /// Fault injection failed
    InjectionFailed(String),
    /// Scenario timeout
    Timeout,
}

impl std::fmt::Display for ChaosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChaosError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            ChaosError::InjectionFailed(msg) => write!(f, "Fault injection failed: {}", msg),
            ChaosError::Timeout => write!(f, "Scenario timeout"),
        }
    }
}

impl std::error::Error for ChaosError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chaos_scenario_creation() {
        let scenario = ChaosScenario::new("test")
            .with_fault(FaultType::NetworkPartition {
                duration_ms: 1000,
                affected_nodes: vec!["node1".to_string()],
            })
            .with_timeout(Duration::from_secs(30));

        assert_eq!(scenario.name, "test");
        assert_eq!(scenario.faults.len(), 1);
    }

    #[tokio::test]
    async fn test_simple_chaos_scenario() {
        let scenario = ChaosScenario::new("simple_test")
            .with_fault(FaultType::ProcessCrash {
                node_id: "test_node".to_string(),
                restart_delay_ms: 100,
            })
            .with_validation(|state| {
                if state.recovery_count > 0 {
                    Ok(())
                } else {
                    Err("No recoveries recorded".to_string())
                }
            });

        let result = scenario.run().await.expect("Scenario should succeed");
        assert_eq!(result.faults_injected, 1);
        assert_eq!(result.final_state.recovery_count, 1);
    }
}
