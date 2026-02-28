//! CLI initialization and setup
//!
//! Logging, banner, error handling, and display helpers.

use std::io::IsTerminal;

use colored::Colorize;

use crate::Result;

/// Exit codes following ecoBin standard
///
/// **ecoBin Compliance**: Standard exit codes for consistent system integration
pub mod exit_codes {
    /// General error - unspecified failure
    pub const GENERAL_ERROR: i32 = 1;
    /// Configuration error - invalid config, missing required settings
    pub const CONFIG_ERROR: i32 = 2;
    /// Runtime/network error - connection failures, resource exhaustion
    pub const RUNTIME_ERROR: i32 = 3;
    /// Interrupted - SIGINT/SIGTERM (Ctrl+C), ecoBin standard
    pub const INTERRUPTED: i32 = 130;
}

/// Determine appropriate exit code from error
pub fn exit_code_for_error(error: &dyn std::error::Error) -> i32 {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("config")
        || error_str.contains("manifest")
        || error_str.contains("invalid")
        || error_str.contains("missing")
        || error_str.contains("not found")
        || error_str.contains("parse")
    {
        return exit_codes::CONFIG_ERROR;
    }

    if error_str.contains("connection")
        || error_str.contains("network")
        || error_str.contains("timeout")
        || error_str.contains("refused")
        || error_str.contains("resource")
        || error_str.contains("exhausted")
        || error_str.contains("memory")
    {
        return exit_codes::RUNTIME_ERROR;
    }

    exit_codes::GENERAL_ERROR
}

/// Initialize enhanced logging with better formatting
pub fn init_enhanced_logging(verbose: bool) -> Result<()> {
    use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(tracing_subscriber::fmt::time::SystemTime)
        .with_ansi(std::io::stderr().is_terminal());

    if std::env::var("CI").is_ok() || !std::io::stderr().is_terminal() {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    Ok(())
}

/// Print the ToadStool banner
pub fn print_banner() {
    let banner = r#"
🍄 ████████╗ ██████╗  █████╗ ██████╗ ███████╗████████╗ ██████╗  ██████╗ ██╗     
🍄 ╚══██╔══╝██╔═══██╗██╔══██╗██╔══██╗██╔════╝╚══██╔══╝██╔═══██╗██╔═══██╗██║     
🍄    ██║   ██║   ██║███████║██║  ██║███████╗   ██║   ██║   ██║██║   ██║██║     
🍄    ██║   ██║   ██║██╔══██║██║  ██║╚════██║   ██║   ██║   ██║██║   ██║██║     
🍄    ██║   ╚██████╔╝██║  ██║██████╔╝███████║   ██║   ╚██████╔╝╚██████╔╝███████╗
🍄    ╚═╝    ╚═════╝ ╚═╝  ╚═╝╚═════╝ ╚══════╝   ╚═╝    ╚═════╝  ╚═════╝ ╚══════╝
"#;

    println!("{}", banner.bright_green().bold());
    println!(
        "{}",
        "Universal Compute Platform - The Backbone of SOVEREIGN SCIENCE"
            .bright_cyan()
            .bold()
    );
    println!(
        "{}",
        "Version 0.1.0 | If it has a chip and memory, ToadStool runs on it".bright_white()
    );
    println!();
}

/// Get error suggestion based on error content
///
/// Uses zero-copy optimized implementation from utils::error_formatting
/// with Cow<'_, str> for efficient string handling.
fn get_error_suggestion(error: &dyn std::error::Error) -> Option<String> {
    crate::utils::error_formatting::get_error_suggestion(error)
        .map(|cow: std::borrow::Cow<'_, str>| cow.into_owned())
}

/// Enhanced error reporting with context
pub fn print_enhanced_error(error: &dyn std::error::Error) {
    eprintln!("\n{}", "💥 ERROR".red().bold());
    eprintln!("{}", "═".repeat(60).red());

    eprintln!("{} {}", "Message:".red().bold(), error);

    let mut chain_len = 0;
    let mut current: Option<&dyn std::error::Error> = Some(error);
    while current.is_some() {
        chain_len += 1;
        current = current.and_then(|e| e.source());
    }
    if chain_len > 1 {
        eprintln!("\n{}", "📋 Error Chain:".yellow().bold());
        let mut i = 0;
        let mut current: Option<&dyn std::error::Error> = Some(error);
        while let Some(err) = current {
            if i > 0 {
                eprintln!("  {} {}", format!("{i}.").cyan(), err);
            }
            i += 1;
            current = err.source();
        }
    }

    if let Some(suggestion) = get_error_suggestion(error) {
        eprintln!("\n{}", suggestion.green());
    }

    eprintln!("\n{}", "📚 Need Help?".blue().bold());
    eprintln!("  {} toadstool --help", "•".blue());
    eprintln!("  {} toadstool <command> --help", "•".blue());
    eprintln!("  {} https://docs.toadstool.dev", "•".blue());
    eprintln!();
}

/// Enhanced success reporting
pub fn print_success_message(message: &str) {
    println!("\n{} {}", "✅".green().bold(), message.green().bold());
}

/// Print operation summary
pub fn print_operation_summary(
    operation: &str,
    duration: std::time::Duration,
    details: Option<&str>,
) {
    println!("\n{}", "📊 Operation Summary".blue().bold());
    println!("{}", "─".repeat(40).blue());
    println!("Operation: {}", operation.cyan());
    println!("Duration:  {:.2}s", duration.as_secs_f64());
    if let Some(details) = details {
        println!("Details:   {details}");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_for_error_config() {
        #[derive(Debug)]
        struct ConfigError;
        impl std::fmt::Display for ConfigError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "invalid config")
            }
        }
        impl std::error::Error for ConfigError {}

        assert_eq!(exit_code_for_error(&ConfigError), exit_codes::CONFIG_ERROR);
    }

    #[test]
    fn test_exit_code_for_error_manifest() {
        #[derive(Debug)]
        struct ManifestError;
        impl std::fmt::Display for ManifestError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "manifest not found")
            }
        }
        impl std::error::Error for ManifestError {}

        assert_eq!(
            exit_code_for_error(&ManifestError),
            exit_codes::CONFIG_ERROR
        );
    }

    #[test]
    fn test_exit_code_for_error_runtime() {
        #[derive(Debug)]
        struct RuntimeError;
        impl std::fmt::Display for RuntimeError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "connection refused")
            }
        }
        impl std::error::Error for RuntimeError {}

        assert_eq!(
            exit_code_for_error(&RuntimeError),
            exit_codes::RUNTIME_ERROR
        );
    }

    #[test]
    fn test_exit_code_for_error_network() {
        #[derive(Debug)]
        struct NetworkError;
        impl std::fmt::Display for NetworkError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "network timeout")
            }
        }
        impl std::error::Error for NetworkError {}

        assert_eq!(
            exit_code_for_error(&NetworkError),
            exit_codes::RUNTIME_ERROR
        );
    }

    #[test]
    fn test_exit_code_for_error_general() {
        #[derive(Debug)]
        struct GeneralError;
        impl std::fmt::Display for GeneralError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "something went wrong")
            }
        }
        impl std::error::Error for GeneralError {}

        assert_eq!(
            exit_code_for_error(&GeneralError),
            exit_codes::GENERAL_ERROR
        );
    }

    #[test]
    fn test_print_banner_no_panic() {
        print_banner();
    }
}
