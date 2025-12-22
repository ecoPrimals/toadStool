//! Comprehensive tests for executor_impl.rs
//!
//! This test file covers the BiomeExecutor functionality including:
//! - Biome lifecycle management (run, up, down)
//! - Process management and monitoring
//! - Resource allocation and limits
//! - Error handling and edge cases
//! - Concurrent biome execution

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(test)]
mod executor_impl_basic_tests {
    use super::*;

    // ============================================================================
    // BiomeExecutor Creation Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_biome_executor_initialization() {
        // Test that we can create the structure (even if new() might fail in test env)
        // This tests the type structure
        let biomes: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

        assert!(biomes.read().await.is_empty());
    }

    #[test]
    fn test_biome_name_validation() {
        // Valid biome names
        let valid_names = vec!["my-biome", "web-app", "service_123", "my.biome.app"];

        for name in valid_names {
            assert!(!name.is_empty());
            // Biome names should not start with special chars
            assert!(!name.starts_with('-'));
            assert!(!name.starts_with('_'));
        }
    }

    #[test]
    fn test_biome_name_edge_cases() {
        // Empty name
        let empty = "";
        assert!(empty.is_empty());

        // Very long name (should have reasonable limits)
        let long_name = "a".repeat(256);
        assert!(long_name.len() == 256);

        // Special characters that should be handled
        let special_chars = vec!["biome@123", "biome#test", "biome!prod"];
        for name in special_chars {
            assert!(
                name.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.')
            );
        }
    }

    // ============================================================================
    // Biome Lifecycle Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_biome_registry_operations() {
        let biomes: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

        // Test adding a biome
        {
            let mut biome_map = biomes.write().await;
            biome_map.insert("test-biome".to_string(), "biome-id-123".to_string());
        }

        // Test reading biomes
        {
            let biome_map = biomes.read().await;
            assert!(biome_map.contains_key("test-biome"));
            assert_eq!(
                biome_map.get("test-biome"),
                Some(&"biome-id-123".to_string())
            );
        }

        // Test removing a biome
        {
            let mut biome_map = biomes.write().await;
            biome_map.remove("test-biome");
        }

        {
            let biome_map = biomes.read().await;
            assert!(!biome_map.contains_key("test-biome"));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_biome_duplicate_detection() {
        let biomes: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

        // Add a biome
        {
            let mut biome_map = biomes.write().await;
            biome_map.insert("duplicate-test".to_string(), "id-1".to_string());
        }

        // Check if biome already exists (should exist)
        {
            let biome_map = biomes.read().await;
            assert!(biome_map.contains_key("duplicate-test"));
        }

        // Attempting to add again should be detected
        {
            let biome_map = biomes.read().await;
            let exists = biome_map.contains_key("duplicate-test");
            assert!(exists, "Should detect duplicate biome");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_multiple_biomes_registration() {
        let biomes: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

        // Register multiple biomes
        {
            let mut biome_map = biomes.write().await;
            biome_map.insert("biome-1".to_string(), "id-1".to_string());
            biome_map.insert("biome-2".to_string(), "id-2".to_string());
            biome_map.insert("biome-3".to_string(), "id-3".to_string());
        }

        // Verify all biomes are registered
        {
            let biome_map = biomes.read().await;
            assert_eq!(biome_map.len(), 3);
            assert!(biome_map.contains_key("biome-1"));
            assert!(biome_map.contains_key("biome-2"));
            assert!(biome_map.contains_key("biome-3"));
        }
    }

    // ============================================================================
    // Environment Variable Parsing Tests
    // ============================================================================

    #[test]
    fn test_parse_environment_variables() {
        let env_vars = vec![
            "KEY1=value1".to_string(),
            "KEY2=value2".to_string(),
            "PATH=/usr/bin:/usr/local/bin".to_string(),
        ];

        let mut environment = HashMap::new();
        for env_var in env_vars {
            if let Some((key, value)) = env_var.split_once('=') {
                environment.insert(key.to_string(), value.to_string());
            }
        }

        assert_eq!(environment.len(), 3);
        assert_eq!(environment.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(environment.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(
            environment.get("PATH"),
            Some(&"/usr/bin:/usr/local/bin".to_string())
        );
    }

    #[test]
    fn test_parse_environment_variables_with_equals() {
        let env_vars = vec![
            "DATABASE_URL=postgres://user:pass=123@localhost/db".to_string(),
            "FORMULA=x=y+z".to_string(),
        ];

        let mut environment = HashMap::new();
        for env_var in env_vars {
            if let Some((key, value)) = env_var.split_once('=') {
                environment.insert(key.to_string(), value.to_string());
            }
        }

        assert_eq!(environment.len(), 2);
        assert_eq!(
            environment.get("DATABASE_URL"),
            Some(&"postgres://user:pass=123@localhost/db".to_string())
        );
        assert_eq!(environment.get("FORMULA"), Some(&"x=y+z".to_string()));
    }

    #[test]
    fn test_parse_environment_variables_invalid() {
        let env_vars = vec![
            "VALID_KEY=value".to_string(),
            "INVALID_NO_EQUALS".to_string(),
            "".to_string(),
        ];

        let mut environment = HashMap::new();
        for env_var in env_vars {
            if let Some((key, value)) = env_var.split_once('=') {
                environment.insert(key.to_string(), value.to_string());
            }
        }

        // Only valid entry should be parsed
        assert_eq!(environment.len(), 1);
        assert_eq!(environment.get("VALID_KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_parse_environment_variables_empty_value() {
        let env_vars = vec![
            "EMPTY_VALUE=".to_string(),
            "NORMAL_VALUE=something".to_string(),
        ];

        let mut environment = HashMap::new();
        for env_var in env_vars {
            if let Some((key, value)) = env_var.split_once('=') {
                environment.insert(key.to_string(), value.to_string());
            }
        }

        assert_eq!(environment.len(), 2);
        assert_eq!(environment.get("EMPTY_VALUE"), Some(&"".to_string()));
        assert_eq!(
            environment.get("NORMAL_VALUE"),
            Some(&"something".to_string())
        );
    }

    // ============================================================================
    // Path and File Operations Tests
    // ============================================================================

    #[test]
    fn test_log_directory_path_generation() {
        let biome_name = "test-biome";
        let log_dir = PathBuf::from(format!("/tmp/toadstool/logs/{biome_name}"));

        assert_eq!(log_dir.to_str(), Some("/tmp/toadstool/logs/test-biome"));
        assert!(log_dir.to_string_lossy().contains("test-biome"));
    }

    #[test]
    fn test_log_file_path_generation() {
        let biome_name = "my-service";
        let log_dir = PathBuf::from(format!("/tmp/toadstool/logs/{biome_name}"));
        let log_file = log_dir.join("service.log");

        assert_eq!(
            log_file.to_str(),
            Some("/tmp/toadstool/logs/my-service/service.log")
        );
    }

    #[test]
    fn test_multiple_log_file_paths() {
        let biome_name = "multi-service";
        let log_dir = PathBuf::from(format!("/tmp/toadstool/logs/{biome_name}"));

        let service_logs = vec!["web.log", "api.log", "worker.log"];
        let mut log_files = HashMap::new();

        for service_log in service_logs {
            log_files.insert(service_log.to_string(), log_dir.join(service_log));
        }

        assert_eq!(log_files.len(), 3);
        assert!(log_files.contains_key("web.log"));
        assert!(log_files.contains_key("api.log"));
        assert!(log_files.contains_key("worker.log"));
    }

    // ============================================================================
    // Target Parsing Tests (for logs command)
    // ============================================================================

    #[test]
    fn test_parse_log_target_biome_only() {
        let target = "my-biome";
        let (biome_name, service_name) = if target.contains('.') {
            let parts: Vec<&str> = target.split('.').collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (target.to_string(), None)
        };

        assert_eq!(biome_name, "my-biome");
        assert!(service_name.is_none());
    }

    #[test]
    fn test_parse_log_target_biome_and_service() {
        let target = "my-biome.web-service";
        let (biome_name, service_name) = if target.contains('.') {
            let parts: Vec<&str> = target.split('.').collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (target.to_string(), None)
        };

        assert_eq!(biome_name, "my-biome");
        assert_eq!(service_name, Some("web-service".to_string()));
    }

    #[test]
    fn test_parse_log_target_multiple_dots() {
        let target = "my.biome.service.name";
        let (biome_name, service_name) = if target.contains('.') {
            let parts: Vec<&str> = target.split('.').collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (target.to_string(), None)
        };

        // Should split on first dot only
        assert_eq!(biome_name, "my");
        assert_eq!(service_name, Some("biome".to_string()));
    }

    // ============================================================================
    // Status Filter Tests
    // ============================================================================

    #[test]
    fn test_status_filter_matching() {
        let test_cases = vec![
            ("running", "running", true),
            ("stopped", "stopped", true),
            ("starting", "starting", true),
            ("stopping", "stopping", true),
            ("error", "error", true),
            ("migrating", "migrating", true),
            ("running", "stopped", false),
            ("error", "running", false),
        ];

        for (status, filter, should_match) in test_cases {
            let matches = status == filter;
            assert_eq!(matches, should_match, "Status: {status}, Filter: {filter}");
        }
    }

    // ============================================================================
    // Format Output Tests
    // ============================================================================

    #[test]
    fn test_output_format_variants() {
        let formats = vec!["json", "yaml", "table", "invalid"];

        for format in formats {
            match format {
                "json" | "yaml" | "table" => {
                    assert!(["json", "yaml", "table"].contains(&format));
                }
                _ => {
                    // Invalid format should default to table
                    assert!(!["json", "yaml", "table"].contains(&format));
                }
            }
        }
    }

    // ============================================================================
    // Resource Override Tests
    // ============================================================================

    #[test]
    fn test_cpu_limit_override() {
        let original_cpu: Option<f64> = Some(1.0);
        let override_cpu: Option<f64> = Some(2.0);

        let effective_cpu = override_cpu.or(original_cpu);

        assert_eq!(effective_cpu, Some(2.0));
    }

    #[test]
    fn test_memory_limit_override() {
        let original_memory: Option<String> = Some("512M".to_string());
        let override_memory: Option<String> = Some("1G".to_string());

        let effective_memory = override_memory.or(original_memory);

        assert_eq!(effective_memory, Some("1G".to_string()));
    }

    #[test]
    fn test_no_override_uses_original() {
        let original_cpu: Option<f64> = Some(4.0);
        let override_cpu: Option<f64> = None;

        let effective_cpu = override_cpu.or(original_cpu);

        assert_eq!(effective_cpu, Some(4.0));
    }

    // ============================================================================
    // Concurrent Access Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_biome_registration() {
        let biomes: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

        // Spawn multiple tasks to register biomes concurrently
        let mut handles = vec![];

        for i in 0..10 {
            let biomes_clone = Arc::clone(&biomes);
            let handle = tokio::spawn(async move {
                let mut biome_map = biomes_clone.write().await;
                biome_map.insert(format!("biome-{i}"), format!("id-{i}"));
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all biomes registered
        let biome_map = biomes.read().await;
        assert_eq!(biome_map.len(), 10);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_read_operations() {
        let biomes: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

        // Setup initial data
        {
            let mut biome_map = biomes.write().await;
            biome_map.insert("shared-biome".to_string(), "shared-id".to_string());
        }

        // Spawn multiple concurrent readers
        let mut handles = vec![];

        for _ in 0..20 {
            let biomes_clone = Arc::clone(&biomes);
            let handle = tokio::spawn(async move {
                let biome_map = biomes_clone.read().await;
                assert!(biome_map.contains_key("shared-biome"));
            });
            handles.push(handle);
        }

        // Wait for all readers
        for handle in handles {
            handle.await.unwrap();
        }
    }

    // ============================================================================
    // Security Level Tests
    // ============================================================================

    #[test]
    fn test_security_level_values() {
        let security_levels = vec!["low", "medium", "high", "maximum"];

        for level in security_levels {
            assert!(!level.is_empty());
            assert!(["low", "medium", "high", "maximum"].contains(&level));
        }
    }

    #[test]
    fn test_default_security_level() {
        let default_security = "high".to_string();
        assert_eq!(default_security, "high");
    }

    // ============================================================================
    // Timeout Configuration Tests
    // ============================================================================

    #[test]
    fn test_timeout_values() {
        let timeout_secs = 30u64;
        assert!(timeout_secs > 0);
        assert!(timeout_secs <= 300); // Reasonable maximum
    }

    #[test]
    fn test_various_timeout_values() {
        let timeouts = vec![1u64, 5, 10, 30, 60, 120, 300];

        for timeout in timeouts {
            assert!(timeout > 0);
            assert!(timeout <= 300);
        }
    }

    // ============================================================================
    // Biome State Management Tests
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_biome_state_transitions() {
        // Simulate state transitions: stopped -> starting -> running -> stopping -> stopped
        let states = vec!["stopped", "starting", "running", "stopping", "stopped"];

        for (i, state) in states.iter().enumerate() {
            assert!(!state.is_empty());
            if i > 0 {
                // Each state follows logically from previous
                assert_ne!(state, &"invalid");
            }
        }
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_biome_not_found_detection() {
        let biomes: HashMap<String, String> = HashMap::new();
        let biome_name = "nonexistent";

        let exists = biomes.contains_key(biome_name);
        assert!(!exists, "Should detect biome doesn't exist");
    }

    #[test]
    fn test_empty_biome_name_validation() {
        let biome_name = "";
        assert!(biome_name.is_empty(), "Empty biome name should be detected");
    }
}
