//! Error formatting utilities with zero-copy optimizations
//!
//! Uses `Cow<'_, str>` for conditional allocation - only allocates when formatting is needed.

use std::borrow::Cow;

/// Get contextual error suggestion based on error content
///
/// Returns `Cow::Borrowed` for static suggestions, `Cow::Owned` for dynamic ones.
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
            "💡 Check BearDog permissions with 'toadstool ecosystem auth --validate-only' and security policies.",
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

    // Ecosystem errors
    if error_str.contains("songbird") {
        return Some(Cow::Borrowed(
            "💡 Use 'toadstool ecosystem discover' to find Songbird instances or check network connectivity.",
        ));
    }

    if error_str.contains("nestgate") {
        return Some(Cow::Borrowed(
            "💡 Verify NestGate endpoint and credentials. Use 'toadstool ecosystem storage --help' for options.",
        ));
    }

    if error_str.contains("beardog") {
        return Some(Cow::Borrowed(
            "💡 Install BearDog permissions with 'toadstool ecosystem auth <permission-file>'.",
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
}
