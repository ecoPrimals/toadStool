//! Execution result reporting and output formatting.
//!
//! Formats execution results for human-readable (text) or machine-readable (JSON) output.

use crate::Result;
use colored::Colorize;
use std::time::Duration;

use super::spec::WorkloadFile;

/// Print execution results in human-readable text format.
pub(super) fn print_text_output(
    workload: &WorkloadFile,
    response: &toadstool::ExecutionResponse,
    duration: Duration,
) -> Result<()> {
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_green()
    );
    println!("{}", "✅ Execution Complete!".bright_green().bold());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_green()
    );
    println!();
    println!("Workload:      {}", workload.metadata.name.bright_cyan());
    println!("Execution ID:  {}", response.execution_id);
    println!("Status:        {:?}", response.status);
    println!("Duration:      {:.3}s", duration.as_secs_f64());

    if let Some(exit_code) = response.output.exit_code {
        let status_str = if exit_code == 0 {
            format!("{}", exit_code).bright_green()
        } else {
            format!("{}", exit_code).bright_red()
        };
        println!("Exit Code:     {}", status_str);
    }

    println!();

    if let Some(stdout) = &response.output.stdout {
        if !stdout.is_empty() {
            println!("{}", "Standard Output:".bright_blue());
            println!("{}", "─".repeat(60));
            println!("{}", stdout);
            println!("{}", "─".repeat(60));
            println!();
        }
    }

    if let Some(stderr) = &response.output.stderr {
        if !stderr.is_empty() {
            println!("{}", "Standard Error:".bright_yellow());
            println!("{}", "─".repeat(60));
            println!("{}", stderr);
            println!("{}", "─".repeat(60));
            println!();
        }
    }

    Ok(())
}

/// Print execution results in JSON format.
pub(super) fn print_json_output(
    response: &toadstool::ExecutionResponse,
    duration: Duration,
) -> Result<()> {
    let output = serde_json::json!({
        "execution_id": response.execution_id,
        "status": format!("{:?}", response.status),
        "duration_secs": duration.as_secs_f64(),
        "exit_code": response.output.exit_code,
        "stdout": response.output.stdout,
        "stderr": response.output.stderr,
        "duration_internal": format!("{:?}", response.duration),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
