// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::commands::definitions::{Commands, UniversalCommands};
use crate::commands::dispatch::execute_command;
use crate::{Cli, CliContext};

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
