//! Traditional computing platforms
//!
//! Support for conventional CPU architectures including x86_64, ARM64,
//! RISC-V, PowerPC, SPARC, and MIPS platforms.

use serde::{Deserialize, Serialize};

/// Traditional computing platforms
///
/// Represents mainstream CPU architectures with detailed capability information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum TraditionalPlatform {
    /// x86_64 (AMD64) architecture
    X86_64 {
        /// CPU model name
        cpu_model: String,
        /// Physical cores
        cores: u32,
        /// Hardware threads
        threads: u32,
        /// Cache size in MB
        cache_mb: u32,
        /// Total system memory in GB
        memory_gb: u32,
        /// CPU features (AVX, SSE, etc.)
        features: Vec<String>,
    },

    /// ARM64 (AArch64) architecture
    ARM64 {
        /// CPU model name
        cpu_model: String,
        /// Number of cores
        cores: u32,
        /// Uses big.LITTLE configuration
        big_little: bool,
        /// Total system memory in GB
        memory_gb: u32,
        /// ARM-specific features (NEON, SVE, etc.)
        features: Vec<String>,
    },

    /// RISC-V architecture
    RISCV {
        /// CPU model name
        cpu_model: String,
        /// Number of cores
        cores: u32,
        /// RISC-V extensions (M, A, F, D, C, etc.)
        extensions: Vec<String>,
        /// Total system memory in GB
        memory_gb: u32,
    },

    /// PowerPC architecture
    PowerPC {
        /// CPU model name
        cpu_model: String,
        /// Number of cores
        cores: u32,
        /// Total system memory in GB
        memory_gb: u32,
        /// PowerPC-specific features
        features: Vec<String>,
    },

    /// SPARC architecture
    SPARC {
        /// CPU model name
        cpu_model: String,
        /// Number of cores
        cores: u32,
        /// Total system memory in GB
        memory_gb: u32,
        /// SPARC-specific features
        features: Vec<String>,
    },

    /// MIPS architecture
    MIPS {
        /// CPU model name
        cpu_model: String,
        /// Number of cores
        cores: u32,
        /// Total system memory in GB
        memory_gb: u32,
        /// MIPS-specific features
        features: Vec<String>,
    },
}

impl TraditionalPlatform {
    /// Get the architecture name
    pub fn architecture_name(&self) -> &'static str {
        match self {
            Self::X86_64 { .. } => "x86_64",
            Self::ARM64 { .. } => "ARM64",
            Self::RISCV { .. } => "RISC-V",
            Self::PowerPC { .. } => "PowerPC",
            Self::SPARC { .. } => "SPARC",
            Self::MIPS { .. } => "MIPS",
        }
    }

    /// Get number of cores
    pub const fn cores(&self) -> u32 {
        match self {
            Self::X86_64 { cores, .. }
            | Self::ARM64 { cores, .. }
            | Self::RISCV { cores, .. }
            | Self::PowerPC { cores, .. }
            | Self::SPARC { cores, .. }
            | Self::MIPS { cores, .. } => *cores,
        }
    }

    /// Get total memory in GB
    pub const fn memory_gb(&self) -> u32 {
        match self {
            Self::X86_64 { memory_gb, .. }
            | Self::ARM64 { memory_gb, .. }
            | Self::RISCV { memory_gb, .. }
            | Self::PowerPC { memory_gb, .. }
            | Self::SPARC { memory_gb, .. }
            | Self::MIPS { memory_gb, .. } => *memory_gb,
        }
    }

    /// Check if platform supports specific feature
    pub fn has_feature(&self, feature: &str) -> bool {
        match self {
            Self::X86_64 { features, .. }
            | Self::ARM64 { features, .. }
            | Self::PowerPC { features, .. }
            | Self::SPARC { features, .. }
            | Self::MIPS { features, .. } => {
                features.iter().any(|f| f.eq_ignore_ascii_case(feature))
            }
            Self::RISCV { extensions, .. } => {
                extensions.iter().any(|e| e.eq_ignore_ascii_case(feature))
            }
        }
    }

    /// Check if this is a modern/high-performance platform
    pub const fn is_high_performance(&self) -> bool {
        match self {
            Self::X86_64 {
                cores, memory_gb, ..
            }
            | Self::ARM64 {
                cores, memory_gb, ..
            } => *cores >= 8 && *memory_gb >= 16,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_name() {
        let x86 = TraditionalPlatform::X86_64 {
            cpu_model: "Intel Core i7".to_string(),
            cores: 8,
            threads: 16,
            cache_mb: 16,
            memory_gb: 32,
            features: vec!["AVX2".to_string()],
        };

        assert_eq!(x86.architecture_name(), "x86_64");
        assert_eq!(x86.cores(), 8);
        assert_eq!(x86.memory_gb(), 32);
    }

    #[test]
    fn test_has_feature() {
        let x86 = TraditionalPlatform::X86_64 {
            cpu_model: "AMD Ryzen".to_string(),
            cores: 16,
            threads: 32,
            cache_mb: 64,
            memory_gb: 64,
            features: vec!["AVX2".to_string(), "SSE4.2".to_string()],
        };

        assert!(x86.has_feature("AVX2"));
        assert!(x86.has_feature("avx2")); // Case insensitive
        assert!(!x86.has_feature("AVX512"));
    }

    #[test]
    fn test_high_performance() {
        let high_perf = TraditionalPlatform::X86_64 {
            cpu_model: "High-end".to_string(),
            cores: 16,
            threads: 32,
            cache_mb: 64,
            memory_gb: 64,
            features: vec![],
        };

        let low_perf = TraditionalPlatform::X86_64 {
            cpu_model: "Budget".to_string(),
            cores: 4,
            threads: 4,
            cache_mb: 6,
            memory_gb: 8,
            features: vec![],
        };

        assert!(high_perf.is_high_performance());
        assert!(!low_perf.is_high_performance());
    }

    #[test]
    fn test_serialization() {
        let platform = TraditionalPlatform::ARM64 {
            cpu_model: "ARM Cortex-A78".to_string(),
            cores: 8,
            big_little: true,
            memory_gb: 16,
            features: vec!["NEON".to_string()],
        };

        let json = serde_json::to_string(&platform).unwrap();
        let deserialized: TraditionalPlatform = serde_json::from_str(&json).unwrap();

        assert_eq!(platform, deserialized);
    }
}
