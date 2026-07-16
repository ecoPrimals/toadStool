// SPDX-License-Identifier: AGPL-3.0-or-later
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
    /// Rust systems language runtime.
    Rust {
        /// Rust version (e.g. 1.75).
        version: String,
        /// Target triple (e.g. x86_64-unknown-linux-gnu).
        target_triple: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// C language runtime.
    C {
        /// Compiler (e.g. gcc, clang).
        compiler: String,
        /// C standard (e.g. c17).
        standard: String,
        /// Optimization flags.
        optimizations: Vec<String>,
    },
    /// C++ language runtime.
    Cpp {
        /// Compiler (e.g. g++, clang++).
        compiler: String,
        /// C++ standard (e.g. c++20).
        standard: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Go language runtime.
    Go {
        /// Go version.
        version: String,
        /// GOOS (e.g. linux).
        goos: String,
        /// GOARCH (e.g. amd64).
        goarch: String,
    },
    /// Zig language runtime.
    Zig {
        /// Zig version.
        version: String,
        /// Target architecture.
        target: String,
        /// Build mode (Debug, ReleaseSafe, etc.).
        mode: String,
    },
    /// Java memory-managed runtime.
    Java {
        /// Java version.
        version: String,
        /// JVM (e.g. openjdk, graalvm).
        vm: String,
        /// Garbage collector (e.g. g1, zgc).
        gc: String,
    },
    /// C# runtime.
    CSharp {
        /// .NET version.
        version: String,
        /// Runtime (e.g. dotnet).
        runtime: String,
        /// Framework (e.g. net8.0).
        framework: String,
    },
    /// Python interpreter runtime.
    Python {
        /// Python version.
        version: String,
        /// Implementation (cpython, pypy, etc.).
        implementation: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// JavaScript/Node runtime.
    JavaScript {
        /// Engine (v8, spidermonkey, etc.).
        engine: String,
        /// Version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Ruby interpreter.
    Ruby {
        /// Ruby version.
        version: String,
        /// Implementation (mri, jruby, etc.).
        implementation: String,
    },
    /// Kotlin JVM language.
    Kotlin {
        /// Kotlin version.
        version: String,
        /// Target (jvm, js, native).
        target: String,
    },
    /// Scala JVM language.
    Scala {
        /// Scala version.
        version: String,
        /// Platform (jvm, js, native).
        platform: String,
    },
    /// Haskell functional language.
    Haskell {
        /// Compiler (ghc, etc.).
        compiler: String,
        /// Version.
        version: String,
        /// Language extensions.
        extensions: Vec<String>,
    },
    /// OCaml functional language.
    OCaml {
        /// OCaml version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Erlang/OTP runtime.
    Erlang {
        /// Erlang version.
        version: String,
        /// OTP version.
        otp_version: String,
    },
    /// Elixir on BEAM.
    Elixir {
        /// Elixir version.
        version: String,
        /// OTP version.
        otp_version: String,
    },
    /// F# functional language.
    FSharp {
        /// F# version.
        version: String,
        /// .NET runtime.
        runtime: String,
    },
    /// Lisp dialect.
    Lisp {
        /// Dialect (common-lisp, scheme, etc.).
        dialect: String,
        /// Implementation (sbcl, racket, etc.).
        implementation: String,
    },
    /// Bash shell scripting.
    Bash {
        /// Bash version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// PowerShell scripting.
    PowerShell {
        /// PowerShell version.
        version: String,
        /// Platform (core, windows).
        platform: String,
    },
    /// Lua scripting language.
    Lua {
        /// Lua version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Perl scripting.
    Perl {
        /// Perl version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// R statistical language.
    R {
        /// R version.
        version: String,
        /// Installed packages.
        packages: Vec<String>,
    },
    /// MATLAB numerical computing.
    Matlab {
        /// MATLAB version.
        version: String,
        /// Installed toolboxes.
        toolboxes: Vec<String>,
    },
    /// Mathematica symbolic computing.
    Mathematica {
        /// Mathematica version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Julia scientific computing.
    Julia {
        /// Julia version.
        version: String,
        /// Installed packages.
        packages: Vec<String>,
    },
    /// Mojo AI language.
    Mojo {
        /// Mojo version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Carbon experimental C++ successor.
    Carbon {
        /// Carbon version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Gleam functional language.
    Gleam {
        /// Gleam version.
        version: String,
        /// Target (erlang, javascript).
        target: String,
    },
    /// Crystal compiled Ruby-like language.
    Crystal {
        /// Crystal version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Assembly language.
    Assembly {
        /// Target architecture.
        architecture: String,
        /// Assembler (nasm, gas, etc.).
        assembler: String,
        /// Output format (elf, mach-o, etc.).
        format: String,
    },
    /// Brainfuck esoteric language.
    Brainfuck {
        /// Interpreter identifier.
        interpreter: String,
    },
    /// Whitespace esoteric language.
    Whitespace {
        /// Interpreter identifier.
        interpreter: String,
    },
    /// Shakespeare esoteric language.
    Shakespeare {
        /// Interpreter identifier.
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
