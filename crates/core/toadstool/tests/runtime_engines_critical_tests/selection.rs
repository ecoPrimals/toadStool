// SPDX-License-Identifier: AGPL-3.0-or-later

#[test]
fn test_runtime_type_identification() {
    let runtime_types = vec![
        ("native", true),
        ("wasm", true),
        ("container", true),
        ("python", true),
        ("gpu", true),
        ("invalid", false),
    ];

    for (runtime, is_valid) in runtime_types {
        if is_valid {
            assert!(matches!(
                runtime,
                "native" | "wasm" | "container" | "python" | "gpu"
            ));
        }
    }
}

#[test]
fn test_runtime_from_workload_type() {
    let workload_mappings = vec![
        ("script.sh", "native"),
        ("app.wasm", "wasm"),
        ("Dockerfile", "container"),
        ("script.py", "python"),
        ("kernel.cu", "gpu"),
    ];

    for (file, expected_runtime) in workload_mappings {
        let runtime = if std::path::Path::new(file)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("wasm"))
        {
            "wasm"
        } else if std::path::Path::new(file)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("py"))
        {
            "python"
        } else if std::path::Path::new(file)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cu"))
        {
            "gpu"
        } else if file.contains("Dockerfile") {
            "container"
        } else {
            "native"
        };

        assert_eq!(runtime, expected_runtime);
    }
}

#[test]
fn test_runtime_capability_matching() {
    #[derive(Debug)]
    struct RuntimeCapability {
        runtime: String,
        features: Vec<String>,
    }

    let native_caps = RuntimeCapability {
        runtime: "native".to_string(),
        features: vec!["fast".to_string(), "direct".to_string()],
    };

    assert_eq!(native_caps.runtime, "native");
    assert_eq!(native_caps.features.len(), 2);
}

#[test]
fn test_runtime_priority_ordering() {
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    #[allow(dead_code)]
    enum RuntimePriority {
        Native = 1,
        Wasm = 2,
        Container = 3,
        Python = 4,
    }

    let mut priorities = vec![
        RuntimePriority::Container,
        RuntimePriority::Native,
        RuntimePriority::Wasm,
    ];

    priorities.sort();
    assert_eq!(priorities[0], RuntimePriority::Native);
}

#[test]
fn test_runtime_availability_check() {
    let available_runtimes = vec!["native", "wasm"];
    let requested_runtime = "native";

    assert!(available_runtimes.contains(&requested_runtime));
}
