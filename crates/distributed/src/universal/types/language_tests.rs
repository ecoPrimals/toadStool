// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from language.rs (S336).

use super::language::*;

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
