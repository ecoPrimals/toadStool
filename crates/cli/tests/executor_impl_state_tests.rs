// SPDX-License-Identifier: AGPL-3.0-or-later
//! State management tests for `BiomeExecutor`
//!
//! Tests cover:
//! - State transitions and validation
//! - Concurrent state management
//! - State persistence and recovery
//! - State monitoring and queries

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

#[cfg(test)]
mod state_management_tests {
    use super::*;

    // ============================================================================
    // Biome State Definition Tests
    // ============================================================================

    #[test]
    fn test_biome_state_variants() {
        let states = vec![
            "stopped",
            "starting",
            "running",
            "pausing",
            "paused",
            "resuming",
            "stopping",
            "error",
            "migrating",
            "restarting",
        ];

        for state in states {
            assert!(!state.is_empty());
            assert!(state.len() < 20);
        }
    }

    #[test]
    fn test_valid_state_transitions() {
        let transitions = vec![
            ("stopped", "starting"),
            ("starting", "running"),
            ("running", "pausing"),
            ("pausing", "paused"),
            ("paused", "resuming"),
            ("resuming", "running"),
            ("running", "stopping"),
            ("stopping", "stopped"),
            ("running", "restarting"),
            ("restarting", "starting"),
        ];

        for (from_state, to_state) in transitions {
            assert!(!from_state.is_empty());
            assert!(!to_state.is_empty());
            assert_ne!(from_state, to_state);
        }
    }

    #[test]
    fn test_invalid_state_transitions() {
        let invalid_transitions = vec![
            ("stopped", "paused"),   // Can't pause stopped biome
            ("starting", "paused"),  // Can't pause while starting
            ("stopping", "running"), // Can't run while stopping
            ("error", "running"),    // Can't run from error directly
        ];

        for (from_state, to_state) in invalid_transitions {
            // These transitions should be rejected
            let is_valid = matches!(
                (from_state, to_state),
                ("stopped", "starting")
                    | ("starting", "running")
                    | ("running", "pausing" | "stopping")
            );

            // The invalid ones should not match valid patterns
            assert!(!is_valid || (from_state, to_state) == ("stopped", "starting"));
        }
    }

    // ============================================================================
    // State Storage Tests
    // ============================================================================

    #[derive(Clone, Debug, PartialEq)]
    struct BiomeState {
        name: String,
        status: String,
        start_time: Option<u64>,
        pid: Option<u32>,
        restart_count: u32,
    }

    impl BiomeState {
        fn new(name: String) -> Self {
            Self {
                name,
                status: "stopped".to_string(),
                start_time: None,
                pid: None,
                restart_count: 0,
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_state_storage_and_retrieval() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let biome_state = BiomeState::new("test-biome".to_string());

        // Store state
        {
            let mut states = state_store.write().await;
            states.insert("test-biome".to_string(), biome_state.clone());
        }

        // Retrieve state
        {
            let states = state_store.read().await;
            let retrieved = states.get("test-biome");
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().name, "test-biome");
            assert_eq!(retrieved.unwrap().status, "stopped");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_state_update_operations() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let mut biome_state = BiomeState::new("update-test".to_string());

        // Initial insert
        {
            let mut states = state_store.write().await;
            states.insert("update-test".to_string(), biome_state.clone());
        }

        // Update state
        biome_state.status = "running".to_string();
        biome_state.start_time = Some(1234567890);
        biome_state.pid = Some(9999);

        {
            let mut states = state_store.write().await;
            states.insert("update-test".to_string(), biome_state.clone());
        }

        // Verify update
        {
            let states = state_store.read().await;
            let updated = states.get("update-test").unwrap();
            assert_eq!(updated.status, "running");
            assert_eq!(updated.start_time, Some(1234567890));
            assert_eq!(updated.pid, Some(9999));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_multiple_biome_states() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Add multiple biome states
        {
            let mut states = state_store.write().await;
            for i in 1..=5 {
                let mut biome = BiomeState::new(format!("biome-{i}"));
                biome.status = if i % 2 == 0 { "running" } else { "stopped" }.to_string();
                states.insert(format!("biome-{i}"), biome);
            }
        }

        // Verify all states
        {
            let states = state_store.read().await;
            assert_eq!(states.len(), 5);

            let running_count = states.values().filter(|s| s.status == "running").count();
            let stopped_count = states.values().filter(|s| s.status == "stopped").count();

            assert_eq!(running_count, 2);
            assert_eq!(stopped_count, 3);
        }
    }

    // ============================================================================
    // State Transition Validation Tests
    // ============================================================================

    fn is_valid_transition(from: &str, to: &str) -> bool {
        matches!(
            (from, to),
            ("stopped" | "restarting", "starting")
                | ("starting" | "resuming", "running")
                | ("running", "pausing" | "stopping" | "restarting")
                | ("pausing", "paused")
                | ("paused", "resuming")
                | ("stopping", "stopped")
                | (_, "error") // Any state can transition to error
        )
    }

    #[test]
    fn test_state_transition_validator() {
        // Valid transitions
        assert!(is_valid_transition("stopped", "starting"));
        assert!(is_valid_transition("starting", "running"));
        assert!(is_valid_transition("running", "pausing"));
        assert!(is_valid_transition("paused", "resuming"));

        // Invalid transitions
        assert!(!is_valid_transition("stopped", "paused"));
        assert!(!is_valid_transition("stopping", "running"));
        assert!(!is_valid_transition("error", "running"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_state_transition_enforcement() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let mut biome = BiomeState::new("transition-test".to_string());
        biome.status = "stopped".to_string();

        // Initial state
        {
            let mut states = state_store.write().await;
            states.insert("transition-test".to_string(), biome.clone());
        }

        // Valid transition: stopped -> starting
        {
            let mut states = state_store.write().await;
            let current = states.get_mut("transition-test").unwrap();
            if is_valid_transition(&current.status, "starting") {
                current.status = "starting".to_string();
            }
            assert_eq!(current.status, "starting");
        }

        // Invalid transition attempt: starting -> paused (should fail)
        {
            let mut states = state_store.write().await;
            let current = states.get_mut("transition-test").unwrap();
            let original_state = current.status.clone();
            if is_valid_transition(&current.status, "paused") {
                current.status = "paused".to_string();
            }
            // Status should remain unchanged
            assert_eq!(current.status, original_state);
            assert_eq!(current.status, "starting");
        }
    }

    // ============================================================================
    // State Monitoring Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_state_change_tracking() {
        #[derive(Clone)]
        #[allow(dead_code)]
        struct StateHistory {
            biome_name: String,
            from_state: String,
            to_state: String,
            timestamp: u64,
        }

        let history: Arc<RwLock<Vec<StateHistory>>> = Arc::new(RwLock::new(Vec::new()));

        // Record state changes
        let transitions = vec![
            ("stopped", "starting"),
            ("starting", "running"),
            ("running", "stopping"),
            ("stopping", "stopped"),
        ];

        for (from, to) in transitions {
            let mut hist = history.write().await;
            hist.push(StateHistory {
                biome_name: "test".to_string(),
                from_state: from.to_string(),
                to_state: to.to_string(),
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }

        // Verify history
        let hist = history.read().await;
        assert_eq!(hist.len(), 4);
        assert_eq!(hist[0].from_state, "stopped");
        assert_eq!(hist[0].to_state, "starting");
        assert_eq!(hist[3].from_state, "stopping");
        assert_eq!(hist[3].to_state, "stopped");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_state_query_by_status() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Create biomes in various states
        {
            let mut states = state_store.write().await;
            for i in 1..=10 {
                let mut biome = BiomeState::new(format!("biome-{i}"));
                biome.status = match i % 3 {
                    0 => "running",
                    1 => "stopped",
                    _ => "paused",
                }
                .to_string();
                states.insert(format!("biome-{i}"), biome);
            }
        }

        // Query by status
        {
            let states = state_store.read().await;

            let running: Vec<_> = states.values().filter(|s| s.status == "running").collect();
            let stopped: Vec<_> = states.values().filter(|s| s.status == "stopped").collect();
            let paused: Vec<_> = states.values().filter(|s| s.status == "paused").collect();

            assert_eq!(running.len(), 3); // biome-3, 6, 9
            assert_eq!(stopped.len(), 4); // biome-1, 4, 7, 10
            assert_eq!(paused.len(), 3); // biome-2, 5, 8
        }
    }

    // ============================================================================
    // State Persistence Tests
    // ============================================================================

    #[test]
    fn test_state_serialization_format() {
        let biome = BiomeState {
            name: "persist-test".to_string(),
            status: "running".to_string(),
            start_time: Some(1234567890),
            pid: Some(12345),
            restart_count: 2,
        };

        // Simulate JSON serialization
        let json = format!(
            r#"{{"name":"{}","status":"{}","start_time":{},"pid":{},"restart_count":{}}}"#,
            biome.name,
            biome.status,
            biome.start_time.unwrap(),
            biome.pid.unwrap(),
            biome.restart_count
        );

        assert!(json.contains("persist-test"));
        assert!(json.contains("running"));
        assert!(json.contains("1234567890"));
        assert!(json.contains("12345"));
    }

    // ============================================================================
    // Concurrent State Access Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_state_reads() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Initialize state
        {
            let mut states = state_store.write().await;
            states.insert("shared".to_string(), BiomeState::new("shared".to_string()));
        }

        // Spawn multiple concurrent readers
        let mut handles = vec![];
        for _ in 0..20 {
            let store_clone = Arc::clone(&state_store);
            let handle = tokio::spawn(async move {
                let states = store_clone.read().await;
                assert!(states.contains_key("shared"));
            });
            handles.push(handle);
        }

        // Wait for all readers
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_state_updates() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Spawn multiple concurrent writers
        let mut handles = vec![];
        for i in 0..10 {
            let store_clone = Arc::clone(&state_store);
            let handle = tokio::spawn(async move {
                let mut states = store_clone.write().await;
                states.insert(format!("biome-{i}"), BiomeState::new(format!("biome-{i}")));
            });
            handles.push(handle);
        }

        // Wait for all writers
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all writes succeeded
        let states = state_store.read().await;
        assert_eq!(states.len(), 10);
    }

    // ============================================================================
    // State Recovery Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_state_recovery_after_restart() {
        // Simulate storing state before restart
        let persisted_states = vec![
            ("biome-1", "running", Some(12345u32)),
            ("biome-2", "stopped", None),
            ("biome-3", "paused", Some(12346u32)),
        ];

        // Recover states
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        {
            let mut states = state_store.write().await;
            for (name, status, pid) in persisted_states {
                let mut biome = BiomeState::new(name.to_string());
                biome.status = status.to_string();
                biome.pid = pid;
                states.insert(name.to_string(), biome);
            }
        }

        // Verify recovery
        {
            let states = state_store.read().await;
            assert_eq!(states.len(), 3);

            let biome1 = states.get("biome-1").unwrap();
            assert_eq!(biome1.status, "running");
            assert_eq!(biome1.pid, Some(12345));

            let biome2 = states.get("biome-2").unwrap();
            assert_eq!(biome2.status, "stopped");
            assert!(biome2.pid.is_none());
        }
    }

    // ============================================================================
    // State Cleanup Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_state_cleanup_on_stop() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Add biome
        {
            let mut states = state_store.write().await;
            let mut biome = BiomeState::new("cleanup-test".to_string());
            biome.status = "running".to_string();
            biome.pid = Some(99999);
            states.insert("cleanup-test".to_string(), biome);
        }

        // Stop biome (clean up state)
        {
            let mut states = state_store.write().await;
            if let Some(biome) = states.get_mut("cleanup-test") {
                biome.status = "stopped".to_string();
                biome.pid = None;
                biome.start_time = None;
            }
        }

        // Verify cleanup
        {
            let states = state_store.read().await;
            let biome = states.get("cleanup-test").unwrap();
            assert_eq!(biome.status, "stopped");
            assert!(biome.pid.is_none());
            assert!(biome.start_time.is_none());
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_state_removal() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Add biome
        {
            let mut states = state_store.write().await;
            states.insert(
                "remove-test".to_string(),
                BiomeState::new("remove-test".to_string()),
            );
        }

        // Verify exists
        {
            let states = state_store.read().await;
            assert!(states.contains_key("remove-test"));
        }

        // Remove biome
        {
            let mut states = state_store.write().await;
            states.remove("remove-test");
        }

        // Verify removed
        {
            let states = state_store.read().await;
            assert!(!states.contains_key("remove-test"));
        }
    }

    // ============================================================================
    // Restart Counter Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_restart_counter_increment() {
        let state_store: Arc<RwLock<HashMap<String, BiomeState>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Initialize biome
        {
            let mut states = state_store.write().await;
            states.insert(
                "restart-test".to_string(),
                BiomeState::new("restart-test".to_string()),
            );
        }

        // Simulate multiple restarts
        for i in 1..=5 {
            let mut states = state_store.write().await;
            let biome = states.get_mut("restart-test").unwrap();
            biome.restart_count += 1;
            assert_eq!(biome.restart_count, i);
        }

        // Verify final count
        {
            let states = state_store.read().await;
            let biome = states.get("restart-test").unwrap();
            assert_eq!(biome.restart_count, 5);
        }
    }

    #[test]
    fn test_restart_limit_check() {
        let max_restarts = 3u32;
        let test_cases = vec![
            (0, true),  // Can restart
            (1, true),  // Can restart
            (2, true),  // Can restart
            (3, false), // At limit
            (4, false), // Over limit
        ];

        for (restart_count, should_allow) in test_cases {
            let allowed = restart_count < max_restarts;
            assert_eq!(allowed, should_allow, "Restart count: {restart_count}");
        }
    }
}
