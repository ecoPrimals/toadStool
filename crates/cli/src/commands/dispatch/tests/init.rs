// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::commands::definitions::Commands;
use crate::commands::dispatch::execute_command;
use crate::{Cli, CliContext};
use tempfile::TempDir;

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
