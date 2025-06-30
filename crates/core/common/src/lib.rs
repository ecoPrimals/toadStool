//! Common utilities and types for ToadStool
//!
//! This crate provides shared types, utilities, and constants used across
//! all ToadStool components.


use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generate a new unique identifier
pub fn generate_id() -> Uuid {
    Uuid::new_v4()
}

/// Platform information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Operating system
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// Total memory in bytes
    pub total_memory: u64,
    /// Available memory in bytes
    pub available_memory: u64,
}

impl PlatformInfo {
    /// Get current platform information
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_cores: num_cpus::get(),
            total_memory: 0, // TODO: Implement with sysinfo
            available_memory: 0, // TODO: Implement with sysinfo
        }
    }
}

/// Common result type with string error
pub type CommonResult<T> = Result<T, String>;

/// Format bytes as human readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Format duration as human readable string
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    
    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{}m{}s", minutes, seconds)
    } else {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        format!("{}h{}m{}s", hours, minutes, seconds)
    }
}

/// Validation trait for types that can be validated
pub trait Validate {
    type Error;
    
    /// Validate the object
    fn validate(&self) -> Result<(), Self::Error>;
}

/// String extensions
pub trait StringExt {
    /// Check if string is empty or contains only whitespace
    fn is_blank(&self) -> bool;
    
    /// Truncate string to maximum length
    fn truncate_to(&self, max_len: usize) -> String;
}

impl StringExt for str {
    fn is_blank(&self) -> bool {
        self.trim().is_empty()
    }
    
    fn truncate_to(&self, max_len: usize) -> String {
        if self.len() <= max_len {
            self.to_string()
        } else {
            format!("{}...", &self[..max_len.saturating_sub(3)])
        }
    }
}

impl StringExt for String {
    fn is_blank(&self) -> bool {
        self.as_str().is_blank()
    }
    
    fn truncate_to(&self, max_len: usize) -> String {
        self.as_str().truncate_to(max_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(std::time::Duration::from_secs(90)), "1m30s");
        assert_eq!(format_duration(std::time::Duration::from_secs(3661)), "1h1m1s");
    }

    #[test]
    fn test_string_extensions() {
        assert!("".is_blank());
        assert!("   ".is_blank());
        assert!(!"hello".is_blank());
        
        assert_eq!("hello".truncate_to(10), "hello");
        assert_eq!("hello world".truncate_to(8), "hello...");
    }

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
    }
} 