// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dispatch command tests

#[cfg(test)]
#[expect(
    clippy::module_inception,
    reason = "module name matches parent for API clarity"
)]
mod tests {
    use std::path::PathBuf;

    use crate::commands::definitions::{Commands, EcosystemCommands, UniversalCommands};
    use crate::commands::dispatch::execute_command;
    use crate::executor::{RunBiomeOptions, UpBiomeOptions};
    use crate::{Cli, CliContext};
    use tempfile::TempDir;
    use tokio::fs;

    fn valid_manifest_yaml() -> &'static str {
        r#"
metadata:
  name: test-biome
  version: "1.0.0"
  created: 1735689600
  updated: 1735689600
  tags: []

primals: {}
services: {}

resources:
  cpu_limit: 2.0
  memory_limit: "2GB"

security:
  isolation_level: "high"
  trust_level: "medium"
  beardog_required: false
  crypto_policies: []
  allowed_networks: []
  forbidden_syscalls: []

networking:
  mode: "bridge"
  dns_servers: []
  port_mappings: []
  network_policies: []

storage:
  nestgate_integration: false
  datasets: []
  volumes: []
"#
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_validate_valid_manifest() {
        let temp_dir = TempDir::new().expect("temp dir");
        let manifest_path = temp_dir.path().join("biome.yaml");
        fs::write(&manifest_path, valid_manifest_yaml())
            .await
            .expect("write manifest");

        let cli = Cli {
            command: Commands::Validate {
                manifest: manifest_path,
                check_resources: false,
                check_security: false,
                format: "text".to_string(),
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "validate should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_validate_json_format() {
        let temp_dir = TempDir::new().expect("temp dir");
        let manifest_path = temp_dir.path().join("biome.yaml");
        fs::write(&manifest_path, valid_manifest_yaml())
            .await
            .expect("write manifest");

        let cli = Cli {
            command: Commands::Validate {
                manifest: manifest_path,
                check_resources: true,
                check_security: true,
                format: "json".to_string(),
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "validate json should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_validate_nonexistent_manifest() {
        let cli = Cli {
            command: Commands::Validate {
                manifest: PathBuf::from("/nonexistent/path/biome.yaml"),
                check_resources: false,
                check_security: false,
                format: "text".to_string(),
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_err(), "validate nonexistent should fail");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_init_science_template() {
        let temp_dir = TempDir::new().expect("temp dir");
        let cli = Cli {
            command: Commands::Init {
                path: temp_dir.path().to_path_buf(),
                template: "science".to_string(),
                force: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "init science should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_init_basic_template() {
        let temp_dir = TempDir::new().expect("temp dir");
        let output_dir = temp_dir.path().to_path_buf();

        let cli = Cli {
            command: Commands::Init {
                path: output_dir.clone(),
                template: "basic".to_string(),
                force: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "init basic should succeed: {:?}",
            result.err()
        );
        assert!(output_dir.join("biome.yaml").exists());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_init_invalid_template() {
        let temp_dir = TempDir::new().expect("temp dir");
        let cli = Cli {
            command: Commands::Init {
                path: temp_dir.path().to_path_buf(),
                template: "nonexistent-template".to_string(),
                force: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_err(), "init invalid template should fail");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_capabilities() {
        let cli = Cli {
            command: Commands::Capabilities {
                format: "table".to_string(),
                detailed: false,
                test_platform: None,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "capabilities should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_capabilities_with_test_platform() {
        let cli = Cli {
            command: Commands::Capabilities {
                format: "json".to_string(),
                detailed: true,
                test_platform: Some("linux".to_string()),
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "capabilities with test_platform should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_ecosystem_discover() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Discover {
                    services: vec!["crypto".to_string()],
                    timeout: 1,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "ecosystem discover should succeed (may return empty): {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_ecosystem_discover_empty_services() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Discover {
                    services: vec![],
                    timeout: 1,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "ecosystem discover empty should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_ecosystem_register_error_path() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Register {
                    endpoint: "127.0.0.1:1".to_string(),
                    token: None,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_err(),
            "ecosystem register to unreachable endpoint should fail"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_ecosystem_auth_error_path() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Auth {
                    permission_file: PathBuf::from("/nonexistent/permissions.json"),
                    validate_only: true,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_err(),
            "ecosystem auth with nonexistent file should fail"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_ecosystem_storage_error_path() {
        let cli = Cli {
            command: Commands::Ecosystem {
                action: EcosystemCommands::Storage {
                    endpoint: "http://127.0.0.1:1".to_string(),
                    mount: PathBuf::from("/data"),
                    dataset: None,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_err(),
            "ecosystem storage with unreachable endpoint should fail"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_universal_detect() {
        let cli = Cli {
            command: Commands::Universal {
                operation: UniversalCommands::Detect {
                    categories: vec!["traditional".to_string()],
                    test: false,
                    output: None,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "universal detect should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_universal_benchmark() {
        let cli = Cli {
            command: Commands::Universal {
                operation: UniversalCommands::Benchmark {
                    suite: "standard".to_string(),
                    platforms: vec![],
                    format: "json".to_string(),
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "universal benchmark should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_universal_migrate_error_path() {
        let cli = Cli {
            command: Commands::Universal {
                operation: UniversalCommands::Migrate {
                    source: "nonexistent-source".to_string(),
                    target: "nonexistent-target".to_string(),
                    pause: false,
                    verify: false,
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_err(),
            "universal migrate with nonexistent source should fail"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_universal_federate() {
        let cli = Cli {
            command: Commands::Universal {
                operation: UniversalCommands::Federate {
                    endpoint: "127.0.0.1:9999".to_string(),
                    mode: "peer".to_string(),
                    resources: vec![],
                },
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(
            result.is_ok(),
            "universal federate should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_down_nonexistent() {
        let cli = Cli {
            command: Commands::Down {
                biome: "nonexistent-biome-xyz".to_string(),
                force: false,
                timeout: 30,
                purge: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_err(), "down nonexistent should fail");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_ps() {
        let cli = Cli {
            command: Commands::Ps {
                all: false,
                format: "table".to_string(),
                resources: false,
                status: None,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_ok(), "ps should succeed: {:?}", result.err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_execute_command_doctor() {
        let cli = Cli {
            command: Commands::Doctor {
                all: false,
                hardware: true,
                ecosystem: false,
                config: false,
                format: "text".to_string(),
                fix: false,
            },
            verbose: false,
            config: None,
            directory: None,
        };
        let ctx = CliContext::new(&cli).expect("context");

        let result = execute_command(&cli, &ctx).await;
        assert!(result.is_ok(), "doctor should succeed: {:?}", result.err());
    }

    #[test]
    fn test_dispatch_module_compiles_and_commands_accessible() {
        // Verify Commands enum is constructible and execute_command exists
        let _cmd = Commands::Doctor {
            all: false,
            hardware: false,
            ecosystem: false,
            config: false,
            format: "text".to_string(),
            fix: false,
        };
        // execute_command is the key public function - verify it's in scope
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
}
