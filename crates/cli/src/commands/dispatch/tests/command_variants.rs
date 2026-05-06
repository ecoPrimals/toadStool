// SPDX-License-Identifier: AGPL-3.0-or-later
use std::path::PathBuf;

use crate::commands::definitions::{Commands, EcosystemCommands, UniversalCommands};
use crate::commands::dispatch::execute_command;
use crate::executor::{RunBiomeOptions, UpBiomeOptions};

#[test]
fn test_dispatch_module_compiles_and_commands_accessible() {
    let _cmd = Commands::Doctor {
        all: false,
        hardware: false,
        ecosystem: false,
        config: false,
        format: "text".to_string(),
        fix: false,
    };
    let _ = execute_command;
}

#[test]
fn test_commands_run_variant() {
    let cmd = Commands::Run {
        manifest: PathBuf::from("biome.yaml"),
        name: Some("test-biome".to_string()),
        env: vec!["KEY=value".to_string()],
        debug: true,
        cpu_limit: Some(2.0),
        memory_limit: Some("512Mi".to_string()),
        security: "high".to_string(),
    };
    match &cmd {
        Commands::Run {
            manifest,
            name,
            env,
            debug,
            cpu_limit,
            memory_limit,
            security,
        } => {
            assert_eq!(manifest, &PathBuf::from("biome.yaml"));
            assert_eq!(name.as_deref(), Some("test-biome"));
            assert_eq!(env.len(), 1);
            assert!(*debug);
            assert_eq!(*cpu_limit, Some(2.0));
            assert_eq!(memory_limit.as_deref(), Some("512Mi"));
            assert_eq!(security, "high");
        }
        _ => panic!("Expected Run variant"),
    }
}

#[test]
fn test_commands_up_variant() {
    let cmd = Commands::Up {
        manifest: PathBuf::from("biome.yaml"),
        detach: true,
        name: None,
        env: vec![],
        restart: true,
        health_interval: 60,
    };
    match &cmd {
        Commands::Up {
            manifest,
            detach,
            restart,
            health_interval,
            ..
        } => {
            assert_eq!(manifest, &PathBuf::from("biome.yaml"));
            assert!(*detach);
            assert!(*restart);
            assert_eq!(*health_interval, 60);
        }
        _ => panic!("Expected Up variant"),
    }
}

#[test]
fn test_commands_down_variant() {
    let cmd = Commands::Down {
        biome: "my-biome".to_string(),
        force: true,
        timeout: 10,
        purge: true,
    };
    match &cmd {
        Commands::Down {
            biome,
            force,
            timeout,
            purge,
        } => {
            assert_eq!(biome, "my-biome");
            assert!(*force);
            assert_eq!(*timeout, 10);
            assert!(*purge);
        }
        _ => panic!("Expected Down variant"),
    }
}

#[test]
fn test_commands_validate_variant() {
    let cmd = Commands::Validate {
        manifest: PathBuf::from("manifest.yaml"),
        check_resources: true,
        check_security: true,
        format: "json".to_string(),
    };
    match &cmd {
        Commands::Validate {
            manifest,
            check_resources,
            check_security,
            format,
        } => {
            assert_eq!(manifest, &PathBuf::from("manifest.yaml"));
            assert!(*check_resources);
            assert!(*check_security);
            assert_eq!(format, "json");
        }
        _ => panic!("Expected Validate variant"),
    }
}

#[test]
fn test_commands_init_variant() {
    let cmd = Commands::Init {
        path: PathBuf::from("."),
        template: "basic".to_string(),
        force: true,
    };
    match &cmd {
        Commands::Init {
            path,
            template,
            force,
        } => {
            assert_eq!(path, &PathBuf::from("."));
            assert_eq!(template, "basic");
            assert!(*force);
        }
        _ => panic!("Expected Init variant"),
    }
}

#[test]
fn test_commands_capabilities_variant() {
    let cmd = Commands::Capabilities {
        format: "table".to_string(),
        detailed: true,
        test_platform: Some("linux".to_string()),
    };
    match &cmd {
        Commands::Capabilities {
            format,
            detailed,
            test_platform,
        } => {
            assert_eq!(format, "table");
            assert!(*detailed);
            assert_eq!(test_platform.as_deref(), Some("linux"));
        }
        _ => panic!("Expected Capabilities variant"),
    }
}

#[test]
fn test_commands_ecosystem_variant() {
    let cmd = Commands::Ecosystem {
        action: EcosystemCommands::Discover {
            services: vec!["crypto".to_string()],
            timeout: 5,
        },
    };
    match &cmd {
        Commands::Ecosystem { action } => match action {
            EcosystemCommands::Discover { services, timeout } => {
                assert_eq!(services.len(), 1);
                assert_eq!(*timeout, 5);
            }
            _ => panic!("Expected Discover subcommand"),
        },
        _ => panic!("Expected Ecosystem variant"),
    }
}

#[test]
fn test_commands_universal_variant() {
    let cmd = Commands::Universal {
        operation: UniversalCommands::Detect {
            categories: vec!["traditional".to_string()],
            test: false,
            output: None,
        },
    };
    match &cmd {
        Commands::Universal { operation } => match operation {
            UniversalCommands::Detect {
                categories, test, ..
            } => {
                assert_eq!(categories.len(), 1);
                assert!(!*test);
            }
            _ => panic!("Expected Detect subcommand"),
        },
        _ => panic!("Expected Universal variant"),
    }
}

#[test]
fn test_run_biome_options_construction() {
    let opts = RunBiomeOptions {
        manifest_path: PathBuf::from("biome.yaml"),
        name: Some("test".to_string()),
        env: vec!["FOO=bar".to_string()],
        debug: false,
        cpu_limit: None,
        memory_limit: None,
        security: "medium".to_string(),
    };
    assert_eq!(opts.manifest_path, PathBuf::from("biome.yaml"));
    assert_eq!(opts.name.unwrap(), "test");
    assert_eq!(opts.env.len(), 1);
    assert_eq!(opts.security, "medium");
}

#[test]
fn test_up_biome_options_construction() {
    let opts = UpBiomeOptions {
        manifest_path: PathBuf::from("biome.yaml"),
        detach: true,
        name: None,
        env: vec![],
        restart: false,
        health_interval: 30,
    };
    assert_eq!(opts.manifest_path, PathBuf::from("biome.yaml"));
    assert!(opts.detach);
    assert_eq!(opts.health_interval, 30);
}

#[test]
fn test_commands_server_variant() {
    let cmd = Commands::Server {
        register: true,
        bind: None,
        port: 8080,
        socket: None,
        config: None,
        max_workloads: 10,
        biomeos_socket: None,
        family_id: None,
    };
    match &cmd {
        Commands::Server {
            register,
            port,
            max_workloads,
            ..
        } => {
            assert!(*register);
            assert_eq!(*port, 8080);
            assert_eq!(*max_workloads, 10);
        }
        _ => panic!("Expected Server variant"),
    }
}

#[test]
fn test_commands_byob_server_variant() {
    let cmd = Commands::ByobServer {
        bind: Some("0.0.0.0".to_string()),
        port: Some(8084),
        config: None,
    };
    match &cmd {
        Commands::ByobServer { bind, port, .. } => {
            assert_eq!(bind.as_deref(), Some("0.0.0.0"));
            assert_eq!(*port, Some(8084));
        }
        _ => panic!("Expected ByobServer variant"),
    }
}

#[test]
fn test_commands_transport_variant() {
    use crate::commands::definitions::TransportCommands;
    let cmd = Commands::Transport {
        action: TransportCommands::List {
            format: "text".to_string(),
        },
    };
    match &cmd {
        Commands::Transport { action } => {
            assert!(matches!(action, TransportCommands::List { .. }));
        }
        _ => panic!("Expected Transport variant"),
    }
}

#[test]
fn test_commands_execute_variant() {
    let cmd = Commands::Execute {
        workload: PathBuf::from("workload.wasm"),
        runtime: Some("wasm".to_string()),
        env: vec!["KEY=val".to_string()],
        timeout: 30,
        format: "json".to_string(),
    };
    match &cmd {
        Commands::Execute {
            workload,
            runtime,
            env,
            timeout,
            format,
        } => {
            assert_eq!(workload, &PathBuf::from("workload.wasm"));
            assert_eq!(runtime.as_deref(), Some("wasm"));
            assert_eq!(env.len(), 1);
            assert_eq!(*timeout, 30);
            assert_eq!(format, "json");
        }
        _ => panic!("Expected Execute variant"),
    }
}

#[test]
fn test_commands_logs_variant() {
    let cmd = Commands::Logs {
        target: "biome".to_string(),
        follow: false,
        lines: 100,
        timestamps: true,
        level: Some("info".to_string()),
        grep: None,
    };
    match &cmd {
        Commands::Logs {
            target,
            follow,
            lines,
            timestamps,
            level,
            ..
        } => {
            assert_eq!(target, "biome");
            assert!(!*follow);
            assert_eq!(*lines, 100);
            assert!(*timestamps);
            assert_eq!(level.as_deref(), Some("info"));
        }
        _ => panic!("Expected Logs variant"),
    }
}

#[test]
fn test_commands_doctor_all_flags() {
    let cmd = Commands::Doctor {
        all: true,
        hardware: true,
        ecosystem: true,
        config: true,
        format: "json".to_string(),
        fix: true,
    };
    match &cmd {
        Commands::Doctor {
            all,
            hardware,
            ecosystem,
            config,
            fix,
            ..
        } => {
            assert!(*all);
            assert!(*hardware);
            assert!(*ecosystem);
            assert!(*config);
            assert!(*fix);
        }
        _ => panic!("Expected Doctor variant"),
    }
}
