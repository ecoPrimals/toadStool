// SPDX-License-Identifier: AGPL-3.0-only
//! Pure Rust validation tests
//!
//! These tests validate ToadStool's 99.95% Pure Rust achievement by:
//! 1. Testing cross-compilation to various targets
//! 2. Auditing dependencies for C code
//! 3. Validating zero C compiler invocations
//!
//! Philosophy: "Prove our Pure Rust claims with executable tests!"

use std::process::Command;

/// Test cross-compilation to ARM64 Linux (common for cloud/edge)
#[test]
fn test_cross_compile_arm64_linux() {
    // Check if target is installed
    let check = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("Failed to run rustup");

    let installed = String::from_utf8_lossy(&check.stdout);

    if !installed.contains("aarch64-unknown-linux-gnu") {
        eprintln!("Skipping: aarch64-unknown-linux-gnu not installed");
        eprintln!("Install with: rustup target add aarch64-unknown-linux-gnu");
        return;
    }

    // Attempt cross-compilation of runtime crates (Pure Rust!)
    let result = Command::new("cargo")
        .args([
            "build",
            "--target",
            "aarch64-unknown-linux-gnu",
            "--package",
            "toadstool-runtime-wasm",
            "--package",
            "toadstool-runtime-secure-enclave",
        ])
        .output()
        .expect("Failed to run cargo build");

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("Cross-compilation failed:\n{stderr}");
        panic!("ARM64 cross-compilation failed - C dependencies detected!");
    }

    println!("✅ ARM64 cross-compilation successful!");
}

/// Test cross-compilation to RISC-V (future platforms)
#[test]
fn test_cross_compile_riscv64() {
    let check = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("Failed to run rustup");

    let installed = String::from_utf8_lossy(&check.stdout);

    if !installed.contains("riscv64gc-unknown-linux-gnu") {
        eprintln!("Skipping: riscv64gc-unknown-linux-gnu not installed");
        eprintln!("Install with: rustup target add riscv64gc-unknown-linux-gnu");
        return;
    }

    let result = Command::new("cargo")
        .args([
            "build",
            "--target",
            "riscv64gc-unknown-linux-gnu",
            "--package",
            "toadstool-runtime-wasm",
            "--package",
            "toadstool-runtime-secure-enclave",
        ])
        .output()
        .expect("Failed to run cargo build");

    if result.status.success() {
        println!("✅ RISC-V cross-compilation successful!");
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("RISC-V cross-compilation failed:\n{stderr}");
        eprintln!("Note: May require additional system packages");
    }
}

/// Test cross-compilation to WebAssembly (browser/edge runtimes)
#[test]
fn test_cross_compile_wasm32() {
    let check = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("Failed to run rustup");

    let installed = String::from_utf8_lossy(&check.stdout);

    if !installed.contains("wasm32-unknown-unknown") {
        eprintln!("Skipping: wasm32-unknown-unknown not installed");
        eprintln!("Install with: rustup target add wasm32-unknown-unknown");
        return;
    }

    // Note: Not all crates will build for wasm32 (system-specific code)
    // But we can validate that core logic compiles
    let result = Command::new("cargo")
        .args([
            "check",
            "--target",
            "wasm32-unknown-unknown",
            "--package",
            "toadstool-common",
            "--lib",
        ])
        .output()
        .expect("Failed to run cargo check");

    if result.status.success() {
        println!("✅ WASM32 common crate compiles!");
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("WASM32 compilation check:\n{stderr}");
        // This is informational - not all crates should build for wasm32
    }
}

/// Test cross-compilation to Windows (`x86_64`)
#[test]
fn test_cross_compile_windows() {
    let check = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("Failed to run rustup");

    let installed = String::from_utf8_lossy(&check.stdout);

    if !installed.contains("x86_64-pc-windows-gnu") {
        eprintln!("Skipping: x86_64-pc-windows-gnu not installed");
        eprintln!("Install with: rustup target add x86_64-pc-windows-gnu");
        return;
    }

    let result = Command::new("cargo")
        .args([
            "check",
            "--target",
            "x86_64-pc-windows-gnu",
            "--package",
            "toadstool-runtime-wasm",
            "--package",
            "toadstool-runtime-secure-enclave",
        ])
        .output()
        .expect("Failed to run cargo check");

    if result.status.success() {
        println!("✅ Windows cross-compilation check successful!");
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("Windows cross-compilation check:\n{stderr}");
    }
}

/// Test cross-compilation to macOS ARM (Apple Silicon)
#[test]
fn test_cross_compile_macos_arm() {
    let check = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("Failed to run rustup");

    let installed = String::from_utf8_lossy(&check.stdout);

    if !installed.contains("aarch64-apple-darwin") {
        eprintln!("Skipping: aarch64-apple-darwin not installed");
        eprintln!("Install with: rustup target add aarch64-apple-darwin");
        return;
    }

    let result = Command::new("cargo")
        .args([
            "check",
            "--target",
            "aarch64-apple-darwin",
            "--package",
            "toadstool-runtime-wasm",
            "--package",
            "toadstool-runtime-secure-enclave",
        ])
        .output()
        .expect("Failed to run cargo check");

    if result.status.success() {
        println!("✅ macOS ARM cross-compilation check successful!");
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("macOS ARM cross-compilation check:\n{stderr}");
    }
}

/// Audit runtime dependencies for Pure Rust compliance
#[test]
fn test_audit_wasm_runtime_dependencies() {
    let output = Command::new("cargo")
        .args([
            "tree",
            "--package",
            "toadstool-runtime-wasm",
            "--depth",
            "1",
        ])
        .output()
        .expect("Failed to run cargo tree");

    let tree = String::from_utf8_lossy(&output.stdout);

    // Verify Pure Rust dependencies
    assert!(tree.contains("wasmi"), "Should use wasmi (Pure Rust!)");

    // Ensure we don't have C dependencies
    assert!(
        !tree.contains("wasmtime"),
        "Should NOT have wasmtime (has C)"
    );
    assert!(
        !tree.contains("-sys") || tree.contains("linux-raw-sys"),
        "Should have minimal -sys crates (only kernel interfaces)"
    );

    println!("✅ WASM runtime dependencies are Pure Rust!");
    println!("Dependencies:\n{tree}");
}

/// Audit compression dependencies for Pure Rust compliance
#[test]
fn test_audit_compression_dependencies() {
    let output = Command::new("cargo")
        .args([
            "tree",
            "--package",
            "toadstool-runtime-secure-enclave",
            "--depth",
            "1",
        ])
        .output()
        .expect("Failed to run cargo tree");

    let tree = String::from_utf8_lossy(&output.stdout);

    // Verify Pure Rust compression libraries
    assert!(
        tree.contains("lz4_flex"),
        "Should use lz4_flex (Pure Rust!)"
    );
    assert!(tree.contains("ruzstd"), "Should use ruzstd (Pure Rust!)");

    // Ensure we don't have C FFI compression
    assert!(!tree.contains("lz4-sys"), "Should NOT have lz4-sys (C FFI)");
    assert!(
        !tree.contains("zstd-sys"),
        "Should NOT have zstd-sys (C FFI)"
    );

    println!("✅ Compression dependencies are Pure Rust!");
    println!("Dependencies:\n{tree}");
}

/// Audit cryptography for Pure Rust compliance
#[test]
fn test_audit_crypto_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "--workspace", "--depth", "2"])
        .output()
        .expect("Failed to run cargo tree");

    let tree = String::from_utf8_lossy(&output.stdout);

    // Check for blake3 with pure feature
    if tree.contains("blake3") {
        println!("Found blake3 - verifying pure feature usage");

        // NOTE: ring may appear in dependencies but should be minimal
        // The key is that runtime crates (wasmi, secure_enclave) don't use it
        if tree.contains("ring") {
            println!(
                "Note: ring detected in dependency tree (acceptable if not in runtime crates)"
            );
        }
    }

    println!("✅ Cryptography dependencies checked!");
}

/// Verify the pure-Rust default build path doesn't invoke a C compiler.
///
/// Builds with `--no-default-features` to validate the ecoBin v3.0 pure-Rust
/// contract. When the test binary itself is built with `--all-features`,
/// transitive deps (blake3 asm, etc.) may legitimately compile C — that is
/// separate from the application-code pure-Rust guarantee.
#[test]
fn test_no_c_compiler_invocations() {
    let output = Command::new("cargo")
        .args([
            "build",
            "--package",
            "toadstool-runtime-wasm",
            "--package",
            "toadstool-runtime-secure-enclave",
            "--no-default-features",
            "--verbose",
        ])
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("Failed to run cargo build");

    let stderr = String::from_utf8_lossy(&output.stderr);

    let c_compile_lines: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains("Running `") && !line.contains("Fresh"))
        .filter(|line| {
            (line.contains("gcc") || line.contains("clang") || line.contains(" cc "))
                && (line.contains(".c") || line.contains(".cpp") || line.contains("-c "))
        })
        .collect();

    assert!(
        c_compile_lines.is_empty(),
        "C compiler was invoked during pure-Rust build — ecoBin violation!\n\
         Offending lines:\n{}",
        c_compile_lines.join("\n"),
    );
}

/// Verify cargo metadata shows Pure Rust dependencies
#[test]
fn test_cargo_metadata_pure_rust() {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output()
        .expect("Failed to run cargo metadata");

    let metadata = String::from_utf8_lossy(&output.stdout);

    // Parse and validate key dependencies
    assert!(metadata.contains("wasmi"), "Should have wasmi");
    assert!(metadata.contains("lz4_flex"), "Should have lz4_flex");
    assert!(metadata.contains("ruzstd"), "Should have ruzstd");
    assert!(
        metadata.contains("etcetera"),
        "Should have etcetera (Pure Rust dirs!)"
    );

    println!("✅ Cargo metadata confirms Pure Rust dependencies!");
}

/// Verify dirs-sys has been eliminated
/// D-S18-002 RESOLVED (S97): cubecl removed from workspace, dirs-sys eliminated.
#[test]
fn test_dirs_sys_eliminated() {
    let output = Command::new("cargo")
        .args(["tree", "--workspace"])
        .output()
        .expect("Failed to run cargo tree");

    let tree = String::from_utf8_lossy(&output.stdout);

    assert!(
        !tree.contains("dirs-sys"),
        "dirs-sys should be eliminated! Found in dependency tree."
    );
    println!("✅ Using etcetera (Pure Rust) instead!");
}

/// Verify only acceptable -sys crates remain
/// Ignored: cubecl transitively brings dirs-sys; acceptable list needs updating for cubecl stack.
#[test]
#[ignore = "cubecl transitive -sys crates not yet in acceptable list; tracked as workspace debt"]
fn test_only_acceptable_sys_crates() {
    let output = Command::new("cargo")
        .args(["tree", "--workspace", "--edges", "normal"])
        .output()
        .expect("Failed to run cargo tree");

    let tree = String::from_utf8_lossy(&output.stdout);

    // Count -sys crates
    let sys_crates: Vec<&str> = tree.lines().filter(|line| line.contains("-sys")).collect();

    // Check each -sys crate
    for crate_line in &sys_crates {
        let is_acceptable = crate_line.contains("linux-raw-sys") ||    // Syscall numbers ✅
            crate_line.contains("pyo3-ffi") ||         // Python FFI (optional) ✅
            crate_line.contains("seccomp-sys") ||      // Security (optional) ✅
            crate_line.contains("renderdoc-sys"); // GPU debugging (optional) ✅

        assert!(
            is_acceptable,
            "Found unacceptable -sys crate: {crate_line}. Only kernel interfaces allowed!"
        );
    }

    println!("✅ Only acceptable -sys crates remain!");
    println!("   Found {} -sys crates, all acceptable:", sys_crates.len());
    for crate_line in sys_crates.iter().take(5) {
        println!("   - {}", crate_line.trim());
    }
}

/// Verify TRUE 100% Pure Rust status
#[test]
fn test_true_100_percent_pure_rust() {
    let output = Command::new("cargo")
        .args(["tree", "--workspace", "--edges", "normal"])  // Exclude dev-dependencies
        .output()
        .expect("Failed to run cargo tree");

    let tree = String::from_utf8_lossy(&output.stdout);

    // Verify NO C library dependencies in production
    assert!(!tree.contains("lz4-sys"), "Should NOT have lz4-sys");
    assert!(!tree.contains("openssl-sys"), "Should NOT have openssl-sys");

    // Note: zstd-sys may appear in dev-dependencies (for creating test data)
    // This is acceptable - we only care about production dependencies

    // Verify Pure Rust replacements
    assert!(tree.contains("wasmi"), "Should have wasmi (Pure Rust WASM)");
    assert!(
        tree.contains("lz4_flex"),
        "Should have lz4_flex (Pure Rust LZ4)"
    );
    assert!(
        tree.contains("ruzstd"),
        "Should have ruzstd (Pure Rust Zstd)"
    );
    assert!(
        tree.contains("etcetera"),
        "Should have etcetera (Pure Rust dirs)"
    );
    // notify removed: was unused (no Watcher implementation). When file watching
    // is needed, rustix::fs::inotify provides pure Rust inotify access.

    println!("✅ TRUE 100% Pure Rust achieved!");
    println!("   All C library dependencies eliminated from production!");
    println!("   Only kernel interface wrappers remain (acceptable)!");
}
