// SPDX-License-Identifier: AGPL-3.0-or-later
//! Chaos Testing - Network Failures
//!
//! Tests system behavior under network chaos:
//! - Connection drops
//! - Timeouts
//! - Packet loss
//! - Network partitions
//! - Latency spikes

use std::time::Duration;
use tokio::time::sleep;

/// Simulated network failure
#[derive(Debug, Clone)]
pub enum NetworkFailure {
    /// Connection drops immediately
    ConnectionDrop,

    /// Request times out
    Timeout { after_ms: u64 },

    /// Random packet loss
    PacketLoss { rate: f32 },

    /// Network partition (split brain)
    Partition { duration_ms: u64 },

    /// High latency
    Latency { delay_ms: u64 },
}

/// Network chaos simulator
pub struct NetworkChaos {
    #[allow(dead_code)]
    failure_rate: f32,
    active_failures: Vec<NetworkFailure>,
}

impl NetworkChaos {
    pub fn new(failure_rate: f32) -> Self {
        Self {
            failure_rate,
            active_failures: vec![],
        }
    }

    pub fn inject_failure(&mut self, failure: NetworkFailure) {
        self.active_failures.push(failure);
    }

    pub async fn simulate_request(&self) -> Result<(), NetworkFailure> {
        // Simulate network delay
        sleep(Duration::from_millis(10)).await;

        // Check for active failures
        for failure in &self.active_failures {
            match failure {
                NetworkFailure::ConnectionDrop => {
                    return Err(NetworkFailure::ConnectionDrop);
                }
                NetworkFailure::Timeout { after_ms } => {
                    sleep(Duration::from_millis(*after_ms)).await;
                    return Err(NetworkFailure::Timeout {
                        after_ms: *after_ms,
                    });
                }
                NetworkFailure::PacketLoss { rate } => {
                    if rand::random::<f32>() < *rate {
                        return Err(NetworkFailure::PacketLoss { rate: *rate });
                    }
                }
                NetworkFailure::Latency { delay_ms } => {
                    sleep(Duration::from_millis(*delay_ms)).await;
                }
                NetworkFailure::Partition { .. } => {
                    return Err(NetworkFailure::Partition { duration_ms: 1000 });
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// CONNECTION DROP TESTS
// ============================================================================

#[tokio::test]
async fn test_connection_drop_single() {
    let mut chaos = NetworkChaos::new(0.0);
    chaos.inject_failure(NetworkFailure::ConnectionDrop);

    let result = chaos.simulate_request().await;
    assert!(result.is_err());

    if let Err(NetworkFailure::ConnectionDrop) = result {
        // Expected
    } else {
        panic!("Expected ConnectionDrop");
    }
}

#[tokio::test]
async fn test_connection_drop_recovery() {
    let mut chaos = NetworkChaos::new(0.0);

    // First request fails
    chaos.inject_failure(NetworkFailure::ConnectionDrop);
    let result1 = chaos.simulate_request().await;
    assert!(result1.is_err());

    // Clear failures and retry
    chaos.active_failures.clear();
    let result2 = chaos.simulate_request().await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_connection_drop_multiple_towers() {
    // Simulate 3 towers, 1 drops
    let mut towers = vec![];
    for i in 0..3 {
        let mut chaos = NetworkChaos::new(0.0);
        if i == 1 {
            chaos.inject_failure(NetworkFailure::ConnectionDrop);
        }
        towers.push(chaos);
    }

    // Try all towers
    let mut successful = 0;
    for tower in &towers {
        if tower.simulate_request().await.is_ok() {
            successful += 1;
        }
    }

    assert_eq!(successful, 2); // 2 out of 3 should succeed
}

// ============================================================================
// TIMEOUT TESTS
// ============================================================================

#[tokio::test]
async fn test_timeout_short() {
    let mut chaos = NetworkChaos::new(0.0);
    chaos.inject_failure(NetworkFailure::Timeout { after_ms: 50 });

    let start = std::time::Instant::now();
    let result = chaos.simulate_request().await;
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(elapsed.as_millis() >= 50);
}

#[tokio::test]
async fn test_timeout_with_retry() {
    let mut chaos = NetworkChaos::new(0.0);
    chaos.inject_failure(NetworkFailure::Timeout { after_ms: 100 });

    // First attempt times out
    let result1 = chaos.simulate_request().await;
    assert!(result1.is_err());

    // Retry without timeout
    chaos.active_failures.clear();
    let result2 = chaos.simulate_request().await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_timeout_cascade() {
    // Multiple timeouts in sequence
    let mut chaos = NetworkChaos::new(0.0);

    for _ in 0..3 {
        chaos.inject_failure(NetworkFailure::Timeout { after_ms: 50 });
        let result = chaos.simulate_request().await;
        assert!(result.is_err());
        chaos.active_failures.clear();
    }
}

// ============================================================================
// PACKET LOSS TESTS
// ============================================================================

#[tokio::test]
async fn test_packet_loss_low_rate() {
    let mut chaos = NetworkChaos::new(0.0);
    chaos.inject_failure(NetworkFailure::PacketLoss { rate: 0.1 }); // 10% loss

    let mut attempts = 0;
    let mut failures = 0;

    for _ in 0..100 {
        attempts += 1;
        if chaos.simulate_request().await.is_err() {
            failures += 1;
        }
    }

    // Should be around 10% failure rate (with statistical variance)
    // With 100 trials at 10% rate, binomial 99% CI ≈ [2%, 22%]
    let failure_rate = failures as f32 / attempts as f32;
    assert!(
        (0.02..=0.22).contains(&failure_rate),
        "Expected ~10% failure rate, got {:.0}%",
        failure_rate * 100.0
    );
}

#[tokio::test]
async fn test_packet_loss_high_rate() {
    let mut chaos = NetworkChaos::new(0.0);
    chaos.inject_failure(NetworkFailure::PacketLoss { rate: 0.5 }); // 50% loss

    let mut attempts = 0;
    let mut failures = 0;

    for _ in 0..100 {
        attempts += 1;
        if chaos.simulate_request().await.is_err() {
            failures += 1;
        }
    }

    // Should be around 50% failure rate (with statistical variance)
    // With 100 trials at 50% rate, binomial 99% CI ≈ [35%, 65%]
    let failure_rate = failures as f32 / attempts as f32;
    assert!(
        (0.35..=0.65).contains(&failure_rate),
        "Expected ~50% failure rate, got {:.0}%",
        failure_rate * 100.0
    );
}

// ============================================================================
// NETWORK PARTITION TESTS
// ============================================================================

#[tokio::test]
async fn test_network_partition() {
    let mut chaos = NetworkChaos::new(0.0);
    chaos.inject_failure(NetworkFailure::Partition { duration_ms: 1000 });

    let result = chaos.simulate_request().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_partition_recovery() {
    let mut chaos = NetworkChaos::new(0.0);

    // Create partition
    chaos.inject_failure(NetworkFailure::Partition { duration_ms: 100 });
    let result1 = chaos.simulate_request().await;
    assert!(result1.is_err());

    // Wait for partition to heal
    sleep(Duration::from_millis(150)).await;
    chaos.active_failures.clear();

    let result2 = chaos.simulate_request().await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_split_brain_scenario() {
    // Simulate split brain: two groups that can't communicate
    let group_a = vec![NetworkChaos::new(0.0), NetworkChaos::new(0.0)];
    let group_b = vec![NetworkChaos::new(0.0), NetworkChaos::new(0.0)];

    // Inject partition between groups
    let group_a_mut: Vec<_> = group_a
        .into_iter()
        .map(|mut c| {
            c.inject_failure(NetworkFailure::Partition { duration_ms: 1000 });
            c
        })
        .collect();

    // Group A can communicate internally
    for _chaos in &group_a_mut {
        // Simulate internal communication (no partition)
        let internal = NetworkChaos::new(0.0);
        assert!(internal.simulate_request().await.is_ok());
    }

    // Group B can communicate internally
    for _chaos in &group_b {
        let internal = NetworkChaos::new(0.0);
        assert!(internal.simulate_request().await.is_ok());
    }

    // Cross-group communication fails
    for chaos in &group_a_mut {
        assert!(chaos.simulate_request().await.is_err());
    }
}

// ============================================================================
// LATENCY TESTS
// ============================================================================

#[tokio::test]
async fn test_high_latency() {
    let mut chaos = NetworkChaos::new(0.0);
    chaos.inject_failure(NetworkFailure::Latency { delay_ms: 200 });

    let start = std::time::Instant::now();
    let result = chaos.simulate_request().await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(elapsed.as_millis() >= 200);
}

#[tokio::test]
async fn test_variable_latency() {
    let mut chaos = NetworkChaos::new(0.0);

    let latencies = vec![50, 100, 200, 500];
    let mut measured = vec![];

    for latency in latencies {
        chaos.active_failures.clear();
        chaos.inject_failure(NetworkFailure::Latency { delay_ms: latency });

        let start = std::time::Instant::now();
        let _ = chaos.simulate_request().await;
        let elapsed = start.elapsed().as_millis();

        measured.push(elapsed);
    }

    // Verify increasing latency
    for i in 1..measured.len() {
        assert!(measured[i] > measured[i - 1]);
    }
}

// ============================================================================
// COMBINED FAILURE TESTS
// ============================================================================

#[tokio::test]
async fn test_combined_packet_loss_and_latency() {
    let mut chaos = NetworkChaos::new(0.0);
    chaos.inject_failure(NetworkFailure::PacketLoss { rate: 0.3 });
    chaos.inject_failure(NetworkFailure::Latency { delay_ms: 100 });

    let start = std::time::Instant::now();
    let result = chaos.simulate_request().await;
    let elapsed = start.elapsed();

    // Either succeeds with latency or fails due to packet loss
    if result.is_ok() {
        assert!(elapsed.as_millis() >= 100);
    }
}

#[tokio::test]
async fn test_cascading_failures() {
    // Simulate cascading failures across multiple towers
    let mut towers = vec![];
    for _ in 0..5 {
        towers.push(NetworkChaos::new(0.0));
    }

    // First tower fails, causing load on others
    towers[0].inject_failure(NetworkFailure::ConnectionDrop);

    // Increased load causes timeouts on other towers
    for tower in towers.iter_mut().skip(1) {
        tower.inject_failure(NetworkFailure::Timeout { after_ms: 100 });
    }

    // Try all towers
    let mut successful = 0;
    for tower in &towers {
        if tower.simulate_request().await.is_ok() {
            successful += 1;
        }
    }

    // Most should fail under cascading load
    assert!(successful < 2);
}

#[tokio::test]
async fn test_recovery_under_chaos() {
    let mut chaos = NetworkChaos::new(0.0);

    // Inject multiple failures
    chaos.inject_failure(NetworkFailure::PacketLoss { rate: 0.5 });
    chaos.inject_failure(NetworkFailure::Latency { delay_ms: 50 });

    // Try multiple times with retry logic
    let mut attempts = 0;
    let max_attempts = 10;

    while attempts < max_attempts {
        attempts += 1;
        if chaos.simulate_request().await.is_ok() {
            break;
        }
        sleep(Duration::from_millis(10)).await;
    }

    // Should eventually succeed with retries
    assert!(attempts < max_attempts);
}
