// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

#[cfg(test)]
mod plugin_system_tests {
    use super::super::*;

    fn create_test_manifest(name: &str) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            plugin_type: "test".to_string(),
            entry_point: format!("lib{name}.so"),
            ..Default::default()
        }
    }

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert_eq!(manager.list_plugins().len(), 0);
    }

    #[test]
    fn test_register_plugin() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        let result = manager.register_plugin(manifest);
        assert!(result.is_ok());
        assert_eq!(manager.list_plugins().len(), 1);
    }

    #[test]
    fn test_load_plugin() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        manager
            .register_plugin(manifest)
            .expect("register should succeed");
        let result = manager.load_plugin("test-plugin");
        assert!(result.is_err());
        let err = result.expect_err("expected LoadFailed");
        assert!(matches!(err, PluginError::LoadFailed(_)));
        assert!(err.to_string().contains("deprecated"));

        let info = manager
            .get_plugin_info("test-plugin")
            .expect("plugin info should exist");
        assert_eq!(info.state, PluginState::Failed);
        assert!(manager.active_plugins().is_empty());
    }

    #[test]
    fn test_unload_plugin() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        manager
            .register_plugin(manifest)
            .expect("register should succeed");
        assert!(manager.load_plugin("test-plugin").is_err());

        let result = manager.unload_plugin("test-plugin");
        assert!(result.is_ok());

        let active = manager.active_plugins();
        assert_eq!(active.len(), 0);
        let info = manager
            .get_plugin_info("test-plugin")
            .expect("plugin info should exist");
        assert_eq!(info.state, PluginState::Unloaded);
    }

    #[test]
    fn test_plugin_dependencies() {
        let mut manager = PluginManager::new();

        let dep_manifest = create_test_manifest("dependency");
        manager
            .register_plugin(dep_manifest)
            .expect("register dep should succeed");

        let mut main_manifest = create_test_manifest("main-plugin");
        main_manifest.dependencies = vec!["dependency".to_string()];

        let result = manager.register_plugin(main_manifest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_dependency() {
        let mut manager = PluginManager::new();

        let mut manifest = create_test_manifest("main-plugin");
        manifest.dependencies = vec!["missing-dep".to_string()];

        let result = manager.register_plugin(manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_manifest() {
        let mut manager = PluginManager::new();

        let manifest = PluginManifest {
            name: String::new(), // Invalid: empty name
            version: "1.0.0".to_string(),
            plugin_type: "test".to_string(),
            entry_point: "lib.so".to_string(),
            ..Default::default()
        };

        let result = manager.register_plugin(manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_limit() {
        let config = PluginConfig {
            max_plugins: 2,
            ..Default::default()
        };

        let mut manager = PluginManager::with_config(config);

        manager
            .register_plugin(create_test_manifest("plugin1"))
            .expect("register plugin1 should succeed");
        manager
            .register_plugin(create_test_manifest("plugin2"))
            .expect("register plugin2 should succeed");

        let result = manager.register_plugin(create_test_manifest("plugin3"));
        assert!(result.is_err());
    }

    #[test]
    fn test_plugins_by_type() {
        let mut manager = PluginManager::new();

        let mut manifest1 = create_test_manifest("provider1");
        manifest1.plugin_type = "cloud_provider".to_string();

        let mut manifest2 = create_test_manifest("provider2");
        manifest2.plugin_type = "cloud_provider".to_string();

        let mut manifest3 = create_test_manifest("storage1");
        manifest3.plugin_type = "storage".to_string();

        manager
            .register_plugin(manifest1)
            .expect("register manifest1 should succeed");
        manager
            .register_plugin(manifest2)
            .expect("register manifest2 should succeed");
        manager
            .register_plugin(manifest3)
            .expect("register manifest3 should succeed");

        let cloud_plugins = manager.plugins_by_type("cloud_provider");
        assert_eq!(cloud_plugins.len(), 2);

        let storage_plugins = manager.plugins_by_type("storage");
        assert_eq!(storage_plugins.len(), 1);
    }

    #[test]
    fn test_typed_registry() {
        let mut registry: TypedPluginRegistry<i32> = TypedPluginRegistry::new();

        registry.register("test1".to_string(), 42);
        registry.register("test2".to_string(), 100);

        assert_eq!(registry.list().len(), 2);
        assert_eq!(*registry.get("test1").expect("test1 should exist"), 42);
        assert!(registry.has("test2"));
    }

    #[test]
    fn test_plugin_state_transitions() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        manager
            .register_plugin(manifest)
            .expect("register should succeed");

        let info = manager
            .get_plugin_info("test-plugin")
            .expect("plugin info should exist");
        assert_eq!(info.state, PluginState::Registered);

        manager
            .load_plugin("test-plugin")
            .expect_err("load should fail for deprecated C FFI");
        let info = manager
            .get_plugin_info("test-plugin")
            .expect("plugin info should exist");
        assert_eq!(info.state, PluginState::Failed);

        manager
            .unload_plugin("test-plugin")
            .expect("unload should succeed");
        let info = manager
            .get_plugin_info("test-plugin")
            .expect("plugin info should exist");
        assert_eq!(info.state, PluginState::Unloaded);
    }

    #[test]
    fn test_register_plugin_disabled_returns_config_error() {
        let config = PluginConfig {
            enabled: false,
            ..Default::default()
        };
        let mut manager = PluginManager::with_config(config);
        let manifest = create_test_manifest("test");
        let result = manager.register_plugin(manifest);
        assert!(result.is_err());
        let err = result.expect_err("expected ConfigError");
        assert!(matches!(err, PluginError::ConfigError(_)));
    }

    #[test]
    fn test_load_plugin_not_found() {
        let mut manager = PluginManager::new();
        let result = manager.load_plugin("nonexistent");
        assert!(result.is_err());
        let err = result.expect_err("expected NotFound");
        assert!(matches!(err, PluginError::NotFound(_)));
    }

    #[test]
    fn test_unload_plugin_not_found() {
        let mut manager = PluginManager::new();
        let result = manager.unload_plugin("nonexistent");
        assert!(result.is_err());
        let err = result.expect_err("expected NotFound");
        assert!(matches!(err, PluginError::NotFound(_)));
    }

    #[test]
    fn test_invalid_manifest_empty_version() {
        let mut manager = PluginManager::new();
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: String::new(),
            plugin_type: "test".to_string(),
            entry_point: "lib.so".to_string(),
            ..Default::default()
        };
        let result = manager.register_plugin(manifest);
        assert!(result.is_err());
        let err = result.expect_err("expected InvalidManifest");
        assert!(matches!(err, PluginError::InvalidManifest(_)));
    }

    #[test]
    fn test_invalid_manifest_empty_plugin_type() {
        let mut manager = PluginManager::new();
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: "1.0".to_string(),
            plugin_type: String::new(),
            entry_point: "lib.so".to_string(),
            ..Default::default()
        };
        let result = manager.register_plugin(manifest);
        assert!(result.is_err());
        let err = result.expect_err("expected InvalidManifest");
        assert!(matches!(err, PluginError::InvalidManifest(_)));
    }

    #[test]
    fn test_invalid_manifest_empty_entry_point() {
        let mut manager = PluginManager::new();
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: "1.0".to_string(),
            plugin_type: "test".to_string(),
            entry_point: String::new(),
            ..Default::default()
        };
        let result = manager.register_plugin(manifest);
        assert!(result.is_err());
        let err = result.expect_err("expected InvalidManifest");
        assert!(matches!(err, PluginError::InvalidManifest(_)));
    }

    #[test]
    fn test_discover_plugins_finds_manifest_in_subdir() {
        let temp = tempfile::tempdir().expect("temp dir");
        let plugin_dir = temp.path().join("discovered-plugin");
        std::fs::create_dir_all(&plugin_dir).expect("create plugin dir");
        let manifest = create_test_manifest("discovered-plugin");
        let content = serde_json::to_string(&manifest).expect("serialize");
        std::fs::write(plugin_dir.join("plugin.json"), content).expect("write");

        let mut manager = PluginManager::new();
        manager.add_search_path(temp.path().to_path_buf());

        let discovered = manager.discover_plugins();
        let found = discovered.iter().find(|m| m.name == "discovered-plugin");
        assert!(
            found.is_some(),
            "Should discover plugin in subdir, got {discovered:?}"
        );
        assert_eq!(
            found.expect("discovered-plugin should be found").version,
            "1.0.0"
        );
    }

    #[test]
    fn test_discover_plugins_invalid_json_skipped() {
        let temp_dir = std::env::temp_dir().join("toadstool_plugin_invalid_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manifest_path = temp_dir.join("plugin.json");
        std::fs::write(&manifest_path, "{ invalid json }")
            .expect("write invalid json should succeed");

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_dir.clone());

        let discovered = manager.discover_plugins();
        assert!(discovered.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_plugin_config_defaults() {
        let config = PluginConfig::default();
        assert!(config.enabled);
        assert!(!config.require_signatures);
        assert_eq!(config.max_plugins, 100);
        assert_eq!(config.plugin_timeout_secs, 30);
    }

    #[test]
    fn test_plugin_error_display() {
        let err = PluginError::NotFound("x".to_string());
        assert!(format!("{err}").contains('x'));

        let err = PluginError::DependencyNotMet("a requires b".to_string());
        assert!(format!("{err}").contains("a requires b"));
    }

    #[test]
    fn test_plugin_manager_with_config() {
        let config = PluginConfig {
            max_plugins: 5,
            plugin_timeout_secs: 60,
            ..Default::default()
        };
        let manager = PluginManager::with_config(config);
        assert_eq!(manager.list_plugins().len(), 0);
    }

    #[test]
    fn test_add_search_path_and_search_paths() {
        let mut manager = PluginManager::new();
        let extra = std::path::PathBuf::from("/custom/plugins");
        manager.add_search_path(extra.clone());
        let paths = manager.search_paths();
        assert!(paths.iter().any(|p| p == &extra));
    }

    #[test]
    fn test_get_plugin_info_nonexistent() {
        let manager = PluginManager::new();
        assert!(manager.get_plugin_info("nonexistent").is_none());
    }

    #[test]
    fn test_get_plugin_info_returns_manifest() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("info-plugin");
        manager
            .register_plugin(manifest)
            .expect("register should succeed");
        let info = manager
            .get_plugin_info("info-plugin")
            .expect("plugin info should exist");
        assert_eq!(info.manifest.name, "info-plugin");
        assert_eq!(info.manifest.version, "1.0.0");
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let manifest = create_test_manifest("ser-test");
        let json = serde_json::to_string(&manifest).expect("serialize manifest");
        let restored: PluginManifest = serde_json::from_str(&json).expect("deserialize manifest");
        assert_eq!(restored.name, manifest.name);
        assert_eq!(restored.plugin_type, manifest.plugin_type);
    }

    #[test]
    fn test_plugin_manifest_default() {
        let m = PluginManifest::default();
        assert!(m.name.is_empty());
        assert!(m.version.is_empty());
        assert!(m.plugin_type.is_empty());
        assert!(m.entry_point.is_empty());
        assert!(m.dependencies.is_empty());
        assert!(m.provides.is_empty());
    }
}
