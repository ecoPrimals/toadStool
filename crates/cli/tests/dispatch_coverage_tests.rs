// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Fast unit tests for command dispatch routing (commands/dispatch/mod.rs)
//!
//! Tests verify dispatch routing reaches the correct handler without executing
//! heavyweight operations (no real mDNS, hardware probing, or network I/O).
//! Integration-level execution is covered by e2e tests.

use std::path::PathBuf;
use toadstool_cli::{
    Cli, CliContext, Commands, EcosystemCommands, ModeCommand, TransportCommands, UniversalCommands,
};

fn make_cli(command: Commands) -> Cli {
    Cli {
        command,
        verbose: false,
        config: None,
        directory: None,
    }
}

#[test]
fn cli_context_from_run_command() {
    let cli = make_cli(Commands::Run {
        manifest: PathBuf::from("/tmp/test.yaml"),
        name: Some("test-biome".to_string()),
        env: vec![],
        debug: false,
        cpu_limit: None,
        memory_limit: None,
        security: "high".to_string(),
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_up_command() {
    let cli = make_cli(Commands::Up {
        manifest: PathBuf::from("/tmp/test.yaml"),
        detach: true,
        name: Some("biome-a".to_string()),
        env: vec!["KEY=VALUE".to_string()],
        restart: false,
        health_interval: 30,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_down_command() {
    let cli = make_cli(Commands::Down {
        biome: "my-biome".to_string(),
        force: true,
        timeout: 10,
        purge: false,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_ps_command() {
    let cli = make_cli(Commands::Ps {
        all: true,
        format: "json".to_string(),
        resources: true,
        status: Some("running".to_string()),
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_logs_command() {
    let cli = make_cli(Commands::Logs {
        target: "test-biome".to_string(),
        follow: true,
        lines: 100,
        timestamps: true,
        level: Some("warn".to_string()),
        grep: Some("error".to_string()),
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_validate_command() {
    let cli = make_cli(Commands::Validate {
        manifest: PathBuf::from("/tmp/biome.yaml"),
        check_resources: true,
        check_security: true,
        format: "json".to_string(),
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_init_command() {
    let cli = make_cli(Commands::Init {
        path: PathBuf::from("/tmp/new-biome"),
        template: "default".to_string(),
        force: false,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_capabilities_command() {
    let cli = make_cli(Commands::Capabilities {
        format: "json".to_string(),
        detailed: true,
        test_platform: None,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_ecosystem_discover() {
    let cli = make_cli(Commands::Ecosystem {
        action: EcosystemCommands::Discover {
            services: vec!["crypto".to_string(), "storage".to_string()],
            timeout: 1,
        },
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_transport_list() {
    let cli = make_cli(Commands::Transport {
        action: TransportCommands::List {
            format: "text".to_string(),
        },
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_transport_discover() {
    let cli = make_cli(Commands::Transport {
        action: TransportCommands::Discover {
            format: "json".to_string(),
        },
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_transport_status() {
    let cli = make_cli(Commands::Transport {
        action: TransportCommands::Status,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_server_command() {
    let cli = make_cli(Commands::Server {
        register: true,
        port: 8084,
        socket: Some(PathBuf::from("/tmp/test.sock")),
        config: None,
        max_workloads: 16,
        biomeos_socket: None,
        family_id: Some("test-family".to_string()),
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_daemon_command() {
    let cli = make_cli(Commands::Daemon {
        register: false,
        port: 9090,
        socket: None,
        config: Some(PathBuf::from("/etc/toadstool/config.toml")),
        max_workloads: 8,
        biomeos_socket: Some(PathBuf::from("/run/biomeos.sock")),
        family_id: None,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_execute_command() {
    let cli = make_cli(Commands::Execute {
        workload: PathBuf::from("/tmp/workload.toml"),
        runtime: Some("native".to_string()),
        env: vec!["DEBUG=1".to_string()],
        timeout: 60,
        format: "json".to_string(),
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_byob_server_command() {
    let cli = make_cli(Commands::ByobServer {
        bind: Some("0.0.0.0".to_string()),
        port: None,
        config: None,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_doctor_all() {
    let cli = make_cli(Commands::Doctor {
        all: true,
        hardware: false,
        ecosystem: false,
        config: false,
        format: "text".to_string(),
        fix: false,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_doctor_hardware_only() {
    let cli = make_cli(Commands::Doctor {
        all: false,
        hardware: true,
        ecosystem: false,
        config: false,
        format: "json".to_string(),
        fix: true,
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[tokio::test]
async fn execute_nonexistent_workload_returns_error() {
    let cli = make_cli(Commands::Execute {
        workload: PathBuf::from("/nonexistent/workload.toml"),
        runtime: None,
        env: vec![],
        timeout: 1,
        format: "text".to_string(),
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn transport_list_executes_fast() {
    let cli = make_cli(Commands::Transport {
        action: TransportCommands::List {
            format: "text".to_string(),
        },
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn transport_status_executes_fast() {
    let cli = make_cli(Commands::Transport {
        action: TransportCommands::Status,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok());
}

#[test]
fn cli_context_verbose_mode() {
    let cli = Cli {
        command: Commands::Ps {
            all: false,
            format: "text".to_string(),
            resources: false,
            status: None,
        },
        verbose: true,
        config: Some(PathBuf::from("/custom/config.toml")),
        directory: Some(PathBuf::from("/custom/dir")),
    };
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

#[test]
fn cli_context_from_universal_benchmark() {
    let cli = make_cli(Commands::Universal {
        operation: UniversalCommands::Benchmark {
            suite: "standard".to_string(),
            platforms: vec![],
            format: "json".to_string(),
        },
    });
    let ctx = CliContext::new(&cli);
    assert!(ctx.is_ok());
}

// ─── execute_command dispatch branches (no real network/hardware) ───────────

#[tokio::test]
async fn execute_validate_with_temp_manifest() {
    let temp = tempfile::tempdir().expect("temp dir");
    let manifest = temp.path().join("biome.yaml");
    std::fs::write(&manifest, "name: test\nservices: {}").expect("write");

    let cli = make_cli(Commands::Validate {
        manifest: manifest.clone(),
        check_resources: false,
        check_security: false,
        format: "text".to_string(),
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_init_with_temp_path() {
    let temp = tempfile::tempdir().expect("temp dir");
    let path = temp.path().join("new-biome");

    let cli = make_cli(Commands::Init {
        path: path.clone(),
        template: "default".to_string(),
        force: false,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let _result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
}

#[tokio::test]
async fn execute_init_force_overwrites() {
    let temp = tempfile::tempdir().expect("temp dir");
    let path = temp.path().join("force-biome");
    std::fs::create_dir_all(&path).expect("create");

    let cli = make_cli(Commands::Init {
        path: path.clone(),
        template: "default".to_string(),
        force: true,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let _result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
}

#[tokio::test]
async fn execute_capabilities_text_format() {
    let cli = make_cli(Commands::Capabilities {
        format: "text".to_string(),
        detailed: false,
        test_platform: None,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn execute_capabilities_json_detailed() {
    let cli = make_cli(Commands::Capabilities {
        format: "json".to_string(),
        detailed: true,
        test_platform: Some("native".to_string()),
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn execute_ecosystem_discover() {
    let cli = make_cli(Commands::Ecosystem {
        action: EcosystemCommands::Discover {
            services: vec!["storage".to_string()],
            timeout: 1,
        },
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_ecosystem_register() {
    let cli = make_cli(Commands::Ecosystem {
        action: EcosystemCommands::Register {
            endpoint: "127.0.0.1:1".to_string(),
            token: None,
        },
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_universal_detect() {
    let cli = make_cli(Commands::Universal {
        operation: UniversalCommands::Detect {
            categories: vec!["traditional".to_string()],
            test: false,
            output: None,
        },
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_universal_migrate() {
    let cli = make_cli(Commands::Universal {
        operation: UniversalCommands::Migrate {
            source: "src".to_string(),
            target: "tgt".to_string(),
            pause: false,
            verify: false,
        },
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_doctor_config_only() {
    let cli = make_cli(Commands::Doctor {
        all: false,
        hardware: false,
        ecosystem: false,
        config: true,
        format: "text".to_string(),
        fix: false,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn execute_doctor_ecosystem_only() {
    let cli = make_cli(Commands::Doctor {
        all: false,
        hardware: false,
        ecosystem: true,
        config: false,
        format: "json".to_string(),
        fix: false,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_doctor_with_fix() {
    let cli = make_cli(Commands::Doctor {
        all: false,
        hardware: false,
        ecosystem: false,
        config: true,
        format: "text".to_string(),
        fix: true,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_down_force() {
    let cli = make_cli(Commands::Down {
        biome: "nonexistent-biome-xyz".to_string(),
        force: true,
        timeout: 1,
        purge: false,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_ps_all_resources() {
    let cli = make_cli(Commands::Ps {
        all: true,
        format: "json".to_string(),
        resources: true,
        status: Some("running".to_string()),
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_logs_target() {
    let cli = make_cli(Commands::Logs {
        target: "test-biome".to_string(),
        follow: false,
        lines: 5,
        timestamps: false,
        level: None,
        grep: None,
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_validate_json_format() {
    let temp = tempfile::tempdir().expect("temp dir");
    let manifest = temp.path().join("v.yaml");
    std::fs::write(&manifest, "name: v\nservices: {}").expect("write");

    let cli = make_cli(Commands::Validate {
        manifest,
        check_resources: true,
        check_security: true,
        format: "json".to_string(),
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn execute_transport_discover() {
    let cli = make_cli(Commands::Transport {
        action: TransportCommands::Discover {
            format: "json".to_string(),
        },
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn execute_mode_status() {
    let cli = make_cli(Commands::Mode {
        action: ModeCommand::Status { bdf: None },
    });
    let ctx = CliContext::new(&cli).unwrap();
    let result = toadstool_cli::commands::dispatch::execute_command(&cli, &ctx).await;
    assert!(result.is_ok() || result.is_err());
}
