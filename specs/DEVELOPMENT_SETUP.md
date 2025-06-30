---
title: ToadStool Development Setup Specification
description: Development environment, tooling, and build system configuration
version: 1.0.0
date: 2025-01-26
author: ToadStool Development Team
priority: HIGH
status: DEV_SPEC
---

# 🛠️ Development Setup Specification

## Executive Summary

ToadStool development environment is designed for **maximum developer productivity** with consistent cross-platform setup, comprehensive tooling, automated testing, and seamless contribution workflows.

---

## 🎯 **Development Environment Architecture**

### **Core Development Stack**
```yaml
core_requirements:
  rust_version: ">=1.75.0"
  cargo_version: ">=1.75.0"
  target_architectures: ["x86_64", "aarch64"]
  supported_platforms: ["linux", "macos", "windows"]
  container_runtime: ["docker", "podman"]
  
development_tools:
  build_system: "cargo + custom scripts"
  testing_framework: "cargo test + custom harness"
  documentation: "cargo doc + mdbook"
  linting: "clippy + custom lints"
  formatting: "rustfmt + custom config"
  security_scanning: "cargo audit + semgrep"
```

### **Project Structure**
```
toadstool/
├── Cargo.toml                 # Workspace configuration
├── Cargo.lock                 # Dependency lock file
├── .rustfmt.toml              # Rust formatting config
├── clippy.toml                # Clippy linting config
├── .github/                   # GitHub workflows and templates
├── docs/                      # Documentation and guides
├── specs/                     # Technical specifications
├── tools/                     # Development tools and scripts
├── tests/                     # Integration and E2E tests
├── benchmarks/                # Performance benchmarks
├── examples/                  # Usage examples
└── crates/                    # Rust crates
    ├── core/                  # Core ToadStool functionality
    │   ├── toadstool/         # Main library crate
    │   ├── config/            # Configuration management
    │   └── common/            # Shared utilities
    ├── runtime/               # Execution runtime implementations
    │   ├── container/         # Container runtime
    │   ├── wasm/              # WebAssembly runtime
    │   ├── native/            # Native runtime
    │   └── gpu/               # GPU compute runtime
    ├── security/              # Security and sandboxing
    │   ├── sandbox/           # Cross-platform sandboxing
    │   ├── policies/          # Security policies
    │   └── monitoring/        # Security monitoring
    ├── management/            # Resource and performance management
    │   ├── resources/         # Resource management
    │   ├── performance/       # Performance optimization
    │   └── monitoring/        # System monitoring
    ├── integration/           # External service integration
    │   ├── songbird/          # Songbird service discovery
    │   ├── nestgate/          # NestGate storage integration
    │   └── protocols/         # Communication protocols
    ├── cli/                   # Command-line interface
    ├── server/                # ToadStool server implementation
    └── client/                # Client libraries
```

---

## 🔧 **Development Environment Setup**

### **Automated Environment Setup**
```bash
#!/bin/bash
# tools/setup-dev-env.sh - Automated development environment setup

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Linux*)  PLATFORM=linux;;
        Darwin*) PLATFORM=macos;;
        CYGWIN*|MINGW*|MSYS*) PLATFORM=windows;;
        *) error "Unsupported platform: $(uname -s)"; exit 1;;
    esac
    info "Detected platform: $PLATFORM"
}

# Install Rust toolchain
install_rust() {
    if ! command -v rustc &> /dev/null; then
        info "Installing Rust toolchain..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    else
        info "Rust already installed: $(rustc --version)"
    fi
    
    # Install required components
    rustup component add clippy rustfmt
    rustup target add wasm32-wasi wasm32-unknown-unknown
    
    # Install additional targets based on platform
    case $PLATFORM in
        linux)
            rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
            ;;
        macos)
            rustup target add x86_64-apple-darwin aarch64-apple-darwin
            ;;
        windows)
            rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc
            ;;
    esac
}

# Install development tools
install_dev_tools() {
    info "Installing development tools..."
    
    # Cargo extensions
    cargo install cargo-audit cargo-outdated cargo-tree cargo-expand
    cargo install cargo-watch cargo-nextest
    cargo install mdbook mdbook-mermaid
    
    # Platform-specific tools
    case $PLATFORM in
        linux)
            install_linux_tools
            ;;
        macos)
            install_macos_tools
            ;;
        windows)
            install_windows_tools
            ;;
    esac
}

# Setup project configuration
setup_project_config() {
    info "Setting up project configuration..."
    
    # Git hooks
    cp tools/git-hooks/* .git/hooks/
    chmod +x .git/hooks/*
    
    # IDE configuration
    mkdir -p .vscode
    cp tools/vscode/* .vscode/
    
    # Environment files
    if [[ ! -f .env.local ]]; then
        cp .env.example .env.local
        warn "Created .env.local - please customize for your environment"
    fi
}

# Verify installation
verify_installation() {
    info "Verifying installation..."
    
    # Check Rust installation
    rustc --version || error "Rust installation failed"
    cargo --version || error "Cargo installation failed"
    
    # Check project build
    cargo check --all-targets || error "Project check failed"
    
    # Run basic tests
    cargo test --lib || error "Basic tests failed"
    
    info "Development environment setup complete!"
}

main() {
    detect_platform
    install_rust
    install_dev_tools
    setup_project_config
    verify_installation
}

main "$@"
```

### **Platform-Specific Development Setup**
```rust
// tools/src/platform_setup.rs - Platform-specific development utilities

use std::process::Command;
use anyhow::{Result, Context};

#[derive(Debug, Clone)]
pub struct PlatformSetup {
    platform: Platform,
    config: PlatformConfig,
}

impl PlatformSetup {
    pub fn new() -> Result<Self> {
        let platform = Platform::detect()?;
        let config = PlatformConfig::load_for_platform(platform)?;
        
        Ok(Self { platform, config })
    }
    
    /// Install platform-specific development dependencies
    pub async fn install_platform_dependencies(&self) -> Result<()> {
        match self.platform {
            Platform::Linux => self.install_linux_dependencies().await,
            Platform::MacOS => self.install_macos_dependencies().await,
            Platform::Windows => self.install_windows_dependencies().await,
        }
    }
    
    async fn install_linux_dependencies(&self) -> Result<()> {
        // Install system packages for development
        let packages = vec![
            "build-essential", "pkg-config", "libssl-dev",
            "libseccomp-dev", "libudev-dev", "libdbus-1-dev",
            "docker.io", "containerd", "podman",
        ];
        
        self.run_package_manager("apt", &["update"])?;
        self.run_package_manager("apt", &["install", "-y"])
            .with_args(&packages)?;
        
        // Setup development containers
        self.setup_development_containers().await?;
        
        Ok(())
    }
    
    async fn install_macos_dependencies(&self) -> Result<()> {
        // Install Homebrew packages
        let packages = vec![
            "pkg-config", "openssl", "docker", "podman",
            "llvm", "cmake",
        ];
        
        self.run_package_manager("brew", &["update"])?;
        self.run_package_manager("brew", &["install"])
            .with_args(&packages)?;
        
        // Setup macOS-specific development tools
        self.setup_macos_dev_tools().await?;
        
        Ok(())
    }
    
    async fn install_windows_dependencies(&self) -> Result<()> {
        // Install via Chocolatey or winget
        let packages = vec![
            "git", "docker-desktop", "llvm", "cmake",
            "windows-sdk-10-version-2004-all",
        ];
        
        self.run_package_manager("choco", &["install", "-y"])
            .with_args(&packages)?;
        
        // Setup Windows-specific development environment
        self.setup_windows_dev_environment().await?;
        
        Ok(())
    }
}
```

---

## 🔨 **Build System Configuration**

### **Comprehensive Cargo Configuration**
```toml
# Cargo.toml - Workspace configuration
[workspace]
members = [
    "crates/core/*",
    "crates/runtime/*",
    "crates/security/*",
    "crates/management/*",
    "crates/integration/*",
    "crates/cli",
    "crates/server",
    "crates/client",
]

resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75.0"
authors = ["ToadStool Development Team"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/ecosystem/toadstool"
homepage = "https://toadstool.ecosystem.dev"
documentation = "https://docs.toadstool.ecosystem.dev"

[workspace.dependencies]
# Core dependencies
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Async and concurrency
async-trait = "0.1"
futures = "0.3"
dashmap = "5.5"

# Configuration and CLI
clap = { version = "4.4", features = ["derive"] }
config = "0.14"
toml = "0.8"

# Networking and protocols
reqwest = { version = "0.11", features = ["json"] }
tonic = "0.10"
prost = "0.12"

# Security and cryptography
ring = "0.17"
rustls = "0.22"
webpki-roots = "0.26"

# Platform-specific dependencies
[target.'cfg(target_os = "linux")'.workspace.dependencies]
libc = "0.2"
nix = "0.27"
seccomp-sys = "0.2"

[target.'cfg(target_os = "macos")'.workspace.dependencies]
core-foundation = "0.9"
security-framework = "2.9"
system-configuration = "0.5"

[target.'cfg(target_os = "windows")'.workspace.dependencies]
winapi = { version = "0.3", features = ["full"] }
windows = { version = "0.52", features = ["full"] }

[profile.dev]
opt-level = 0
debug = true
split-debuginfo = "unpacked"
strip = false
debug-assertions = true
overflow-checks = true
lto = false
panic = "unwind"
incremental = true
codegen-units = 256

[profile.release]
opt-level = 3
debug = false
strip = "symbols"
debug-assertions = false
overflow-checks = false
lto = "thin"
panic = "abort"
incremental = false
codegen-units = 1

[profile.test]
opt-level = 1
debug = true
debug-assertions = true
overflow-checks = true
incremental = true

[profile.bench]
opt-level = 3
debug = false
debug-assertions = false
overflow-checks = false
lto = true
codegen-units = 1
```

### **Custom Build Scripts and Tools**
```rust
// tools/src/build_tools.rs - Custom build utilities

use std::process::{Command, Stdio};
use anyhow::{Result, Context};

pub struct BuildTools {
    workspace_root: PathBuf,
    target_dir: PathBuf,
    cargo_config: CargoConfig,
}

impl BuildTools {
    /// Build all workspace crates with optimizations
    pub async fn build_workspace(&self, options: BuildOptions) -> Result<BuildResult> {
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        
        if options.release {
            cmd.arg("--release");
        }
        
        if let Some(target) = &options.target {
            cmd.args(&["--target", target]);
        }
        
        if options.all_features {
            cmd.arg("--all-features");
        } else if !options.features.is_empty() {
            cmd.args(&["--features", &options.features.join(",")]);
        }
        
        // Add workspace-specific flags
        cmd.args(&["--workspace", "--all-targets"]);
        
        // Configure output
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let output = cmd.output()
            .context("Failed to execute cargo build")?;
        
        BuildResult::from_command_output(output)
    }
    
    /// Run comprehensive test suite
    pub async fn run_tests(&self, options: TestOptions) -> Result<TestResult> {
        let mut results = TestResult::new();
        
        // Unit tests
        results.add_suite(self.run_unit_tests(&options).await?);
        
        // Integration tests
        results.add_suite(self.run_integration_tests(&options).await?);
        
        // Documentation tests
        results.add_suite(self.run_doc_tests(&options).await?);
        
        // Platform-specific tests
        for platform in &options.target_platforms {
            results.add_suite(self.run_platform_tests(platform, &options).await?);
        }
        
        // Performance benchmarks (if requested)
        if options.include_benchmarks {
            results.add_suite(self.run_benchmarks(&options).await?);
        }
        
        Ok(results)
    }
    
    /// Generate comprehensive documentation
    pub async fn generate_documentation(&self) -> Result<DocumentationResult> {
        // Generate API documentation
        let api_docs = self.generate_api_docs().await?;
        
        // Generate specification documentation
        let spec_docs = self.generate_spec_docs().await?;
        
        // Generate user guides
        let user_guides = self.generate_user_guides().await?;
        
        Ok(DocumentationResult {
            api_docs,
            spec_docs,
            user_guides,
        })
    }
}
```

---

## 🧪 **Testing Framework**

### **Comprehensive Testing Strategy**
```rust
// tools/src/testing.rs - Testing framework and utilities

#[derive(Debug, Clone)]
pub struct TestHarness {
    config: TestConfig,
    runners: HashMap<TestType, Box<dyn TestRunner>>,
    reporters: Vec<Box<dyn TestReporter>>,
}

impl TestHarness {
    /// Run all tests with comprehensive reporting
    pub async fn run_all_tests(&self) -> Result<TestReport> {
        let mut report = TestReport::new();
        
        // Unit tests - fast, isolated tests
        report.add_results(self.run_unit_tests().await?);
        
        // Integration tests - test component interactions
        report.add_results(self.run_integration_tests().await?);
        
        // End-to-end tests - full system tests
        report.add_results(self.run_e2e_tests().await?);
        
        // Performance tests - benchmark critical paths
        report.add_results(self.run_performance_tests().await?);
        
        // Security tests - validate security properties
        report.add_results(self.run_security_tests().await?);
        
        // Platform tests - cross-platform compatibility
        report.add_results(self.run_platform_tests().await?);
        
        // Generate comprehensive report
        for reporter in &self.reporters {
            reporter.generate_report(&report).await?;
        }
        
        Ok(report)
    }
    
    async fn run_platform_tests(&self) -> Result<TestResults> {
        let mut results = TestResults::new();
        
        // Test on all supported platforms
        let platforms = vec![Platform::Linux, Platform::MacOS, Platform::Windows];
        
        for platform in platforms {
            if let Some(runner) = self.get_platform_runner(platform) {
                let platform_results = runner.run_tests().await?;
                results.merge(platform_results);
            }
        }
        
        Ok(results)
    }
}

// Specialized test configurations for different test types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Unit test configuration
    pub unit_tests: UnitTestConfig,
    /// Integration test configuration
    pub integration_tests: IntegrationTestConfig,
    /// End-to-end test configuration
    pub e2e_tests: E2ETestConfig,
    /// Performance test configuration
    pub performance_tests: PerformanceTestConfig,
    /// Security test configuration
    pub security_tests: SecurityTestConfig,
    /// Cross-platform test configuration
    pub platform_tests: PlatformTestConfig,
}
```

### **Test Automation and CI/CD Integration**
```yaml
# .github/workflows/ci.yml - Comprehensive CI/CD pipeline
name: Continuous Integration

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    name: Test Suite
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta]
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    
    runs-on: ${{ matrix.os }}
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust toolchain
      uses: dtolnay/rust-toolchain@stable
      with:
        toolchain: ${{ matrix.rust }}
        components: clippy, rustfmt
        targets: ${{ matrix.target }}
    
    - name: Setup development environment
      run: ./tools/setup-dev-env.sh
    
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Check formatting
      run: cargo fmt --all -- --check
    
    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings
    
    - name: Build workspace
      run: cargo build --workspace --all-targets --all-features
    
    - name: Run test suite
      run: cargo test --workspace --all-features
    
    - name: Run integration tests
      run: cargo test --workspace --test '*' --all-features
    
    - name: Run documentation tests
      run: cargo test --workspace --doc --all-features
    
    - name: Security audit
      run: cargo audit
    
    - name: Generate documentation
      run: cargo doc --workspace --all-features --no-deps
```

---

## 📊 **Development Workflows**

### **Code Quality Automation**
```rust
// tools/src/quality.rs - Code quality automation

pub struct QualityTools {
    linter: Box<dyn Linter>,
    formatter: Box<dyn Formatter>,
    security_scanner: Box<dyn SecurityScanner>,
    dependency_checker: Box<dyn DependencyChecker>,
}

impl QualityTools {
    /// Run comprehensive code quality checks
    pub async fn run_quality_checks(&self) -> Result<QualityReport> {
        let mut report = QualityReport::new();
        
        // Format code
        let format_result = self.formatter.format_code().await?;
        report.add_check("formatting", format_result);
        
        // Lint code
        let lint_result = self.linter.lint_code().await?;
        report.add_check("linting", lint_result);
        
        // Security scan
        let security_result = self.security_scanner.scan_code().await?;
        report.add_check("security", security_result);
        
        // Dependency audit
        let dependency_result = self.dependency_checker.check_dependencies().await?;
        report.add_check("dependencies", dependency_result);
        
        Ok(report)
    }
}

// Pre-commit hooks integration
pub struct PreCommitHooks {
    checks: Vec<Box<dyn PreCommitCheck>>,
}

impl PreCommitHooks {
    /// Install pre-commit hooks
    pub fn install_hooks(&self) -> Result<()> {
        let hook_content = self.generate_hook_script()?;
        std::fs::write(".git/hooks/pre-commit", hook_content)?;
        
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(".git/hooks/pre-commit")?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(".git/hooks/pre-commit", perms)?;
        }
        
        Ok(())
    }
}
```

This development setup specification provides a comprehensive foundation for ToadStool development with automated tooling, consistent environments, and robust quality assurance processes across all supported platforms. 