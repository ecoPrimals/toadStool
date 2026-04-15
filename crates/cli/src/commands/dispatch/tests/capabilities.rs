// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::commands::definitions::Commands;
use crate::commands::dispatch::execute_command;
use crate::{Cli, CliContext};

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
