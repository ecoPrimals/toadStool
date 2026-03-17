// SPDX-License-Identifier: AGPL-3.0-only
//! Language runtime platforms
//!
//! Support for various programming language runtimes including systems languages,
//! memory-managed languages, functional languages, scripting languages, and more.

use serde::{Deserialize, Serialize};

/// Language runtime platforms
///
/// Represents various programming language runtimes and execution environments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum LanguageRuntime {
    // Systems languages
    Rust {
        version: String,
        target_triple: String,
        features: Vec<String>,
    },
    C {
        compiler: String,
        standard: String,
        optimizations: Vec<String>,
    },
    Cpp {
        compiler: String,
        standard: String,
        features: Vec<String>,
    },
    Go {
        version: String,
        goos: String,
        goarch: String,
    },
    Zig {
        version: String,
        target: String,
        mode: String,
    },

    // Memory-managed languages
    Java {
        version: String,
        vm: String,
        gc: String,
    },
    CSharp {
        version: String,
        runtime: String,
        framework: String,
    },
    Python {
        version: String,
        implementation: String,
        features: Vec<String>,
    },
    JavaScript {
        engine: String,
        version: String,
        features: Vec<String>,
    },
    Ruby {
        version: String,
        implementation: String,
    },
    Kotlin {
        version: String,
        target: String,
    },
    Scala {
        version: String,
        platform: String,
    },

    // Functional languages
    Haskell {
        compiler: String,
        version: String,
        extensions: Vec<String>,
    },
    OCaml {
        version: String,
        features: Vec<String>,
    },
    Erlang {
        version: String,
        otp_version: String,
    },
    Elixir {
        version: String,
        otp_version: String,
    },
    FSharp {
        version: String,
        runtime: String,
    },
    Lisp {
        dialect: String,
        implementation: String,
    },

    // Scripting languages
    Bash {
        version: String,
        features: Vec<String>,
    },
    PowerShell {
        version: String,
        platform: String,
    },
    Lua {
        version: String,
        features: Vec<String>,
    },
    Perl {
        version: String,
        features: Vec<String>,
    },

    // Domain-specific languages
    R {
        version: String,
        packages: Vec<String>,
    },
    Matlab {
        version: String,
        toolboxes: Vec<String>,
    },
    Mathematica {
        version: String,
        features: Vec<String>,
    },
    Julia {
        version: String,
        packages: Vec<String>,
    },

    // Emerging languages
    Mojo {
        version: String,
        features: Vec<String>,
    },
    Carbon {
        version: String,
        features: Vec<String>,
    },
    Gleam {
        version: String,
        target: String,
    },
    Crystal {
        version: String,
        features: Vec<String>,
    },

    // Assembly languages
    Assembly {
        architecture: String,
        assembler: String,
        format: String,
    },

    // Esoteric languages
    Brainfuck {
        interpreter: String,
    },
    Whitespace {
        interpreter: String,
    },
    Shakespeare {
        interpreter: String,
    },
}

impl LanguageRuntime {
    /// Get the language name
    pub const fn language_name(&self) -> &'static str {
        match self {
            Self::Rust { .. } => "Rust",
            Self::C { .. } => "C",
            Self::Cpp { .. } => "C++",
            Self::Go { .. } => "Go",
            Self::Zig { .. } => "Zig",
            Self::Java { .. } => "Java",
            Self::CSharp { .. } => "C#",
            Self::Python { .. } => "Python",
            Self::JavaScript { .. } => "JavaScript",
            Self::Ruby { .. } => "Ruby",
            Self::Kotlin { .. } => "Kotlin",
            Self::Scala { .. } => "Scala",
            Self::Haskell { .. } => "Haskell",
            Self::OCaml { .. } => "OCaml",
            Self::Erlang { .. } => "Erlang",
            Self::Elixir { .. } => "Elixir",
            Self::FSharp { .. } => "F#",
            Self::Lisp { .. } => "Lisp",
            Self::Bash { .. } => "Bash",
            Self::PowerShell { .. } => "PowerShell",
            Self::Lua { .. } => "Lua",
            Self::Perl { .. } => "Perl",
            Self::R { .. } => "R",
            Self::Matlab { .. } => "MATLAB",
            Self::Mathematica { .. } => "Mathematica",
            Self::Julia { .. } => "Julia",
            Self::Mojo { .. } => "Mojo",
            Self::Carbon { .. } => "Carbon",
            Self::Gleam { .. } => "Gleam",
            Self::Crystal { .. } => "Crystal",
            Self::Assembly { .. } => "Assembly",
            Self::Brainfuck { .. } => "Brainfuck",
            Self::Whitespace { .. } => "Whitespace",
            Self::Shakespeare { .. } => "Shakespeare",
        }
    }

    /// Check if language is a systems language
    pub const fn is_systems_language(&self) -> bool {
        matches!(
            self,
            Self::Rust { .. }
                | Self::C { .. }
                | Self::Cpp { .. }
                | Self::Go { .. }
                | Self::Zig { .. }
                | Self::Assembly { .. }
        )
    }

    /// Check if language is memory-managed
    pub const fn is_memory_managed(&self) -> bool {
        matches!(
            self,
            Self::Java { .. }
                | Self::CSharp { .. }
                | Self::Python { .. }
                | Self::JavaScript { .. }
                | Self::Ruby { .. }
                | Self::Kotlin { .. }
                | Self::Scala { .. }
        )
    }

    /// Check if language is functional
    pub const fn is_functional(&self) -> bool {
        matches!(
            self,
            Self::Haskell { .. }
                | Self::OCaml { .. }
                | Self::Erlang { .. }
                | Self::Elixir { .. }
                | Self::FSharp { .. }
                | Self::Lisp { .. }
        )
    }

    /// Check if language is a scripting language
    pub const fn is_scripting_language(&self) -> bool {
        matches!(
            self,
            Self::Bash { .. }
                | Self::PowerShell { .. }
                | Self::Lua { .. }
                | Self::Perl { .. }
                | Self::Python { .. }
        )
    }

    /// Check if language is domain-specific
    pub const fn is_domain_specific(&self) -> bool {
        matches!(
            self,
            Self::R { .. } | Self::Matlab { .. } | Self::Mathematica { .. } | Self::Julia { .. }
        )
    }

    /// Check if language is esoteric
    pub const fn is_esoteric(&self) -> bool {
        matches!(
            self,
            Self::Brainfuck { .. } | Self::Whitespace { .. } | Self::Shakespeare { .. }
        )
    }

    /// Check if language has strong type system
    pub const fn has_strong_typing(&self) -> bool {
        matches!(
            self,
            Self::Rust { .. }
                | Self::Haskell { .. }
                | Self::OCaml { .. }
                | Self::FSharp { .. }
                | Self::Scala { .. }
                | Self::Kotlin { .. }
        )
    }

    /// Check if suitable for high-performance computing
    pub const fn is_high_performance(&self) -> bool {
        matches!(
            self,
            Self::Rust { .. }
                | Self::C { .. }
                | Self::Cpp { .. }
                | Self::Zig { .. }
                | Self::Julia { .. }
                | Self::Assembly { .. }
        )
    }

    /// Check if suitable for concurrent/parallel programming
    pub const fn supports_concurrency(&self) -> bool {
        matches!(
            self,
            Self::Rust { .. }
                | Self::Go { .. }
                | Self::Erlang { .. }
                | Self::Elixir { .. }
                | Self::Java { .. }
                | Self::Scala { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systems_language() {
        let rust = LanguageRuntime::Rust {
            version: "1.75.0".to_string(),
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            features: vec!["std".to_string()],
        };

        assert_eq!(rust.language_name(), "Rust");
        assert!(rust.is_systems_language());
        assert!(rust.has_strong_typing());
        assert!(rust.is_high_performance());
    }

    #[test]
    fn test_functional_language() {
        let haskell = LanguageRuntime::Haskell {
            compiler: "GHC".to_string(),
            version: "9.4.7".to_string(),
            extensions: vec!["TypeFamilies".to_string()],
        };

        assert!(haskell.is_functional());
        assert!(haskell.has_strong_typing());
        assert!(!haskell.is_systems_language());
    }

    #[test]
    fn test_memory_managed() {
        let java = LanguageRuntime::Java {
            version: "21".to_string(),
            vm: "OpenJDK".to_string(),
            gc: "G1".to_string(),
        };

        assert!(java.is_memory_managed());
        assert!(!java.is_systems_language());
        assert!(java.supports_concurrency());
    }

    #[test]
    fn test_concurrent_language() {
        let erlang = LanguageRuntime::Erlang {
            version: "26.0".to_string(),
            otp_version: "26.0".to_string(),
        };

        assert!(erlang.supports_concurrency());
        assert!(erlang.is_functional());
    }

    #[test]
    fn test_esoteric() {
        let brainfuck = LanguageRuntime::Brainfuck {
            interpreter: "bf-interp".to_string(),
        };

        assert!(brainfuck.is_esoteric());
        assert!(!brainfuck.is_systems_language());
    }

    #[test]
    fn test_domain_specific() {
        let julia = LanguageRuntime::Julia {
            version: "1.9.0".to_string(),
            packages: vec!["DataFrames".to_string()],
        };

        assert!(julia.is_domain_specific());
        assert!(julia.is_high_performance());
    }

    #[test]
    fn test_serialization() {
        let runtime = LanguageRuntime::Python {
            version: "3.11".to_string(),
            implementation: "CPython".to_string(),
            features: vec!["asyncio".to_string()],
        };

        let json = serde_json::to_string(&runtime).unwrap();
        let deserialized: LanguageRuntime = serde_json::from_str(&json).unwrap();

        assert_eq!(runtime, deserialized);
    }
}
