// SPDX-License-Identifier: AGPL-3.0-or-later
//! Transport subcommand implementations
//!
//! Local hardware transport discovery (display, capture, serial) without daemon.

use toadstool_core::{TransportInfo, TransportMedium};
use toadstool_display::{
    discover_capture_transports, discover_display_transports, discover_pcie_transports,
    serial_transport::discover_serial_transports,
};

use crate::{CliError, Result};

/// Execute transport subcommands (local discovery, no daemon).
pub async fn execute_transport_command(
    action: &super::definitions::TransportCommands,
) -> Result<()> {
    match action {
        super::definitions::TransportCommands::Discover { format } => {
            run_discover(format).await?;
        }
        super::definitions::TransportCommands::List { format } => {
            // Same as discover for now (local discovery)
            run_discover(format).await?;
        }
        super::definitions::TransportCommands::Status => {
            run_status().await?;
        }
    }
    Ok(())
}

async fn run_discover(format: &str) -> Result<()> {
    let transports = discover_all_transports();

    match format.to_lowercase().as_str() {
        "json" => {
            let json_transports: Vec<serde_json::Value> = transports
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "label": t.label,
                        "medium": t.medium.to_string(),
                        "direction": t.direction.to_string(),
                    })
                })
                .collect();
            let json =
                serde_json::to_string_pretty(&json_transports).map_err(CliError::Serialization)?;
            println!("{json}");
        }
        _ => {
            print_transports_table(&transports);
        }
    }

    Ok(())
}

async fn run_status() -> Result<()> {
    let transports = discover_all_transports();

    let display_count = transports
        .iter()
        .filter(|t| t.medium == TransportMedium::Display)
        .count();
    let capture_count = transports
        .iter()
        .filter(|t| t.medium == TransportMedium::Capture)
        .count();
    let serial_count = transports
        .iter()
        .filter(|t| t.medium == TransportMedium::Serial)
        .count();
    let pcie_count = transports
        .iter()
        .filter(|t| t.medium == TransportMedium::Pcie)
        .count();

    println!("Transport Layer Status");
    println!("═══════════════════════════════════════════════════");
    println!("  Display outputs:  {display_count}");
    println!("  Capture devices:  {capture_count}");
    println!("  Serial ports:     {serial_count}");
    println!("  PCIe links:       {pcie_count}");
    println!("  Total:            {} transports", transports.len());

    Ok(())
}

fn discover_all_transports() -> Vec<TransportInfo> {
    let mut transports = Vec::new();
    transports.extend(discover_display_transports());
    transports.extend(discover_capture_transports());
    transports.extend(discover_serial_transports());
    transports.extend(discover_pcie_transports());
    transports
}

fn print_transports_table(transports: &[TransportInfo]) {
    println!("Hardware Transports");
    println!("═══════════════════════════════════════════════════");
    println!("{:<30} {:<10} {:<10} Label", "ID", "Medium", "Direction");

    for t in transports {
        println!(
            "{:<30} {:<10} {:<10} {}",
            t.id,
            t.medium.to_string(),
            t.direction.to_string(),
            t.label
        );
    }

    let display_count = transports
        .iter()
        .filter(|t| t.medium == TransportMedium::Display)
        .count();
    let capture_count = transports
        .iter()
        .filter(|t| t.medium == TransportMedium::Capture)
        .count();
    let serial_count = transports
        .iter()
        .filter(|t| t.medium == TransportMedium::Serial)
        .count();

    println!();
    println!(
        "Found: {} transports ({} display, {} capture, {} serial)",
        transports.len(),
        display_count,
        capture_count,
        serial_count
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::definitions::TransportCommands;

    #[tokio::test]
    async fn test_execute_transport_discover_json() {
        let cmd = TransportCommands::Discover {
            format: "json".to_string(),
        };
        let result = execute_transport_command(&cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_transport_discover_text() {
        let cmd = TransportCommands::Discover {
            format: "text".to_string(),
        };
        let result = execute_transport_command(&cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_transport_list() {
        let cmd = TransportCommands::List {
            format: "text".to_string(),
        };
        let result = execute_transport_command(&cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_transport_status() {
        let cmd = TransportCommands::Status;
        let result = execute_transport_command(&cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_transport_discover_unknown_format_defaults_to_table() {
        let cmd = TransportCommands::Discover {
            format: "unknown_format_xyz".to_string(),
        };
        let result = execute_transport_command(&cmd).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_discover_all_transports_returns_vec() {
        let transports = discover_all_transports();
        assert!(transports.iter().all(|t| !t.id.is_empty()));
    }

    #[test]
    fn test_print_transports_table_empty() {
        let transports: Vec<toadstool_core::TransportInfo> = vec![];
        print_transports_table(&transports);
    }

    #[test]
    fn test_print_transports_table_with_transports() {
        use toadstool_core::{TransportDirection, TransportInfo, TransportMedium};
        let transports = vec![TransportInfo {
            id: "test-1".to_string(),
            label: "Test Display".to_string(),
            medium: TransportMedium::Display,
            direction: TransportDirection::Tx,
        }];
        print_transports_table(&transports);
    }
}
