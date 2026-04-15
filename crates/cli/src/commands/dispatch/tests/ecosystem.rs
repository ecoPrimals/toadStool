// SPDX-License-Identifier: AGPL-3.0-or-later
use std::path::PathBuf;

use crate::commands::definitions::{Commands, EcosystemCommands};
use crate::commands::dispatch::execute_command;
use crate::{Cli, CliContext};

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
