// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::unused_async)]
//! State management tests
//!
//! Tier 1 tests: Coverage-measured state management tests
//! Focus: State consistency, transitions, persistence, concurrent access

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// State Initialization Tests
// ============================================================================

#[test]
fn test_state_default_initialization() {
    let state = BiomeState::default();

    assert_eq!(state.status(), BiomeStatus::Created);
    assert_eq!(state.resource_count(), 0);
    assert!(state.metadata().is_empty());
}

#[test]
fn test_state_with_initial_config() {
    let config = StateConfig {
        name: "test-biome".to_string(),
        initial_resources: 5,
    };

    let state = BiomeState::with_config(config);

    assert_eq!(state.name(), "test-biome");
    assert_eq!(state.resource_count(), 5);
}

#[test]
fn test_state_initialization_validation() {
    let invalid_config = StateConfig {
        name: String::new(), // Invalid: empty name
        initial_resources: 0,
    };

    let result = BiomeState::try_from_config(invalid_config);

    assert!(result.is_err());
}

// ============================================================================
// State Transition Tests
// ============================================================================

#[tokio::test]
async fn test_state_transition_created_to_starting() {
    let mut state = BiomeState::default();

    assert_eq!(state.status(), BiomeStatus::Created);

    state.transition_to_starting().await.unwrap();

    assert_eq!(state.status(), BiomeStatus::Starting);
}

#[tokio::test]
async fn test_state_transition_starting_to_running() {
    let mut state = BiomeState::default();

    state.transition_to_starting().await.unwrap();
    state.transition_to_running().await.unwrap();

    assert_eq!(state.status(), BiomeStatus::Running);
}

#[tokio::test]
async fn test_state_transition_invalid() {
    let mut state = BiomeState::default();

    // Cannot go directly from Created to Running
    let result = state.transition_to_running().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_state_transition_to_failed() {
    let mut state = BiomeState::default();

    state.transition_to_starting().await.unwrap();
    state
        .transition_to_failed("startup error".to_string())
        .await
        .unwrap();

    assert_eq!(state.status(), BiomeStatus::Failed);
    assert!(state.error_message().is_some());
}

#[tokio::test]
async fn test_state_transition_complete_lifecycle() {
    let mut state = BiomeState::default();

    // Complete lifecycle
    state.transition_to_starting().await.unwrap();
    state.transition_to_running().await.unwrap();
    state.transition_to_stopping().await.unwrap();
    state.transition_to_stopped().await.unwrap();

    assert_eq!(state.status(), BiomeStatus::Stopped);
}

// ============================================================================
// State Consistency Tests
// ============================================================================

#[tokio::test]
async fn test_state_concurrent_reads() {
    let state = Arc::new(BiomeState::default());

    let mut handles = vec![];
    for _ in 0..100 {
        let s = state.clone();
        let handle = tokio::spawn(async move { s.status() });
        handles.push(handle);
    }

    // All reads should succeed
    for handle in handles {
        let status = handle.await.unwrap();
        assert_eq!(status, BiomeStatus::Created);
    }
}

#[tokio::test]
async fn test_state_concurrent_writes() {
    let state = Arc::new(RwLock::new(BiomeState::default()));

    let mut handles = vec![];
    for i in 0..10 {
        let s = state.clone();
        let handle = tokio::spawn(async move {
            let mut state = s.write().await;
            state.set_metadata(&format!("key-{i}"), i).await
        });
        handles.push(handle);
    }

    // All writes should succeed
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all metadata present
    let state = state.read().await;
    for i in 0..10 {
        assert!(state.get_metadata(&format!("key-{i}")).is_some());
    }
}

#[tokio::test]
async fn test_state_read_write_isolation() {
    let state = Arc::new(RwLock::new(BiomeState::default()));

    // Writer
    let s_write = state.clone();
    let writer = tokio::spawn(async move {
        let mut state = s_write.write().await;
        state.set_metadata("key", 100).await.unwrap();
    });

    // Reader (after write completes)
    writer.await.unwrap();

    let s_read = state.clone();
    let reader = tokio::spawn(async move {
        let state = s_read.read().await;
        state.get_metadata("key")
    });

    let value = reader.await.unwrap();
    assert_eq!(value, Some(100));
}

// ============================================================================
// State Persistence Tests
// ============================================================================

#[tokio::test]
async fn test_state_metadata_persistence() {
    let mut state = BiomeState::default();

    state.set_metadata("key1", 100).await.unwrap();
    state.set_metadata("key2", 200).await.unwrap();

    assert_eq!(state.get_metadata("key1"), Some(100));
    assert_eq!(state.get_metadata("key2"), Some(200));
}

#[tokio::test]
async fn test_state_metadata_update() {
    let mut state = BiomeState::default();

    state.set_metadata("key", 100).await.unwrap();
    assert_eq!(state.get_metadata("key"), Some(100));

    // Update
    state.set_metadata("key", 200).await.unwrap();
    assert_eq!(state.get_metadata("key"), Some(200));
}

#[tokio::test]
async fn test_state_metadata_removal() {
    let mut state = BiomeState::default();

    state.set_metadata("key", 100).await.unwrap();
    assert!(state.get_metadata("key").is_some());

    state.remove_metadata("key").await.unwrap();
    assert!(state.get_metadata("key").is_none());
}

// ============================================================================
// State Query Tests
// ============================================================================

#[tokio::test]
async fn test_state_is_terminal() {
    let mut state = BiomeState::default();

    assert!(!state.is_terminal());

    state.transition_to_starting().await.unwrap();
    assert!(!state.is_terminal());

    state.transition_to_running().await.unwrap();
    assert!(!state.is_terminal());

    state.transition_to_stopping().await.unwrap();
    state.transition_to_stopped().await.unwrap();
    assert!(state.is_terminal());
}

#[test]
fn test_state_can_transition() {
    let state = BiomeState::default();

    assert!(state.can_transition_to(BiomeStatus::Starting));
    assert!(!state.can_transition_to(BiomeStatus::Running));
    assert!(!state.can_transition_to(BiomeStatus::Stopped));
}

#[tokio::test(start_paused = true)]
async fn test_state_uptime_tracking() {
    let mut state = BiomeState::default();

    state.transition_to_starting().await.unwrap();
    state.transition_to_running().await.unwrap();

    // Advance mock time — no sleep needed; tokio::time::Instant tracks paused time.
    tokio::time::advance(tokio::time::Duration::from_millis(100)).await;

    let uptime = state.uptime().await;
    assert!(uptime.as_millis() >= 100);
}

// ============================================================================
// Mock Types (Simplified)
// ============================================================================

#[derive(Clone)]
struct BiomeState {
    status: BiomeStatus,
    metadata: HashMap<String, usize>,
    name: String,
    resource_count: usize,
    error_message: Option<String>,
    start_time: Option<tokio::time::Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BiomeStatus {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

struct StateConfig {
    name: String,
    initial_resources: usize,
}

impl Default for BiomeState {
    fn default() -> Self {
        Self {
            status: BiomeStatus::Created,
            metadata: HashMap::new(),
            name: String::new(),
            resource_count: 0,
            error_message: None,
            start_time: None,
        }
    }
}

impl BiomeState {
    fn with_config(config: StateConfig) -> Self {
        Self {
            name: config.name,
            resource_count: config.initial_resources,
            ..Default::default()
        }
    }

    fn try_from_config(config: StateConfig) -> Result<Self, String> {
        if config.name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        Ok(Self::with_config(config))
    }

    fn status(&self) -> BiomeStatus {
        self.status
    }

    fn resource_count(&self) -> usize {
        self.resource_count
    }

    fn metadata(&self) -> &HashMap<String, usize> {
        &self.metadata
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    async fn transition_to_starting(&mut self) -> Result<(), String> {
        if self.status == BiomeStatus::Created {
            self.status = BiomeStatus::Starting;
            self.start_time = Some(tokio::time::Instant::now());
            Ok(())
        } else {
            Err("Invalid transition".to_string())
        }
    }

    async fn transition_to_running(&mut self) -> Result<(), String> {
        if self.status == BiomeStatus::Starting {
            self.status = BiomeStatus::Running;
            Ok(())
        } else {
            Err("Invalid transition".to_string())
        }
    }

    async fn transition_to_stopping(&mut self) -> Result<(), String> {
        if self.status == BiomeStatus::Running {
            self.status = BiomeStatus::Stopping;
            Ok(())
        } else {
            Err("Invalid transition".to_string())
        }
    }

    async fn transition_to_stopped(&mut self) -> Result<(), String> {
        if self.status == BiomeStatus::Stopping {
            self.status = BiomeStatus::Stopped;
            Ok(())
        } else {
            Err("Invalid transition".to_string())
        }
    }

    async fn transition_to_failed(&mut self, error: String) -> Result<(), String> {
        self.status = BiomeStatus::Failed;
        self.error_message = Some(error);
        Ok(())
    }

    async fn set_metadata(&mut self, key: &str, value: usize) -> Result<(), String> {
        self.metadata.insert(key.to_string(), value);
        Ok(())
    }

    fn get_metadata(&self, key: &str) -> Option<usize> {
        self.metadata.get(key).copied()
    }

    async fn remove_metadata(&mut self, key: &str) -> Result<(), String> {
        self.metadata.remove(key);
        Ok(())
    }

    fn is_terminal(&self) -> bool {
        matches!(self.status, BiomeStatus::Stopped | BiomeStatus::Failed)
    }

    fn can_transition_to(&self, target: BiomeStatus) -> bool {
        matches!(
            (self.status, target),
            (BiomeStatus::Created, BiomeStatus::Starting)
                | (BiomeStatus::Starting, BiomeStatus::Running)
                | (BiomeStatus::Running, BiomeStatus::Stopping)
                | (BiomeStatus::Stopping, BiomeStatus::Stopped)
        )
    }

    async fn uptime(&self) -> tokio::time::Duration {
        if let Some(start) = self.start_time {
            start.elapsed()
        } else {
            tokio::time::Duration::from_secs(0)
        }
    }
}
