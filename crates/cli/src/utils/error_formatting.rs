// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error formatting utilities with zero-copy optimizations
//!
//! Uses `Cow<'_, str>` for conditional allocation - only allocates when formatting is needed.

use std::borrow::Cow;
use toadstool_common::interned_strings::{capabilities, primals};

/// Get contextual error suggestion based on error content.
///
/// Returns `Cow::Borrowed` for static suggestions, `Cow::Owned` for dynamic ones.
///
/// Matches both capability names and legacy primal names in error strings for UX hints.
pub fn get_error_suggestion(error: &dyn std::error::Error) -> Option<Cow<'static, str>> {
    let error_str = error.to_string().to_lowercase();

    // File system errors - static suggestions (zero-copy)
    if error_str.contains("no such file or directory") {
        return Some(Cow::Borrowed(
            "💡 Check that the file path is correct and the file exists. Use 'ls' to verify.",
        ));
    }

    if error_str.contains("permission denied") {
        return Some(Cow::Borrowed(
            "💡 Try running with sudo or check file permissions with 'chmod' and 'chown'.",
        ));
    }

    // Network errors - static suggestions
    if error_str.contains("connection refused") {
        return Some(Cow::Borrowed(
            "💡 Check that the service is running and the address is correct. Use 'netstat -tlnp' to verify.",
        ));
    }

    if error_str.contains("connection timeout") {
        return Some(Cow::Borrowed(
            "💡 Check network connectivity and firewall settings. The service may be overloaded.",
        ));
    }

    // ToadStool specific errors
    if error_str.contains("biome.yaml") {
        return Some(Cow::Borrowed(
            "💡 Use 'toadstool init' to create a new biome.yaml file or 'toadstool validate' to check an existing one.",
        ));
    }

    if error_str.contains("not found") && !error_str.contains("file") {
        return Some(Cow::Borrowed(
            "💡 Use 'toadstool ps' to see available biomes or 'toadstool capabilities' to check platform support.",
        ));
    }

    if error_str.contains("already running") {
        return Some(Cow::Borrowed(
            "💡 Use 'toadstool down <biome>' to stop the existing instance or 'toadstool ps' to check status.",
        ));
    }

    if error_str.contains("insufficient resources") {
        return Some(Cow::Borrowed(
            "💡 Reduce resource requirements in biome.yaml or use 'toadstool resources' to check available capacity.",
        ));
    }

    if error_str.contains("security") {
        return Some(Cow::Borrowed(
            "💡 Check PKI security permissions with 'toadstool ecosystem auth --validate-only' and security policies.",
        ));
    }

    // Runtime errors
    if error_str.contains("wasm") {
        return Some(Cow::Borrowed(
            "💡 Verify WASM module is valid and all required dependencies are available.",
        ));
    }

    if error_str.contains("gpu") {
        return Some(Cow::Borrowed(
            "💡 Check GPU drivers and CUDA/OpenCL installation with 'nvidia-smi' or 'clinfo'.",
        ));
    }

    if error_str.contains("container") {
        return Some(Cow::Borrowed(
            "💡 Verify Docker/container runtime is installed and running. Check with 'docker version'.",
        ));
    }

    // Ecosystem errors (match both capability and legacy route strings)
    if error_str.contains(primals::LEGACY_COORDINATION_LABEL)
        || error_str.contains(capabilities::COORDINATION)
    {
        return Some(Cow::Borrowed(
            "💡 Use 'toadstool ecosystem discover' to find orchestration instances or check network connectivity.",
        ));
    }

    if error_str.contains(primals::LEGACY_STORAGE_LABEL)
        || error_str.contains(capabilities::STORAGE)
    {
        return Some(Cow::Borrowed(
            "💡 Verify storage endpoint and credentials. Use 'toadstool ecosystem storage --help' for options.",
        ));
    }

    if error_str.contains(primals::LEGACY_SECURITY_LABEL)
        || error_str.contains(capabilities::CRYPTO)
    {
        return Some(Cow::Borrowed(
            "💡 Install PKI security permissions with 'toadstool ecosystem auth <permission-file>'.",
        ));
    }

    // General suggestions
    if error_str.contains("parse") || error_str.contains("invalid") {
        return Some(Cow::Borrowed(
            "💡 Check syntax and format of configuration files. Use '--help' for command usage.",
        ));
    }

    if error_str.contains("timeout") {
        return Some(Cow::Borrowed(
            "💡 Increase timeout values or check system performance. Some operations may take longer on slower systems.",
        ));
    }

    None
}

/// Format platform summary with conditional allocation
///
/// Always allocates since we're formatting multiple strings
pub fn format_platform_summary(os: &str, arch: &str, version: &str) -> String {
    if version == "unknown" {
        format!("{os} {arch}")
    } else {
        format!("{os} {arch} ({version})")
    }
}

/// Format version display with fallback
///
/// Returns borrowed "unknown" or owned formatted version
pub fn format_version_display(version: Option<&str>) -> Cow<'static, str> {
    match version {
        Some(v) if !v.is_empty() => Cow::Owned(format!("v{v}")),
        _ => Cow::Borrowed("unknown"),
    }
}

/// Format resource display with units
///
/// Always allocates since we need to format numbers
pub fn format_resource_amount(amount: f64, unit: &str) -> String {
    if amount.fract() == 0.0 {
        // Integer value - simpler format
        format!("{}{}", amount as u64, unit)
    } else {
        // Decimal value - full precision
        format!("{amount:.2}{unit}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_suggestion_borrowed() {
        let error = crate::CliError::Other("No such file or directory".to_string());
        let suggestion = get_error_suggestion(&error);
        assert!(suggestion.is_some());

        // Verify it's borrowed (zero-copy)
        if let Some(Cow::Borrowed(_)) = suggestion {
            // Good - zero allocation
        } else {
            panic!("Expected Cow::Borrowed for static suggestion");
        }
    }

    #[test]
    fn test_format_version_display() {
        assert_eq!(format_version_display(Some("1.0.0")), "v1.0.0");
        assert_eq!(format_version_display(None), "unknown");
        assert_eq!(format_version_display(Some("")), "unknown");
    }

    #[test]
    fn test_format_resource_amount() {
        assert_eq!(format_resource_amount(4.0, "GB"), "4GB");
        assert_eq!(format_resource_amount(2.5, "GB"), "2.50GB");
    }

    #[test]
    fn test_format_platform_summary_with_version() {
        let s = format_platform_summary("linux", "x86_64", "5.10");
        assert_eq!(s, "linux x86_64 (5.10)");
    }

    #[test]
    fn test_format_platform_summary_unknown_version() {
        let s = format_platform_summary("linux", "x86_64", "unknown");
        assert_eq!(s, "linux x86_64");
    }

    #[test]
    fn test_error_suggestion_permission_denied() {
        let err = crate::CliError::Other("Permission denied".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("sudo"));
    }

    #[test]
    fn test_error_suggestion_connection_refused() {
        let err = crate::CliError::Other("connection refused".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("netstat"));
    }

    #[test]
    fn test_error_suggestion_connection_timeout() {
        let err = crate::CliError::Other("connection timeout".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("firewall"));
    }

    #[test]
    fn test_error_suggestion_biome_yaml() {
        let err = crate::CliError::Other("biome.yaml not found".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("toadstool init"));
    }

    #[test]
    fn test_error_suggestion_already_running() {
        let err = crate::CliError::Other("service already running".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("toadstool down"));
    }

    #[test]
    fn test_error_suggestion_insufficient_resources() {
        let err = crate::CliError::Other("insufficient resources".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("biome.yaml"));
    }

    #[test]
    fn test_error_suggestion_security() {
        let err = crate::CliError::Other("security violation".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("PKI"));
    }

    #[test]
    fn test_error_suggestion_wasm() {
        let err = crate::CliError::Other("wasm module invalid".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("WASM"));
    }

    #[test]
    fn test_error_suggestion_timeout() {
        let err = crate::CliError::Other("operation timeout".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("timeout"));
    }

    #[test]
    fn test_error_suggestion_parse_invalid() {
        let err = crate::CliError::Other("parse error: invalid format".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("syntax"));
    }

    #[test]
    fn test_error_suggestion_no_match() {
        let err = crate::CliError::Other("some random error".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_none());
    }

    #[test]
    fn test_error_suggestion_gpu() {
        let err = crate::CliError::Other("gpu initialization failed".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("nvidia-smi"));
    }

    #[test]
    fn test_error_suggestion_container() {
        let err = crate::CliError::Other("container runtime not available".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("Docker"));
    }

    #[test]
    fn test_error_suggestion_not_found_no_file() {
        // "not found" without "file" should match
        let err = crate::CliError::Other("biome not found".to_string());
        let suggestion = get_error_suggestion(&err);
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("toadstool ps"));
    }
}
