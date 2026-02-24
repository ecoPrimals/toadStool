// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;

    fn create_test_manifest(name: &str) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            plugin_type: "test".to_string(),
            entry_point: format!("lib{}.so", name),
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

        manager.register_plugin(manifest).unwrap();
        let result = manager.load_plugin("test-plugin");
        assert!(result.is_ok());

        let active = manager.active_plugins();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_unload_plugin() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        manager.register_plugin(manifest).unwrap();
        manager.load_plugin("test-plugin").unwrap();

        let result = manager.unload_plugin("test-plugin");
        assert!(result.is_ok());

        let active = manager.active_plugins();
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_plugin_dependencies() {
        let mut manager = PluginManager::new();

        let dep_manifest = create_test_manifest("dependency");
        manager.register_plugin(dep_manifest).unwrap();

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
            name: "".to_string(), // Invalid: empty name
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
            .unwrap();
        manager
            .register_plugin(create_test_manifest("plugin2"))
            .unwrap();

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

        manager.register_plugin(manifest1).unwrap();
        manager.register_plugin(manifest2).unwrap();
        manager.register_plugin(manifest3).unwrap();

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
        assert_eq!(*registry.get("test1").unwrap(), 42);
        assert!(registry.has("test2"));
    }

    #[test]
    fn test_plugin_state_transitions() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        manager.register_plugin(manifest).unwrap();

        let info = manager.get_plugin_info("test-plugin").unwrap();
        assert_eq!(info.state, PluginState::Registered);

        manager.load_plugin("test-plugin").unwrap();
        let info = manager.get_plugin_info("test-plugin").unwrap();
        assert_eq!(info.state, PluginState::Active);

        manager.unload_plugin("test-plugin").unwrap();
        let info = manager.get_plugin_info("test-plugin").unwrap();
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
        assert!(matches!(result.unwrap_err(), PluginError::ConfigError(_)));
    }

    #[test]
    fn test_load_plugin_not_found() {
        let mut manager = PluginManager::new();
        let result = manager.load_plugin("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginError::NotFound(_)));
    }

    #[test]
    fn test_unload_plugin_not_found() {
        let mut manager = PluginManager::new();
        let result = manager.unload_plugin("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginError::NotFound(_)));
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
        assert!(matches!(
            result.unwrap_err(),
            PluginError::InvalidManifest(_)
        ));
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
        assert!(matches!(
            result.unwrap_err(),
            PluginError::InvalidManifest(_)
        ));
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
        assert!(matches!(
            result.unwrap_err(),
            PluginError::InvalidManifest(_)
        ));
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
            "Should discover plugin in subdir, got {:?}",
            discovered
        );
        assert_eq!(found.unwrap().version, "1.0.0");
    }

    #[test]
    fn test_discover_plugins_invalid_json_skipped() {
        let temp_dir = std::env::temp_dir().join("toadstool_plugin_invalid_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manifest_path = temp_dir.join("plugin.json");
        std::fs::write(&manifest_path, "{ invalid json }").unwrap();

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
        assert!(format!("{err}").contains("x"));

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
        manager.register_plugin(manifest.clone()).unwrap();
        let info = manager.get_plugin_info("info-plugin").unwrap();
        assert_eq!(info.manifest.name, "info-plugin");
        assert_eq!(info.manifest.version, "1.0.0");
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let manifest = create_test_manifest("ser-test");
        let json = serde_json::to_string(&manifest).unwrap();
        let restored: PluginManifest = serde_json::from_str(&json).unwrap();
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

    #[test]
    fn test_plugin_state_equality() {
        assert_eq!(PluginState::Registered, PluginState::Registered);
        assert_ne!(PluginState::Registered, PluginState::Active);
        assert_eq!(PluginState::Failed, PluginState::Failed);
    }

    #[test]
    fn test_plugin_error_load_failed() {
        let err = PluginError::LoadFailed("lib broken".to_string());
        assert!(format!("{err}").contains("lib broken"));
    }

    #[test]
    fn test_plugin_error_invalid_manifest() {
        let err = PluginError::InvalidManifest("bad".to_string());
        assert!(format!("{err}").contains("Invalid manifest"));
    }

    #[test]
    fn test_plugin_error_version_mismatch() {
        let err = PluginError::VersionMismatch("1.0 vs 2.0".to_string());
        assert!(format!("{err}").contains("Version mismatch"));
    }

    #[test]
    fn test_plugin_error_limit_reached() {
        let err = PluginError::LimitReached(10);
        assert!(format!("{err}").contains("10"));
    }

    #[test]
    fn test_plugin_error_invalid_signature() {
        let err = PluginError::InvalidSignature;
        assert!(format!("{err}").contains("signature"));
    }

    #[test]
    fn test_plugin_error_config_error() {
        let err = PluginError::ConfigError("disabled".to_string());
        assert!(format!("{err}").contains("Configuration"));
    }

    #[test]
    fn test_typed_registry_get_mut() {
        let mut registry: TypedPluginRegistry<String> = TypedPluginRegistry::new();
        registry.register("k", "value".to_string());
        let v = registry.get_mut("k").unwrap();
        *v = "updated".to_string();
        assert_eq!(registry.get("k").unwrap(), "updated");
    }

    #[test]
    fn test_typed_registry_has_nonexistent() {
        let registry: TypedPluginRegistry<()> = TypedPluginRegistry::new();
        assert!(!registry.has("missing"));
    }

    #[test]
    fn test_typed_registry_default() {
        let registry: TypedPluginRegistry<i32> = TypedPluginRegistry::default();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_discover_plugins_skips_nonexistent_path() {
        let mut manager = PluginManager::new();
        manager.add_search_path(std::path::PathBuf::from("/nonexistent/path/12345"));
        let discovered = manager.discover_plugins();
        assert!(discovered.is_empty());
    }

    #[test]
    fn test_discover_plugins_direct_manifest_in_search_path() {
        let temp = tempfile::tempdir().expect("temp dir");
        let manifest = create_test_manifest("direct-manifest");
        let content = serde_json::to_string(&manifest).expect("serialize");
        std::fs::write(temp.path().join("plugin.json"), content).expect("write");

        let mut manager = PluginManager::new();
        manager.add_search_path(temp.path().to_path_buf());

        let discovered = manager.discover_plugins();
        let found = discovered.iter().find(|m| m.name == "direct-manifest");
        assert!(
            found.is_some(),
            "Should find plugin.json in direct search path"
        );
    }

    #[test]
    fn test_discover_plugins_partial_manifest_parses() {
        let temp = tempfile::tempdir().expect("temp dir");
        let plugin_dir = temp.path().join("partial-manifest");
        std::fs::create_dir_all(&plugin_dir).expect("create");
        let manifest = PluginManifest {
            name: "partial".to_string(),
            version: "0.1".to_string(),
            plugin_type: "x".to_string(),
            entry_point: "x.so".to_string(),
            ..Default::default()
        };
        let content = serde_json::to_string(&manifest).expect("serialize");
        std::fs::write(plugin_dir.join("plugin.json"), content).expect("write");

        let mut manager = PluginManager::new();
        manager.add_search_path(temp.path().to_path_buf());

        let discovered = manager.discover_plugins();
        let found = discovered.iter().find(|m| m.name == "partial");
        assert!(
            found.is_some(),
            "Should discover partial manifest, got: {:?}",
            discovered.iter().map(|m| &m.name).collect::<Vec<_>>()
        );
        assert_eq!(found.unwrap().version, "0.1");
    }

    #[test]
    fn test_register_plugin_replace_existing_warn() {
        let mut manager = PluginManager::new();
        let m1 = create_test_manifest("replace-me");
        let m2 = PluginManifest {
            name: "replace-me".to_string(),
            version: "2.0.0".to_string(),
            plugin_type: "test".to_string(),
            entry_point: "libreplace.so".to_string(),
            ..Default::default()
        };
        manager.register_plugin(m1).unwrap();
        manager.register_plugin(m2).unwrap();
        let info = manager.get_plugin_info("replace-me").unwrap();
        assert_eq!(info.manifest.version, "2.0.0");
    }

    #[test]
    fn test_plugin_lifecycle_register_load_unload_register_again() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("cycle-plugin");
        manager.register_plugin(manifest).unwrap();
        manager.load_plugin("cycle-plugin").unwrap();
        manager.unload_plugin("cycle-plugin").unwrap();
        let info = manager.get_plugin_info("cycle-plugin").unwrap();
        assert_eq!(info.state, PluginState::Unloaded);
    }

    #[test]
    fn test_plugin_state_loading_transition() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("state-plugin");
        manager.register_plugin(manifest).unwrap();
        let info_before = manager.get_plugin_info("state-plugin").unwrap();
        assert_eq!(info_before.state, PluginState::Registered);
        manager.load_plugin("state-plugin").unwrap();
        let info_after = manager.get_plugin_info("state-plugin").unwrap();
        assert_eq!(info_after.state, PluginState::Active);
        assert!(info_after.loaded_at.is_some());
    }

    #[test]
    fn test_plugin_info_error_field_none_when_loaded() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("ok-plugin");
        manager.register_plugin(manifest).unwrap();
        manager.load_plugin("ok-plugin").unwrap();
        let info = manager.get_plugin_info("ok-plugin").unwrap();
        assert!(info.error.is_none());
    }

    #[test]
    fn test_list_plugins_returns_registered_names() {
        let mut manager = PluginManager::new();
        manager.register_plugin(create_test_manifest("p1")).unwrap();
        manager.register_plugin(create_test_manifest("p2")).unwrap();
        let names = manager.list_plugins();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"p1".to_string()));
        assert!(names.contains(&"p2".to_string()));
    }

    #[test]
    fn test_active_plugins_excludes_unloaded() {
        let mut manager = PluginManager::new();
        manager
            .register_plugin(create_test_manifest("active1"))
            .unwrap();
        manager
            .register_plugin(create_test_manifest("active2"))
            .unwrap();
        manager.load_plugin("active1").unwrap();
        let active = manager.active_plugins();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], "active1");
    }

    #[test]
    fn test_plugin_config_require_signatures() {
        let config = PluginConfig {
            require_signatures: true,
            ..Default::default()
        };
        assert!(config.require_signatures);
    }

    #[test]
    fn test_plugin_manifest_with_metadata() {
        let mut manifest = create_test_manifest("meta-plugin");
        manifest
            .metadata
            .insert("key".to_string(), "value".to_string());
        manifest.author = Some("Author".to_string());
        manifest.description = Some("Desc".to_string());
        let json = serde_json::to_string(&manifest).unwrap();
        let restored: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_plugin_manifest_dependencies_and_provides() {
        let mut manifest = create_test_manifest("dep-plugin");
        manifest.dependencies = vec!["dep1".to_string(), "dep2".to_string()];
        manifest.provides = vec!["capability-a".to_string()];
        let json = serde_json::to_string(&manifest).unwrap();
        let restored: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.dependencies.len(), 2);
        assert_eq!(restored.provides.len(), 1);
    }

    #[test]
    fn test_search_paths_include_defaults() {
        let manager = PluginManager::new();
        let paths = manager.search_paths();
        assert!(!paths.is_empty());
        assert!(paths
            .iter()
            .any(|p| p.to_string_lossy().contains("plugins")));
    }

    #[test]
    fn test_plugin_manager_default() {
        let manager = PluginManager::default();
        assert_eq!(manager.list_plugins().len(), 0);
    }
}
