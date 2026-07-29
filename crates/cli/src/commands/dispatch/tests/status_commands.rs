// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::commands::definitions::Commands;
use crate::commands::dispatch::execute_command;
use crate::{Cli, CliContext};

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
            check_config: false,
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
