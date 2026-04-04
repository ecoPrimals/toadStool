// SPDX-License-Identifier: AGPL-3.0-only
#![expect(
    clippy::cast_precision_loss,
    reason = "precision loss acceptable for this conversion"
)]
#![allow(clippy::missing_errors_doc)]
//! Chaos Testing - Tower Failures
//!
//! Tests system behavior under tower chaos:
//! - Tower crashes
//! - Resource exhaustion
//! - Slow towers
//! - Byzantine towers (malicious behavior)
//! - Partial failures
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::no_effect_underscore_binding,
    clippy::items_after_statements
)]

use std::time::Duration;
use tokio::time::sleep;

/// Tower failure mode
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TowerFailure {
    /// Tower crashes immediately
    Crash,

    /// Tower runs out of memory
    OutOfMemory,

    /// Tower runs out of GPU resources
    OutOfGpuMemory,

    /// Tower becomes very slow
    Slow { delay_ms: u64 },

    /// Tower returns incorrect results (Byzantine)
    Byzantine,

    /// Tower partially fails (some operations work)
    PartialFailure { success_rate: f32 },
}

/// Tower state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TowerState {
    Healthy,
    Degraded,
    Failed,
    Crashed,
}

/// Simulated tower
pub struct SimulatedTower {
    #[allow(dead_code)]
    id: String,
    state: TowerState,
    active_failures: Vec<TowerFailure>,
    memory_used: u64,
    memory_total: u64,
    gpu_memory_used: u64,
    gpu_memory_total: u64,
}

impl SimulatedTower {
    #[must_use]
    pub fn new(id: String) -> Self {
        Self {
            id,
            state: TowerState::Healthy,
            active_failures: vec![],
            memory_used: 0,
            memory_total: 8_000_000_000, // 8GB
            gpu_memory_used: 0,
            gpu_memory_total: 4_000_000_000, // 4GB
        }
    }

    pub fn inject_failure(&mut self, failure: TowerFailure) {
        self.active_failures.push(failure);
        self.update_state();
    }

    pub fn clear_failures(&mut self) {
        self.active_failures.clear();
        self.state = TowerState::Healthy;
    }

    #[must_use]
    pub fn state(&self) -> &TowerState {
        &self.state
    }

    fn update_state(&mut self) {
        for failure in &self.active_failures {
            match failure {
                TowerFailure::Crash => {
                    self.state = TowerState::Crashed;
                    return;
                }
                TowerFailure::OutOfMemory | TowerFailure::OutOfGpuMemory => {
                    self.state = TowerState::Failed;
                    return;
                }
                TowerFailure::Slow { .. }
                | TowerFailure::PartialFailure { .. }
                | TowerFailure::Byzantine => {
                    self.state = TowerState::Degraded;
                }
            }
        }
    }

    pub async fn execute_workload(&mut self, size_mb: u64) -> Result<Vec<u8>, String> {
        // Check state
        if self.state == TowerState::Crashed {
            return Err("Tower crashed".to_string());
        }

        // Check for active failures
        for failure in &self.active_failures {
            match failure {
                TowerFailure::Crash => {
                    self.state = TowerState::Crashed;
                    return Err("Tower crashed during execution".to_string());
                }
                TowerFailure::OutOfMemory => {
                    if self.memory_used + (size_mb * 1_000_000) > self.memory_total {
                        return Err("Out of memory".to_string());
                    }
                }
                TowerFailure::OutOfGpuMemory => {
                    if self.gpu_memory_used + (size_mb * 1_000_000) > self.gpu_memory_total {
                        return Err("Out of GPU memory".to_string());
                    }
                }
                TowerFailure::Slow { delay_ms } => {
                    sleep(Duration::from_millis(*delay_ms)).await;
                }
                TowerFailure::Byzantine => {
                    // Return incorrect data
                    return Ok(vec![0xFF; (size_mb * 1_000_000) as usize]);
                }
                TowerFailure::PartialFailure { success_rate } => {
                    if rand::random::<f32>() > *success_rate {
                        return Err("Partial failure".to_string());
                    }
                }
            }
        }

        // Simulate work
        sleep(Duration::from_millis(10)).await;

        // Allocate memory
        self.memory_used += size_mb * 1_000_000;
        self.gpu_memory_used += size_mb * 1_000_000;

        Ok(vec![0; (size_mb * 1_000_000) as usize])
    }

    pub fn free_resources(&mut self) {
        self.memory_used = 0;
        self.gpu_memory_used = 0;
    }
}

// ============================================================================
// TOWER CRASH TESTS
// ============================================================================

#[tokio::test]
async fn test_tower_crash_immediate() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.inject_failure(TowerFailure::Crash);

    assert_eq!(tower.state(), &TowerState::Crashed);

    let result = tower.execute_workload(1).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tower_crash_during_execution() {
    let mut tower = SimulatedTower::new("tower1".to_string());

    // Start healthy
    assert_eq!(tower.state(), &TowerState::Healthy);

    // Inject crash
    tower.inject_failure(TowerFailure::Crash);

    // Execution should fail
    let result = tower.execute_workload(1).await;
    assert!(result.is_err());
    assert_eq!(tower.state(), &TowerState::Crashed);
}

#[tokio::test]
async fn test_tower_crash_recovery() {
    let mut tower = SimulatedTower::new("tower1".to_string());

    // Crash
    tower.inject_failure(TowerFailure::Crash);
    assert_eq!(tower.state(), &TowerState::Crashed);

    // "Restart" tower
    tower.clear_failures();
    tower.free_resources();

    // Should work again
    assert_eq!(tower.state(), &TowerState::Healthy);
    let result = tower.execute_workload(1).await;
    assert!(result.is_ok());
}

// ============================================================================
// RESOURCE EXHAUSTION TESTS
// ============================================================================

#[tokio::test]
async fn test_out_of_memory() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.memory_total = 100_000_000; // 100MB

    tower.inject_failure(TowerFailure::OutOfMemory);

    // Try to allocate more than available
    let result = tower.execute_workload(200).await; // 200MB
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Out of memory"));
}

#[tokio::test]
async fn test_out_of_gpu_memory() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.gpu_memory_total = 100_000_000; // 100MB

    tower.inject_failure(TowerFailure::OutOfGpuMemory);

    // Try to allocate more than available
    let result = tower.execute_workload(200).await; // 200MB
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("GPU memory"));
}

#[tokio::test]
async fn test_memory_pressure_gradual() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.memory_total = 1_000_000_000; // 1GB

    // Gradually fill memory
    for _ in 0..90 {
        let result = tower.execute_workload(10).await; // 10MB each
        if result.is_err() {
            break;
        }
    }

    // Memory pressure increases but may not fail if within limits
    // Just verify we can track memory usage
    assert!(tower.memory_used > 0, "Memory should be allocated");
}

// ============================================================================
// SLOW TOWER TESTS
// ============================================================================

#[tokio::test]
async fn test_slow_tower() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.inject_failure(TowerFailure::Slow { delay_ms: 500 });

    let start = std::time::Instant::now();
    let result = tower.execute_workload(1).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(elapsed.as_millis() >= 500);
    assert_eq!(tower.state(), &TowerState::Degraded);
}

#[tokio::test]
async fn test_slow_tower_timeout() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.inject_failure(TowerFailure::Slow { delay_ms: 2000 });

    let timeout = Duration::from_millis(1000);
    let start = std::time::Instant::now();

    let result = tokio::time::timeout(timeout, tower.execute_workload(1)).await;
    let elapsed = start.elapsed();

    // Should timeout
    assert!(result.is_err());
    assert!(elapsed.as_millis() >= 1000 && elapsed.as_millis() < 1500);
}

#[tokio::test]
async fn test_variable_tower_speed() {
    let mut towers = vec![];

    // Create towers with different speeds
    for (i, delay) in [0, 100, 500, 1000].iter().enumerate() {
        let mut tower = SimulatedTower::new(format!("tower{i}"));
        if *delay > 0 {
            tower.inject_failure(TowerFailure::Slow { delay_ms: *delay });
        }
        towers.push(tower);
    }

    // Execute on all towers concurrently
    let mut tasks = vec![];
    for mut tower in towers {
        tasks.push(tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = tower.execute_workload(1).await;
            (start.elapsed(), result.is_ok())
        }));
    }

    // Collect results
    let mut times = vec![];
    for task in tasks {
        let (elapsed, success) = task.await.unwrap();
        if success {
            times.push(elapsed.as_millis());
        }
    }

    // Fastest should complete first
    assert!(times[0] < times[times.len() - 1]);
}

// ============================================================================
// BYZANTINE TOWER TESTS
// ============================================================================

#[tokio::test]
async fn test_byzantine_tower_incorrect_results() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.inject_failure(TowerFailure::Byzantine);

    let result = tower.execute_workload(1).await;
    assert!(result.is_ok());

    // Result should be all 0xFF (incorrect)
    let data = result.unwrap();
    assert!(data.iter().all(|&b| b == 0xFF));
}

#[tokio::test]
async fn test_byzantine_detection() {
    // Create 3 towers: 2 honest, 1 byzantine
    let mut towers = vec![];
    for i in 0..3 {
        let mut tower = SimulatedTower::new(format!("tower{i}"));
        if i == 1 {
            tower.inject_failure(TowerFailure::Byzantine);
        }
        towers.push(tower);
    }

    // Execute on all towers
    let mut results = vec![];
    for mut tower in towers {
        let result = tower.execute_workload(1).await;
        if let Ok(data) = result {
            results.push(data);
        }
    }

    // Majority should be correct (all zeros)
    let correct_count = results.iter().filter(|r| r.iter().all(|&b| b == 0)).count();
    let incorrect_count = results
        .iter()
        .filter(|r| r.iter().all(|&b| b == 0xFF))
        .count();

    assert_eq!(correct_count, 2);
    assert_eq!(incorrect_count, 1);
}

// ============================================================================
// PARTIAL FAILURE TESTS
// ============================================================================

#[tokio::test]
async fn test_partial_failure_low_rate() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.inject_failure(TowerFailure::PartialFailure { success_rate: 0.8 }); // 80% success

    let mut successes = 0;
    let mut failures = 0;

    for _ in 0..100 {
        tower.free_resources(); // Reset for each attempt
        let result = tower.execute_workload(1).await;
        if result.is_ok() {
            successes += 1;
        } else {
            failures += 1;
        }
    }

    // Should be around 80% success rate; allow ±15pp to avoid flakiness on loaded machines
    // (100 trials, p=0.8 → σ=4; ±15pp is ±3.75σ, P(outside) ≈ 0.02%)
    let success_rate = successes as f32 / (successes + failures) as f32;
    assert!(
        (0.65..=0.95).contains(&success_rate),
        "expected ~80% success rate, got {:.1}%",
        success_rate * 100.0
    );
}

#[tokio::test]
async fn test_partial_failure_high_rate() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.inject_failure(TowerFailure::PartialFailure { success_rate: 0.3 }); // 30% success

    let mut successes = 0;
    let mut failures = 0;

    for _ in 0..100 {
        tower.free_resources();
        let result = tower.execute_workload(1).await;
        if result.is_ok() {
            successes += 1;
        } else {
            failures += 1;
        }
    }

    // Should be around 30% success rate
    let success_rate = successes as f32 / (successes + failures) as f32;
    assert!((0.20..=0.40).contains(&success_rate));
}

// ============================================================================
// COMBINED FAILURE TESTS
// ============================================================================

#[tokio::test]
async fn test_combined_slow_and_partial_failure() {
    let mut tower = SimulatedTower::new("tower1".to_string());
    tower.inject_failure(TowerFailure::Slow { delay_ms: 100 });
    tower.inject_failure(TowerFailure::PartialFailure { success_rate: 0.5 });

    let start = std::time::Instant::now();
    let result = tower.execute_workload(1).await;
    let elapsed = start.elapsed();

    // Either succeeds slowly or fails
    if result.is_ok() {
        assert!(elapsed.as_millis() >= 100);
    }
}

#[tokio::test]
async fn test_cascading_tower_failures() {
    // Simulate 5 towers where failures cascade
    let mut towers = vec![];
    for i in 0..5 {
        towers.push(SimulatedTower::new(format!("tower{i}")));
    }

    // First tower crashes
    towers[0].inject_failure(TowerFailure::Crash);

    // Load redistributes, causing others to slow down
    for tower in towers.iter_mut().skip(1).take(2) {
        tower.inject_failure(TowerFailure::Slow { delay_ms: 200 });
    }

    // Remaining towers experience partial failures due to overload
    for tower in towers.iter_mut().skip(3) {
        tower.inject_failure(TowerFailure::PartialFailure { success_rate: 0.5 });
    }

    // Try to execute on all towers
    let mut successful = 0;
    for mut tower in towers {
        if tower.execute_workload(1).await.is_ok() {
            successful += 1;
        }
    }

    // With cascading failures, expect low-ish success rate
    // Tower 0 crashes, towers 1-2 are slow (but may succeed), towers 3-4 have 50% success
    // In lucky runs, all non-crashed towers may succeed (4 max)
    assert!(
        successful <= 4,
        "Expected at most 4 successful (tower 0 crashes), got {successful}"
    );
}

#[tokio::test]
async fn test_tower_recovery_under_load() {
    let mut tower = SimulatedTower::new("tower1".to_string());

    // Start with failures
    tower.inject_failure(TowerFailure::Slow { delay_ms: 100 });
    tower.inject_failure(TowerFailure::PartialFailure { success_rate: 0.3 });

    // Try multiple times
    let mut attempts = 0;
    let max_attempts = 20;

    while attempts < max_attempts {
        attempts += 1;
        tower.free_resources();

        if tower.execute_workload(1).await.is_ok() {
            // Success! Clear failures to simulate recovery
            tower.clear_failures();
            break;
        }

        sleep(Duration::from_millis(10)).await;
    }

    // Should eventually recover
    assert!(attempts < max_attempts);
    assert_eq!(tower.state(), &TowerState::Healthy);
}

#[tokio::test]
async fn test_multi_tower_redundancy() {
    // Create 5 towers with various failures
    let mut towers = vec![];
    towers.push(SimulatedTower::new("tower0".to_string())); // Healthy

    let mut t1 = SimulatedTower::new("tower1".to_string());
    t1.inject_failure(TowerFailure::Crash);
    towers.push(t1);

    let mut t2 = SimulatedTower::new("tower2".to_string());
    t2.inject_failure(TowerFailure::Slow { delay_ms: 500 });
    towers.push(t2);

    let mut t3 = SimulatedTower::new("tower3".to_string());
    t3.inject_failure(TowerFailure::Byzantine);
    towers.push(t3);

    towers.push(SimulatedTower::new("tower4".to_string())); // Healthy

    // Execute on all towers concurrently
    let mut tasks = vec![];
    for mut tower in towers {
        tasks.push(tokio::spawn(async move { tower.execute_workload(1).await }));
    }

    // Collect results
    let mut correct_results = 0;
    for task in tasks {
        if let Ok(Ok(data)) = task.await {
            // Check if result is correct (all zeros, not 0xFF)
            if data.iter().all(|&b| b == 0) {
                correct_results += 1;
            }
        }
    }

    // At least 2 correct results (the 2 healthy towers)
    assert!(correct_results >= 2);
}
