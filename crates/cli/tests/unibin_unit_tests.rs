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
//! `UniBin` Architecture Unit Tests
//!
//! Comprehensive unit tests for ToadStool's `UniBin` implementation.
//! Tests cover server/daemon command handling, argument parsing, and mode detection.
//!
//! ToadStool is the FIRST primal to achieve 100% `UniBin` compliance!

use clap::Parser;
use std::path::PathBuf;
use toadstool_cli::{Cli, Commands};

// ============================================================================
// COMMAND PARSING TESTS
// ============================================================================

#[test]
fn test_server_command_basic() {
    // Test basic server command parsing
    let args = vec!["toadstool", "server"];
    let cli = Cli::parse_from(args);

    assert!(matches!(cli.command, Commands::Server { .. }));
}

#[test]
fn test_server_command_with_register() {
    // Test server command with register flag
    let args = vec!["toadstool", "server", "--register"];
    let cli = Cli::parse_from(args);

    if let Commands::Server { register, .. } = cli.command {
        assert!(register);
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_command_with_custom_port() {
    // Test server command with custom port
    let args = vec!["toadstool", "server", "--port", "9090"];
    let cli = Cli::parse_from(args);

    if let Commands::Server { port, .. } = cli.command {
        assert_eq!(port, 9090);
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_command_with_socket() {
    // Test server command with custom socket path
    let args = vec!["toadstool", "server", "--socket", "/tmp/toadstool.sock"];
    let cli = Cli::parse_from(args);

    if let Commands::Server { socket, .. } = cli.command {
        assert_eq!(socket, Some(PathBuf::from("/tmp/toadstool.sock")));
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_command_with_config() {
    // Test server command with config file
    let args = vec!["toadstool", "server", "--config", "/etc/toadstool.toml"];
    let cli = Cli::parse_from(args);

    if let Commands::Server { config, .. } = cli.command {
        assert_eq!(config, Some(PathBuf::from("/etc/toadstool.toml")));
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_command_with_max_workloads() {
    // Test server command with max workloads
    let args = vec!["toadstool", "server", "--max-workloads", "25"];
    let cli = Cli::parse_from(args);

    if let Commands::Server { max_workloads, .. } = cli.command {
        assert_eq!(max_workloads, 25);
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_command_with_biomeos_socket() {
    // Test server command with BiomeOS socket
    let args = vec![
        "toadstool",
        "server",
        "--biomeos-socket",
        "/tmp/biomeos.sock",
    ];
    let cli = Cli::parse_from(args);

    if let Commands::Server { biomeos_socket, .. } = cli.command {
        assert_eq!(biomeos_socket, Some(PathBuf::from("/tmp/biomeos.sock")));
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_command_all_options() {
    // Test server command with all options combined
    let args = vec![
        "toadstool",
        "server",
        "--register",
        "--port",
        "9090",
        "--socket",
        "/tmp/toadstool.sock",
        "--config",
        "/etc/toadstool.toml",
        "--max-workloads",
        "25",
        "--biomeos-socket",
        "/tmp/biomeos.sock",
    ];
    let cli = Cli::parse_from(args);

    if let Commands::Server {
        register,
        port,
        socket,
        config,
        max_workloads,
        biomeos_socket,
        family_id: _,
    } = cli.command
    {
        assert!(register);
        assert_eq!(port, 9090);
        assert_eq!(socket, Some(PathBuf::from("/tmp/toadstool.sock")));
        assert_eq!(config, Some(PathBuf::from("/etc/toadstool.toml")));
        assert_eq!(max_workloads, 25);
        assert_eq!(biomeos_socket, Some(PathBuf::from("/tmp/biomeos.sock")));
    } else {
        panic!("Expected Server command");
    }
}

// ============================================================================
// DAEMON ALIAS TESTS (Backward Compatibility)
// ============================================================================

#[test]
fn test_daemon_command_basic() {
    // Test daemon command (backward compat alias)
    let args = vec!["toadstool", "daemon"];
    let cli = Cli::parse_from(args);

    assert!(matches!(cli.command, Commands::Daemon { .. }));
}

#[test]
fn test_daemon_command_with_register() {
    // Test daemon command with register flag
    let args = vec!["toadstool", "daemon", "--register"];
    let cli = Cli::parse_from(args);

    if let Commands::Daemon { register, .. } = cli.command {
        assert!(register);
    } else {
        panic!("Expected Daemon command");
    }
}

#[test]
fn test_daemon_command_with_custom_port() {
    // Test daemon command with custom port
    let args = vec!["toadstool", "daemon", "--port", "9090"];
    let cli = Cli::parse_from(args);

    if let Commands::Daemon { port, .. } = cli.command {
        assert_eq!(port, 9090);
    } else {
        panic!("Expected Daemon command");
    }
}

#[test]
fn test_daemon_command_all_options() {
    // Test daemon command with all options (backward compat)
    let args = vec![
        "toadstool",
        "daemon",
        "--register",
        "--port",
        "9090",
        "--socket",
        "/tmp/toadstool.sock",
        "--config",
        "/etc/toadstool.toml",
        "--max-workloads",
        "25",
        "--biomeos-socket",
        "/tmp/biomeos.sock",
    ];
    let cli = Cli::parse_from(args);

    if let Commands::Daemon {
        register,
        port,
        socket,
        config,
        max_workloads,
        biomeos_socket,
        family_id: _,
    } = cli.command
    {
        assert!(register);
        assert_eq!(port, 9090);
        assert_eq!(socket, Some(PathBuf::from("/tmp/toadstool.sock")));
        assert_eq!(config, Some(PathBuf::from("/etc/toadstool.toml")));
        assert_eq!(max_workloads, 25);
        assert_eq!(biomeos_socket, Some(PathBuf::from("/tmp/biomeos.sock")));
    } else {
        panic!("Expected Daemon command");
    }
}

// ============================================================================
// DEFAULT VALUE TESTS
// ============================================================================

#[test]
fn test_server_default_port() {
    // Test default port value
    let args = vec!["toadstool", "server"];
    let cli = Cli::parse_from(args);

    if let Commands::Server { port, .. } = cli.command {
        assert_eq!(port, 0); // OS-assigned port (sovereignty: runtime discovery)
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_default_max_workloads() {
    // Test default max workloads value
    let args = vec!["toadstool", "server"];
    let cli = Cli::parse_from(args);

    if let Commands::Server { max_workloads, .. } = cli.command {
        assert_eq!(max_workloads, 10); // Default max workloads
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_default_register() {
    // Test default register value (false)
    let args = vec!["toadstool", "server"];
    let cli = Cli::parse_from(args);

    if let Commands::Server { register, .. } = cli.command {
        assert!(!register); // Default is false
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_server_default_optional_paths() {
    // Test default optional paths (None)
    let args = vec!["toadstool", "server"];
    let cli = Cli::parse_from(args);

    if let Commands::Server {
        socket,
        config,
        biomeos_socket,
        ..
    } = cli.command
    {
        assert_eq!(socket, None);
        assert_eq!(config, None);
        assert_eq!(biomeos_socket, None);
    } else {
        panic!("Expected Server command");
    }
}

// ============================================================================
// CLI COMMAND TESTS (Non-Server/Daemon)
// ============================================================================

#[test]
fn test_run_command_exists() {
    // Test that other CLI commands still work
    let args = vec!["toadstool", "run", "test.toml"];
    let cli = Cli::parse_from(args);

    assert!(matches!(cli.command, Commands::Run { .. }));
}

#[test]
fn test_up_command_exists() {
    // Test up command
    let args = vec!["toadstool", "up", "test.toml"];
    let cli = Cli::parse_from(args);

    assert!(matches!(cli.command, Commands::Up { .. }));
}

#[test]
fn test_down_command_exists() {
    // Test down command
    let args = vec!["toadstool", "down", "test-biome"];
    let cli = Cli::parse_from(args);

    assert!(matches!(cli.command, Commands::Down { .. }));
}

// ============================================================================
// UNIBIN COMPLIANCE TESTS
// ============================================================================

#[test]
fn test_single_binary_multiple_modes() {
    // Verify UniBin principle: one binary, multiple modes
    let server_args = vec!["toadstool", "server"];
    let cli_args = vec!["toadstool", "run", "test.toml"];
    let daemon_args = vec!["toadstool", "daemon"];

    let server_cli = Cli::parse_from(server_args);
    let cli_cli = Cli::parse_from(cli_args);
    let daemon_cli = Cli::parse_from(daemon_args);

    // All parse successfully from same binary
    assert!(matches!(server_cli.command, Commands::Server { .. }));
    assert!(matches!(cli_cli.command, Commands::Run { .. }));
    assert!(matches!(daemon_cli.command, Commands::Daemon { .. }));
}

#[test]
fn test_ecosystem_standard_naming() {
    // Verify 'server' is the ecosystem standard command name
    let args = vec!["toadstool", "server"];
    let cli = Cli::parse_from(args);

    // Server command exists and parses correctly
    assert!(matches!(cli.command, Commands::Server { .. }));
}

#[test]
fn test_backward_compatibility_preserved() {
    // Verify daemon alias works for backward compatibility
    let daemon_args = vec!["toadstool", "daemon", "--port", "9090"];
    let server_args = vec!["toadstool", "server", "--port", "9090"];

    let daemon_cli = Cli::parse_from(daemon_args);
    let server_cli = Cli::parse_from(server_args);

    // Both commands should parse and have same structure
    match (daemon_cli.command, server_cli.command) {
        (Commands::Daemon { port: d_port, .. }, Commands::Server { port: s_port, .. }) => {
            assert_eq!(d_port, s_port);
        }
        _ => panic!("Expected Daemon and Server commands"),
    }
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
fn test_invalid_port_rejected() {
    // Test that invalid port values are rejected
    let args = vec!["toadstool", "server", "--port", "99999"];
    let result = Cli::try_parse_from(args);

    // Should fail with out-of-range error
    assert!(result.is_err());
}

#[test]
fn test_missing_required_arguments() {
    // Test commands with missing required arguments fail
    let args = vec!["toadstool", "run"]; // Missing manifest path
    let result = Cli::try_parse_from(args);

    // Should fail with missing argument error
    assert!(result.is_err());
}

#[cfg(test)]
mod concurrent_tests {
    use super::*;
    use tokio::task;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_command_parsing() {
        // Test that command parsing is thread-safe
        let handles: Vec<_> = (0..100)
            .map(|i| {
                task::spawn(async move {
                    let port_str = i.to_string();
                    let args = vec!["toadstool", "server", "--port", &port_str];
                    let cli = Cli::parse_from(args);
                    matches!(cli.command, Commands::Server { .. })
                })
            })
            .collect();

        for handle in handles {
            assert!(handle.await.unwrap());
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_mode_detection() {
        // Test concurrent detection of server vs daemon modes
        let server_tasks: Vec<_> = (0..50)
            .map(|_| {
                task::spawn(async {
                    let args = vec!["toadstool", "server"];
                    let cli = Cli::parse_from(args);
                    matches!(cli.command, Commands::Server { .. })
                })
            })
            .collect();

        let daemon_tasks: Vec<_> = (0..50)
            .map(|_| {
                task::spawn(async {
                    let args = vec!["toadstool", "daemon"];
                    let cli = Cli::parse_from(args);
                    matches!(cli.command, Commands::Daemon { .. })
                })
            })
            .collect();

        for handle in server_tasks {
            assert!(handle.await.unwrap());
        }
        for handle in daemon_tasks {
            assert!(handle.await.unwrap());
        }
    }
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;

    #[test]
    fn test_port_range_property() {
        // Test that all valid ports (1024-65535) are accepted
        for port in [1024, 8080, 9090, 32768, 65535] {
            let port_str = port.to_string();
            let args = vec!["toadstool", "server", "--port", &port_str];
            let cli = Cli::parse_from(args);

            if let Commands::Server { port: p, .. } = cli.command {
                assert_eq!(p, port);
            } else {
                panic!("Expected Server command");
            }
        }
    }

    #[test]
    fn test_max_workloads_property() {
        // Test that reasonable max_workloads values are accepted
        for max in [1, 5, 10, 25, 50, 100] {
            let max_str = max.to_string();
            let args = vec!["toadstool", "server", "--max-workloads", &max_str];
            let cli = Cli::parse_from(args);

            if let Commands::Server { max_workloads, .. } = cli.command {
                assert_eq!(max_workloads, max);
            } else {
                panic!("Expected Server command");
            }
        }
    }
}
