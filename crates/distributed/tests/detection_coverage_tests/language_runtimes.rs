// SPDX-License-Identifier: AGPL-3.0-or-later
use super::common::serde_json_roundtrip;
use toadstool_distributed::substrate::LanguageRuntime;

#[test]
fn language_runtime_many_variants_roundtrip() {
    let samples = vec![
        LanguageRuntime::Rust {
            version: "1".into(),
            target_triple: "t".into(),
            features: vec![],
        },
        LanguageRuntime::Cpp {
            compiler: "c++".into(),
            standard: "20".into(),
            features: vec![],
        },
        LanguageRuntime::Zig {
            version: "1".into(),
            target: "t".into(),
            mode: "Release".into(),
        },
        LanguageRuntime::CSharp {
            version: "1".into(),
            runtime: "dotnet".into(),
            framework: "net8".into(),
        },
        LanguageRuntime::Ruby {
            version: "3".into(),
            implementation: "mri".into(),
        },
        LanguageRuntime::Kotlin {
            version: "1".into(),
            target: "jvm".into(),
        },
        LanguageRuntime::Scala {
            version: "1".into(),
            platform: "jvm".into(),
        },
        LanguageRuntime::OCaml {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Elixir {
            version: "1".into(),
            otp_version: "26".into(),
        },
        LanguageRuntime::FSharp {
            version: "1".into(),
            runtime: "dotnet".into(),
        },
        LanguageRuntime::Lisp {
            dialect: "common".into(),
            implementation: "sbcl".into(),
        },
        LanguageRuntime::PowerShell {
            version: "1".into(),
            platform: "core".into(),
        },
        LanguageRuntime::Lua {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Perl {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::R {
            version: "1".into(),
            packages: vec![],
        },
        LanguageRuntime::Matlab {
            version: "1".into(),
            toolboxes: vec![],
        },
        LanguageRuntime::Mathematica {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Julia {
            version: "1".into(),
            packages: vec![],
        },
        LanguageRuntime::Mojo {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Carbon {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Gleam {
            version: "1".into(),
            target: "erlang".into(),
        },
        LanguageRuntime::Crystal {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Assembly {
            architecture: "x86".into(),
            assembler: "nasm".into(),
            format: "elf".into(),
        },
        LanguageRuntime::Brainfuck {
            interpreter: "bf".into(),
        },
        LanguageRuntime::Shakespeare {
            interpreter: "s".into(),
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
        let _ = p.language_name();
    }
}

#[test]
fn language_runtime_c_go_python_java_js_erlang_haskell_roundtrip() {
    let samples = vec![
        LanguageRuntime::C {
            compiler: "gcc".into(),
            standard: "c17".into(),
            optimizations: vec!["O2".into()],
        },
        LanguageRuntime::Go {
            version: "1.22".into(),
            goos: "linux".into(),
            goarch: "amd64".into(),
        },
        LanguageRuntime::Python {
            version: "3.12".into(),
            implementation: "CPython".into(),
            features: vec![],
        },
        LanguageRuntime::Java {
            version: "21".into(),
            vm: "OpenJDK".into(),
            gc: "G1".into(),
        },
        LanguageRuntime::JavaScript {
            engine: "V8".into(),
            version: "20".into(),
            features: vec![],
        },
        LanguageRuntime::Erlang {
            version: "26".into(),
            otp_version: "26".into(),
        },
        LanguageRuntime::Haskell {
            compiler: "ghc".into(),
            version: "9.6".into(),
            extensions: vec![],
        },
        LanguageRuntime::Bash {
            version: "5".into(),
            features: vec![],
        },
    ];
    for p in samples {
        assert_eq!(p, serde_json_roundtrip(&p));
    }
}
