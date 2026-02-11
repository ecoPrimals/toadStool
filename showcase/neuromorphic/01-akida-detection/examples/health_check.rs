//! Board health check and diagnostics example

use akida_detection_demo::{akida_device, detect_all_boards};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("Akida Health Check Report\n");
    println!("==========================\n");

    let mesh = detect_all_boards().await?;

    if mesh.boards.is_empty() {
        println!("No Akida boards detected.\n");
        return Ok(());
    }

    let mut all_healthy = true;

    for board in &mesh.boards {
        let location = board
            .node_name
            .as_ref()
            .map(|n| format!("{} - remote", n))
            .unwrap_or_else(|| "Local".to_string());

        println!("Board {} ({}):", board.index, location);

        // Run diagnostics
        let report = akida_device::run_diagnostics(board)?;

        for test in &report.tests {
            println!("  {} {}: {}", test.status.emoji(), test.name, test.details);
        }

        if !report.all_passed() {
            all_healthy = false;
        }

        println!();
    }

    println!(
        "Overall Status: {}",
        if all_healthy {
            "HEALTHY ✓"
        } else {
            "WARNING ⚠"
        }
    );

    if all_healthy {
        println!("All boards operational and ready for workload scheduling.");
    } else {
        println!("Some boards have warnings. Review diagnostic details above.");
    }

    Ok(())
}
