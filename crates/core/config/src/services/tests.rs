// SPDX-License-Identifier: AGPL-3.0-or-later
#[cfg(test)]
#[allow(unsafe_code)] // env::set_var/remove_var are unsafe in Rust 2024; test-only usage
mod service_registry_tests {
    use super::super::*;

    #[test]
    fn test_services_mod_re_exports() {
        let _registry = ServiceRegistry::default();
        let _ep = ServiceEndpoint::new("test", ServiceType::Compute, "http://localhost:8080");
        let _err = ServiceError::NotFound("x".to_string());
    }

    #[test]
    fn test_service_type_parsing() {
        assert_eq!(
            ServiceType::parse_type("coordinator"),
            ServiceType::Coordinator
        );
        assert_eq!(ServiceType::parse_type("storage"), ServiceType::Storage);
        assert_eq!(ServiceType::parse_type("compute"), ServiceType::Compute);

        match ServiceType::parse_type("custom") {
            ServiceType::Custom(s) => assert_eq!(s, "custom"),
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_service_endpoint_builder() {
        let endpoint = ServiceEndpoint::new("test", ServiceType::Compute, "http://localhost:8080")
            .with_port(8080)
            .with_capability("wasm")
            .with_health_check("/health");

        assert_eq!(endpoint.name, "test");
        assert_eq!(endpoint.port, Some(8080));
        assert_eq!(endpoint.capabilities, vec!["wasm"]);
        assert_eq!(endpoint.health_check, Some("/health".to_string()));
    }

    #[test]
    fn test_service_registry_register() {
        let mut registry = ServiceRegistry::new();

        let songbird = ServiceEndpoint::new(
            "songbird",
            ServiceType::Coordinator,
            "http://localhost:7777",
        );
        registry.register(songbird).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.get("songbird").is_some());
    }

    #[test]
    fn test_service_registry_find_by_type() {
        let mut registry = ServiceRegistry::new();

        registry
            .register(ServiceEndpoint::new(
                "songbird",
                ServiceType::Coordinator,
                "http://localhost:7777",
            ))
            .unwrap();
        registry
            .register(ServiceEndpoint::new(
                "squirrel",
                ServiceType::Storage,
                "http://localhost:8888",
            ))
            .unwrap();

        let coordinators = registry.find_by_type(&ServiceType::Coordinator);
        assert_eq!(coordinators.len(), 1);
        assert_eq!(coordinators[0].name, "songbird");

        let storage = registry.find_by_type(&ServiceType::Storage);
        assert_eq!(storage.len(), 1);
        assert_eq!(storage[0].name, "squirrel");
    }

    #[test]
    fn test_service_registry_coordinator() {
        let mut registry = ServiceRegistry::new();

        registry
            .register(ServiceEndpoint::new(
                "songbird",
                ServiceType::Coordinator,
                "http://localhost:7777",
            ))
            .unwrap();

        let coord = registry.coordinator();
        assert!(coord.is_some());
        assert_eq!(coord.unwrap().name, "songbird");
    }

    #[test]
    fn test_service_registry_storage() {
        let mut registry = ServiceRegistry::new();

        registry
            .register(ServiceEndpoint::new(
                "squirrel",
                ServiceType::Storage,
                "http://localhost:8888",
            ))
            .unwrap();

        let storage = registry.storage();
        assert!(storage.is_some());
        assert_eq!(storage.unwrap().name, "squirrel");
    }

    #[test]
    fn test_service_registry_already_registered() {
        let mut registry = ServiceRegistry::new();

        registry
            .register(ServiceEndpoint::new(
                "test",
                ServiceType::Compute,
                "http://localhost:8080",
            ))
            .unwrap();

        let result = registry.register(ServiceEndpoint::new(
            "test",
            ServiceType::Compute,
            "http://localhost:9090",
        ));
        assert!(matches!(result, Err(ServiceError::AlreadyRegistered(_))));
    }

    #[test]
    fn test_service_registry_register_or_update() {
        let mut registry = ServiceRegistry::new();

        registry.register_or_update(ServiceEndpoint::new(
            "test",
            ServiceType::Compute,
            "http://localhost:8080",
        ));
        assert_eq!(
            registry.get("test").unwrap().endpoint,
            "http://localhost:8080"
        );

        registry.register_or_update(ServiceEndpoint::new(
            "test",
            ServiceType::Compute,
            "http://localhost:9090",
        ));
        assert_eq!(
            registry.get("test").unwrap().endpoint,
            "http://localhost:9090"
        );
    }

    #[test]
    fn test_service_type_parse_all_variants() {
        assert_eq!(
            ServiceType::parse_type("coordinator"),
            ServiceType::Coordinator
        );
        assert_eq!(ServiceType::parse_type("storage"), ServiceType::Storage);
        assert_eq!(ServiceType::parse_type("compute"), ServiceType::Compute);
        assert_eq!(ServiceType::parse_type("messaging"), ServiceType::Messaging);
        assert_eq!(ServiceType::parse_type("database"), ServiceType::Database);
        assert_eq!(ServiceType::parse_type("cache"), ServiceType::Cache);
        assert_eq!(
            ServiceType::parse_type("monitoring"),
            ServiceType::Monitoring
        );
    }

    #[test]
    fn test_service_type_parse_case_insensitive() {
        assert_eq!(
            ServiceType::parse_type("COORDINATOR"),
            ServiceType::Coordinator
        );
        assert_eq!(ServiceType::parse_type("Storage"), ServiceType::Storage);
        assert_eq!(ServiceType::parse_type("CoMpUtE"), ServiceType::Compute);
    }

    #[test]
    fn test_service_type_as_str_all_variants() {
        assert_eq!(ServiceType::Coordinator.as_str(), "coordinator");
        assert_eq!(ServiceType::Storage.as_str(), "storage");
        assert_eq!(ServiceType::Compute.as_str(), "compute");
        assert_eq!(ServiceType::Messaging.as_str(), "messaging");
        assert_eq!(ServiceType::Database.as_str(), "database");
        assert_eq!(ServiceType::Cache.as_str(), "cache");
        assert_eq!(ServiceType::Monitoring.as_str(), "monitoring");
        assert_eq!(
            ServiceType::Custom("my-service".into()).as_str(),
            "my-service"
        );
    }

    #[test]
    fn test_service_type_serialization_roundtrip() {
        let types = [
            ServiceType::Coordinator,
            ServiceType::Storage,
            ServiceType::Custom("mytype".into()),
        ];
        for st in &types {
            let json = serde_json::to_string(st).unwrap();
            let parsed: ServiceType = serde_json::from_str(&json).unwrap();
            assert_eq!(st, &parsed);
        }
    }

    #[test]
    fn test_service_endpoint_defaults() {
        let ep = ServiceEndpoint::new("svc", ServiceType::Compute, "http://localhost:9000");
        assert_eq!(ep.name, "svc");
        assert_eq!(ep.service_type, ServiceType::Compute);
        assert_eq!(ep.endpoint, "http://localhost:9000");
        assert_eq!(ep.port, None);
        assert!(ep.capabilities.is_empty());
        assert_eq!(ep.health_check, None);
        assert!(ep.metadata.is_empty());
    }

    #[test]
    fn test_service_endpoint_with_metadata() {
        let ep = ServiceEndpoint::new("svc", ServiceType::Compute, "http://localhost:9000")
            .with_metadata("version", "1.0")
            .with_metadata("region", "us-east");
        assert_eq!(ep.metadata.get("version"), Some(&"1.0".to_string()));
        assert_eq!(ep.metadata.get("region"), Some(&"us-east".to_string()));
    }

    #[test]
    fn test_service_endpoint_with_capabilities() {
        let ep = ServiceEndpoint::new("svc", ServiceType::Compute, "http://localhost:9000")
            .with_capabilities(vec!["wasm".into(), "container".into()]);
        assert_eq!(ep.capabilities, vec!["wasm", "container"]);
    }

    #[test]
    fn test_service_endpoint_builder_chain() {
        let ep = ServiceEndpoint::new("toadstool", ServiceType::Compute, "http://127.0.0.1:8084")
            .with_port(8084)
            .with_capability("wasm")
            .with_capability("container")
            .with_health_check("/health")
            .with_metadata("env", "dev");
        assert_eq!(ep.port, Some(8084));
        assert_eq!(ep.capabilities.len(), 2);
        assert_eq!(ep.health_check.as_deref(), Some("/health"));
        assert_eq!(ep.metadata.get("env"), Some(&"dev".to_string()));
    }

    #[test]
    fn test_service_endpoint_serialization_roundtrip() {
        let ep = ServiceEndpoint::new("test", ServiceType::Coordinator, "http://localhost:7777")
            .with_port(7777)
            .with_capability("coordination")
            .with_health_check("/health")
            .with_metadata("version", "2.0");
        let json = serde_json::to_string(&ep).unwrap();
        let parsed: ServiceEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(ep.name, parsed.name);
        assert_eq!(ep.endpoint, parsed.endpoint);
        assert_eq!(ep.port, parsed.port);
        assert_eq!(ep.capabilities, parsed.capabilities);
        assert_eq!(ep.health_check, parsed.health_check);
        assert_eq!(ep.metadata, parsed.metadata);
    }

    #[test]
    fn test_service_registry_default() {
        let registry = ServiceRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_service_registry_is_empty() {
        let mut registry = ServiceRegistry::new();
        assert!(registry.is_empty());
        registry
            .register(ServiceEndpoint::new(
                "a",
                ServiceType::Compute,
                "http://a:1",
            ))
            .unwrap();
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_service_registry_all_services() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ServiceEndpoint::new(
                "a",
                ServiceType::Compute,
                "http://a:1",
            ))
            .unwrap();
        registry
            .register(ServiceEndpoint::new(
                "b",
                ServiceType::Storage,
                "http://b:2",
            ))
            .unwrap();
        let all = registry.all_services();
        assert_eq!(all.len(), 2);
        let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_service_registry_compute() {
        let mut registry = ServiceRegistry::new();
        assert!(registry.compute().is_none());

        registry
            .register(ServiceEndpoint::new(
                "toadstool",
                ServiceType::Compute,
                "http://localhost:8084",
            ))
            .unwrap();
        let compute = registry.compute().unwrap();
        assert_eq!(compute.name, "toadstool");
    }

    #[test]
    fn test_service_registry_get_nonexistent() {
        let registry = ServiceRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_service_registry_find_by_type_empty() {
        let registry = ServiceRegistry::new();
        let results = registry.find_by_type(&ServiceType::Coordinator);
        assert!(results.is_empty());
    }

    #[test]
    fn test_service_registry_register_or_update_type_change() {
        let mut registry = ServiceRegistry::new();
        registry.register_or_update(ServiceEndpoint::new(
            "svc",
            ServiceType::Compute,
            "http://localhost:8080",
        ));
        assert_eq!(registry.find_by_type(&ServiceType::Compute).len(), 1);
        assert_eq!(registry.find_by_type(&ServiceType::Storage).len(), 0);

        registry.register_or_update(ServiceEndpoint::new(
            "svc",
            ServiceType::Storage,
            "http://localhost:8081",
        ));
        assert_eq!(registry.find_by_type(&ServiceType::Compute).len(), 0);
        assert_eq!(registry.find_by_type(&ServiceType::Storage).len(), 1);
        assert_eq!(
            registry.get("svc").unwrap().service_type,
            ServiceType::Storage
        );
    }

    #[test]
    fn test_service_registry_from_toml_file_missing() {
        let result = ServiceRegistry::from_toml_file("/nonexistent/path/to/services.toml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Io(_)));
    }

    #[test]
    fn test_service_registry_from_json_file_missing() {
        let result = ServiceRegistry::from_json_file("/nonexistent/path/to/services.json");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Io(_)));
    }

    #[test]
    fn test_service_registry_from_json_file_invalid() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("toadstool_services_invalid_test.json");
        let mut f = std::fs::File::create(&temp).unwrap();
        f.write_all(b"{ invalid json }").unwrap();
        drop(f);

        let result = ServiceRegistry::from_json_file(&temp);
        std::fs::remove_file(&temp).ok();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Parse(_)));
    }

    #[test]
    fn test_service_registry_from_toml_file_valid() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("toadstool_services_valid_test.toml");
        let content = r#"
[services.songbird]
name = "songbird"
type = "coordinator"
endpoint = "http://localhost:7777"
port = 7777
"#;
        let mut f = std::fs::File::create(&temp).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        drop(f);

        let result = ServiceRegistry::from_toml_file(&temp);
        std::fs::remove_file(&temp).ok();
        let registry = result.unwrap();
        assert_eq!(registry.len(), 1);
        let songbird = registry.get("songbird").unwrap();
        assert_eq!(songbird.service_type, ServiceType::Coordinator);
        assert_eq!(songbird.endpoint, "http://localhost:7777");
    }

    #[test]
    fn test_service_registry_from_json_file_valid() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("toadstool_services_valid_test.json");
        let content = r#"{"services":{"squirrel":{"name":"squirrel","type":"storage","endpoint":"http://localhost:8888","port":8888}}}"#;
        let mut f = std::fs::File::create(&temp).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        drop(f);

        let result = ServiceRegistry::from_json_file(&temp);
        std::fs::remove_file(&temp).ok();
        let registry = result.unwrap();
        assert_eq!(registry.len(), 1);
        let squirrel = registry.get("squirrel").unwrap();
        assert_eq!(squirrel.service_type, ServiceType::Storage);
    }

    #[test]
    fn test_service_registry_serialization_roundtrip() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ServiceEndpoint::new(
                "songbird",
                ServiceType::Coordinator,
                "http://localhost:7777",
            ))
            .unwrap();
        registry
            .register(ServiceEndpoint::new(
                "squirrel",
                ServiceType::Storage,
                "http://localhost:8888",
            ))
            .unwrap();

        let json = serde_json::to_string(&registry).unwrap();
        let parsed: ServiceRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(parsed.get("songbird").is_some());
        assert!(parsed.get("squirrel").is_some());
    }

    #[test]
    fn test_service_error_display() {
        let err = ServiceError::NotFound("svc".into());
        assert!(format!("{err}").contains("svc"));

        let err = ServiceError::AlreadyRegistered("dup".into());
        assert!(format!("{err}").contains("dup"));

        let err = ServiceError::InvalidConfig("bad".into());
        assert!(format!("{err}").contains("bad"));
    }

    #[test]
    fn test_service_registry_from_env_coordinator() {
        let coord_key = "TOADSTOOL_COORDINATOR";
        let orig = std::env::var(coord_key).ok();
        // SAFETY: Test-only; sequential test execution
        unsafe { std::env::set_var(coord_key, "songbird:http://localhost:7777") };

        let registry = ServiceRegistry::from_env();
        let coord = registry.coordinator();
        assert!(coord.is_some());
        assert_eq!(coord.unwrap().name, "songbird");
        assert_eq!(coord.unwrap().endpoint, "http://localhost:7777");

        if let Some(v) = orig {
            unsafe { std::env::set_var(coord_key, v) };
        } else {
            unsafe { std::env::remove_var(coord_key) };
        }
    }

    #[test]
    fn test_service_registry_from_env_storage() {
        let storage_key = "TOADSTOOL_STORAGE";
        let orig = std::env::var(storage_key).ok();
        // SAFETY: Test-only; sequential test execution
        unsafe { std::env::set_var(storage_key, "squirrel:http://localhost:8888") };

        let registry = ServiceRegistry::from_env();
        let storage = registry.storage();
        assert!(storage.is_some());
        assert_eq!(storage.unwrap().name, "squirrel");

        if let Some(v) = orig {
            unsafe { std::env::set_var(storage_key, v) };
        } else {
            unsafe { std::env::remove_var(storage_key) };
        }
    }

    #[test]
    fn test_service_registry_from_env_services_json() {
        let services_key = "TOADSTOOL_SERVICES";
        let orig = std::env::var(services_key).ok();
        let json = r#"[{"name":"custom","type":"cache","endpoint":"http://localhost:6379","capabilities":["redis"]}]"#;
        // SAFETY: Test-only; sequential test execution
        unsafe { std::env::set_var(services_key, json) };

        let registry = ServiceRegistry::from_env();
        let cache = registry.find_by_type(&ServiceType::Cache);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[0].name, "custom");
        assert_eq!(cache[0].endpoint, "http://localhost:6379");

        if let Some(v) = orig {
            unsafe { std::env::set_var(services_key, v) };
        } else {
            unsafe { std::env::remove_var(services_key) };
        }
    }

    #[test]
    fn test_service_registry_from_env_no_colon_ignored() {
        let coord_key = "TOADSTOOL_COORDINATOR";
        let orig = std::env::var(coord_key).ok();
        // SAFETY: Test-only; sequential test execution
        unsafe { std::env::set_var(coord_key, "no-colon-here") };

        let registry = ServiceRegistry::from_env();
        assert!(registry.coordinator().is_none());

        if let Some(v) = orig {
            unsafe { std::env::set_var(coord_key, v) };
        } else {
            unsafe { std::env::remove_var(coord_key) };
        }
    }

    #[test]
    fn test_service_registry_from_toml_file_invalid_toml() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("toadstool_services_invalid_test.toml");
        let mut f = std::fs::File::create(&temp).unwrap();
        f.write_all(b"invalid toml [[[ ").unwrap();
        drop(f);

        let result = ServiceRegistry::from_toml_file(&temp);
        std::fs::remove_file(&temp).ok();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ServiceError::Parse(_)),
            "Expected Parse error, got {err:?}"
        );
    }

    #[test]
    fn test_service_error_io_from_std_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let service_err: ServiceError = io_err.into();
        assert!(format!("{service_err}").contains("file not found"));
    }

    #[test]
    fn test_service_registry_toml_deserialize_empty_services() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("toadstool_services_empty_test.toml");
        let content = r"services = {}";
        let mut f = std::fs::File::create(&temp).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        drop(f);

        let result = ServiceRegistry::from_toml_file(&temp);
        std::fs::remove_file(&temp).ok();
        let registry = result.unwrap();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_service_registry_from_env_invalid_json_ignored() {
        let svc_key = "TOADSTOOL_SERVICES";
        let coord_key = "TOADSTOOL_COORDINATOR";
        let storage_key = "TOADSTOOL_STORAGE";
        let orig_svc = std::env::var(svc_key).ok();
        let orig_coord = std::env::var(coord_key).ok();
        let orig_storage = std::env::var(storage_key).ok();

        // SAFETY: Test-only; sequential test execution
        unsafe {
            std::env::remove_var(coord_key);
            std::env::remove_var(storage_key);
            std::env::set_var(svc_key, r"{ invalid json }");
        }

        let registry = ServiceRegistry::from_env();
        assert!(registry.is_empty());

        if let Some(v) = orig_svc {
            unsafe { std::env::set_var(svc_key, v) };
        } else {
            unsafe { std::env::remove_var(svc_key) };
        }
        if let Some(v) = orig_coord {
            unsafe { std::env::set_var(coord_key, v) };
        }
        if let Some(v) = orig_storage {
            unsafe { std::env::set_var(storage_key, v) };
        }
    }

    #[test]
    fn test_service_registry_find_by_type_multiple_of_same_type() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ServiceEndpoint::new(
                "coord1",
                ServiceType::Coordinator,
                "http://localhost:7777",
            ))
            .unwrap();
        registry
            .register(ServiceEndpoint::new(
                "coord2",
                ServiceType::Coordinator,
                "http://localhost:7778",
            ))
            .unwrap();

        let coords = registry.find_by_type(&ServiceType::Coordinator);
        assert_eq!(coords.len(), 2);
        let names: Vec<&str> = coords.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"coord1"));
        assert!(names.contains(&"coord2"));
    }

    #[test]
    fn test_service_endpoint_with_port_const() {
        let ep = ServiceEndpoint::new("svc", ServiceType::Compute, "http://localhost:9000")
            .with_port(9000);
        assert_eq!(ep.port, Some(9000));
    }

    #[test]
    fn test_service_registry_register_or_update_preserves_order() {
        let mut registry = ServiceRegistry::new();
        registry.register_or_update(ServiceEndpoint::new(
            "a",
            ServiceType::Compute,
            "http://a:1",
        ));
        registry.register_or_update(ServiceEndpoint::new(
            "b",
            ServiceType::Compute,
            "http://b:2",
        ));

        let compute = registry.find_by_type(&ServiceType::Compute);
        assert_eq!(compute.len(), 2);
    }

    #[test]
    fn test_service_type_custom_serialization() {
        let custom = ServiceType::Custom("my-service".to_string());
        let json = serde_json::to_string(&custom).unwrap();
        let parsed: ServiceType = serde_json::from_str(&json).unwrap();
        match parsed {
            ServiceType::Custom(s) => assert_eq!(s, "my-service"),
            _ => panic!("expected Custom"),
        }
    }

    #[test]
    fn test_service_error_parse_display() {
        let err = ServiceError::Parse("TOML parse error".into());
        assert!(format!("{err}").contains("parse"));
    }

    #[test]
    fn test_service_registry_from_env_coordinator_with_whitespace() {
        let key = "TOADSTOOL_COORDINATOR";
        let orig = std::env::var(key).ok();
        // SAFETY: Test-only; sequential test execution
        unsafe { std::env::set_var(key, "  songbird  :  http://localhost:7777  ") };

        let registry = ServiceRegistry::from_env();
        let coord = registry.coordinator();
        assert!(coord.is_some());
        assert_eq!(coord.unwrap().name, "songbird");

        if let Some(v) = orig {
            unsafe { std::env::set_var(key, v) };
        } else {
            unsafe { std::env::remove_var(key) };
        }
    }

    #[test]
    fn test_service_registry_get_returns_correct_endpoint() {
        let mut registry = ServiceRegistry::new();
        let ep = ServiceEndpoint::new("test", ServiceType::Cache, "http://cache:6379")
            .with_health_check("/health")
            .with_metadata("version", "1.0");
        registry.register(ep).unwrap();

        let retrieved = registry.get("test").unwrap();
        assert_eq!(retrieved.endpoint, "http://cache:6379");
        assert_eq!(retrieved.health_check.as_deref(), Some("/health"));
        assert_eq!(retrieved.metadata.get("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_service_registry_len_increments_on_register() {
        let mut registry = ServiceRegistry::new();
        assert_eq!(registry.len(), 0);
        registry
            .register(ServiceEndpoint::new(
                "a",
                ServiceType::Compute,
                "http://a:1",
            ))
            .unwrap();
        assert_eq!(registry.len(), 1);
        registry
            .register(ServiceEndpoint::new(
                "b",
                ServiceType::Storage,
                "http://b:2",
            ))
            .unwrap();
        assert_eq!(registry.len(), 2);
    }
}
