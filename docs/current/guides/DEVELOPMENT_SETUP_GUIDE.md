# 🛠️ ToadStool Development Setup Guide

**Date**: January 2025  
**Status**: ACTIVE - Consolidated development setup guide  
**Audience**: Developers, Contributors, DevOps  
**Version**: 2.0 (Consolidated)

---

## 🚀 **Quick Start**

### **Prerequisites**
```bash
# System Requirements
- Rust 1.70+ (latest stable recommended)
- Docker 20.10+ (for container runtime)
- Git 2.30+
- 8GB+ RAM recommended
- 50GB+ available disk space

# Platform Support
- Linux (Ubuntu 20.04+, CentOS 8+, Arch Linux)
- macOS 12+ (Intel and Apple Silicon)
- Windows 11 (WSL2 recommended)
```

### **Installation**
```bash
# 1. Clone repository
git clone https://github.com/ecoPrimals/toadstool.git
cd toadstool

# 2. Build project
cargo build --release

# 3. Run tests
cargo test --workspace

# 4. Run examples
cargo run --example production_universal_demo
```

---

## 🏗️ **Development Environment Setup**

### **Rust Development Environment**
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install required components
rustup component add rustfmt clippy

# Install development tools
cargo install cargo-watch cargo-expand cargo-audit
```

### **IDE Configuration**

#### **VS Code (Recommended)**
```json
// .vscode/settings.json
{
    "rust-analyzer.cargo.features": ["all"],
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.allFeatures": true,
    "files.watcherExclude": {
        "**/target/**": true
    }
}
```

#### **Required Extensions**
- `rust-lang.rust-analyzer` - Rust language support
- `vadimcn.vscode-lldb` - Debugging support
- `serayuzgur.crates` - Cargo.toml management
- `tamasfe.even-better-toml` - TOML syntax support

### **Development Tools**
```bash
# Code formatting
cargo fmt

# Linting
cargo clippy -- -D warnings

# Security audit
cargo audit

# Documentation generation
cargo doc --open

# Watch mode for development
cargo watch -x "build --workspace"
```

---

## 🧪 **Testing Setup**

### **Test Categories**
```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration_tests

# Performance benchmarks
cargo test --test performance_benchmarks

# Chaos engineering tests
cargo test --test chaos_engineering

# All tests
cargo test --workspace
```

### **Test Configuration**
```toml
# Cargo.toml - Test configuration
[dev-dependencies]
tokio-test = "0.4"
mockall = "0.11"
proptest = "1.0"
criterion = "0.5"
tempfile = "3.8"

[[test]]
name = "integration_tests"
path = "tests/integration/mod.rs"

[[bench]]
name = "performance_benchmarks"
harness = false
```

### **Mock Services for Testing**
```rust
// Test configuration
use mockall::predicate::*;
use toadstool_testing::mocks::*;

#[tokio::test]
async fn test_ecosystem_integration() {
    let mut mock_songbird = MockSongbirdClient::new();
    mock_songbird
        .expect_discover_services()
        .returning(|| Ok(vec!["toadstool-1", "toadstool-2"]));
    
    // Test implementation
}
```

---

## 🐳 **Container Development**

### **Docker Setup**
```dockerfile
# Dockerfile.dev - Development container
FROM rust:1.70-slim

WORKDIR /workspace
COPY . .

RUN cargo build --workspace
CMD ["cargo", "test", "--workspace"]
```

### **Docker Compose for Development**
```yaml
# docker-compose.dev.yml
version: '3.8'
services:
  toadstool-dev:
    build:
      context: .
      dockerfile: Dockerfile.dev
    volumes:
      - .:/workspace
      - cargo-cache:/usr/local/cargo/registry
    environment:
      - RUST_LOG=debug
      - TOADSTOOL_ENV=development
    ports:
      - "8080:8080"
      - "8081:8081"

volumes:
  cargo-cache:
```

### **Development Commands**
```bash
# Start development environment
docker-compose -f docker-compose.dev.yml up

# Run tests in container
docker-compose -f docker-compose.dev.yml run toadstool-dev cargo test

# Shell access
docker-compose -f docker-compose.dev.yml exec toadstool-dev bash
```

---

## 🔧 **Configuration Management**

### **Environment Configuration**
```bash
# Development environment
export TOADSTOOL_ENV=development
export RUST_LOG=debug
export TOADSTOOL_LOG_LEVEL=debug

# Songbird integration
export SONGBIRD_HOST=localhost
export SONGBIRD_PORT=8080

# BearDog security
export BEARDOG_SECURITY_LEVEL=Basic
export BEARDOG_CRYPTO_KEY_PATH=/tmp/beardog-keys

# NestGate storage
export NESTGATE_STORAGE_PATH=/tmp/toadstool-storage
```

### **Configuration Files**
```toml
# toadstool.toml - Main configuration
[runtime]
default_timeout = 300
max_concurrent_executions = 100

[logging]
level = "debug"
format = "json"

[security]
isolation_level = "Standard"
crypto_verification = true

[ecosystem]
songbird_discovery = true
auto_registration = true
```

---

## 🚀 **Zero-Touch Development**

### **Intelligent Auto-Configuration**
```rust
// Auto-configuration for development
use toadstool_auto_config::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Auto-detect development environment
    let config = AutoConfig::new()
        .detect_hardware()
        .discover_ecosystem_services()
        .optimize_for_development()
        .build()
        .await?;
    
    // Initialize ToadStool with auto-config
    let platform = ToadStool::new(config).await?;
    platform.start().await?;
    
    Ok(())
}
```

### **Natural Language Configuration**
```bash
# Natural language configuration for development
cargo run --bin toadstool-cli -- configure \
    "Set up development environment with debug logging and local Songbird"

# AI-assisted development setup
cargo run --bin toadstool-cli -- setup \
    "Configure for local development with hot reloading and test mocks"
```

---

## 📊 **Monitoring and Debugging**

### **Development Monitoring**
```rust
// Development monitoring setup
use toadstool_monitoring::*;

let monitor = DevelopmentMonitor::new()
    .with_real_time_metrics()
    .with_performance_tracking()
    .with_debug_logging()
    .start();
```

### **Debugging Tools**
```bash
# Debug builds with symbols
cargo build --workspace --profile dev

# Run with debugging
RUST_BACKTRACE=1 cargo run --example debug_demo

# Memory debugging with valgrind
cargo install cargo-valgrind
cargo valgrind --example memory_test

# Performance profiling
cargo install cargo-profiler
cargo profiler --example performance_test
```

### **Log Analysis**
```bash
# Structured logging analysis
cargo run --example logging_demo 2>&1 | jq '.'

# Performance metrics
cargo run --example metrics_demo | grep "performance"

# Error tracking
cargo run --example error_demo 2>&1 | grep "ERROR"
```

---

## 🔒 **Security Development**

### **Security Testing**
```bash
# Security audit
cargo audit

# Vulnerability scanning
cargo install cargo-deny
cargo deny check

# Security benchmarks
cargo test --test security_tests
```

### **Crypto Development**
```rust
// Crypto development setup
use toadstool_security::*;

let crypto_context = CryptoContext::development()
    .with_test_keys()
    .with_relaxed_validation()
    .build()?;
```

---

## 🚀 **Performance Development**

### **Performance Testing**
```bash
# Performance benchmarks
cargo bench

# Load testing
cargo run --example load_test

# Memory profiling
cargo run --example memory_profile
```

### **Optimization Tools**
```bash
# Performance analysis
cargo install cargo-flamegraph
cargo flamegraph --example performance_demo

# Memory analysis
cargo install cargo-bloat
cargo bloat --release --crates

# Binary size optimization
cargo install cargo-strip
cargo strip --release
```

---

## 🌐 **Ecosystem Development**

### **Local Ecosystem Setup**
```bash
# Start local ecosystem services
docker-compose -f ecosystem/docker-compose.yml up -d

# Verify ecosystem connectivity
cargo run --example ecosystem_health_check

# Test ecosystem integration
cargo test --test ecosystem_integration
```

### **Service Mocking**
```rust
// Mock ecosystem services for development
use toadstool_testing::ecosystem_mocks::*;

let mock_ecosystem = MockEcosystem::new()
    .with_songbird_mock()
    .with_beardog_mock()
    .with_nestgate_mock()
    .start()
    .await?;
```

---

## 📚 **Documentation Development**

### **Documentation Generation**
```bash
# Generate API documentation
cargo doc --workspace --no-deps --open

# Generate book documentation
mdbook build docs/book

# Generate specification docs
cargo run --bin doc-generator
```

### **Documentation Testing**
```bash
# Test code examples in documentation
cargo test --doc

# Validate documentation links
cargo install cargo-deadlinks
cargo deadlinks --check-http
```

---

## 🎯 **Development Workflows**

### **Feature Development**
```bash
# 1. Create feature branch
git checkout -b feature/new-runtime-engine

# 2. Implement feature with TDD
cargo watch -x "test --lib new_runtime_engine"

# 3. Run full test suite
cargo test --workspace

# 4. Format and lint
cargo fmt && cargo clippy

# 5. Create pull request
git push origin feature/new-runtime-engine
```

### **Bug Fixing**
```bash
# 1. Reproduce bug with test
cargo test --test bug_reproduction

# 2. Fix bug with monitoring
cargo watch -x "test bug_fix"

# 3. Verify fix
cargo test --workspace

# 4. Performance regression check
cargo bench
```

### **Release Preparation**
```bash
# 1. Version bump
cargo set-version 0.2.0

# 2. Update changelog
./scripts/generate-changelog.sh

# 3. Full test suite
cargo test --workspace --release

# 4. Security audit
cargo audit

# 5. Performance benchmarks
cargo bench --workspace
```

---

## 🔧 **Troubleshooting**

### **Common Issues**

#### **Compilation Errors**
```bash
# Clean build cache
cargo clean

# Update dependencies
cargo update

# Check for conflicting features
cargo tree --duplicates
```

#### **Test Failures**
```bash
# Run specific test with output
cargo test test_name -- --nocapture

# Run tests with backtrace
RUST_BACKTRACE=1 cargo test

# Run tests with debugging
RUST_LOG=debug cargo test
```

#### **Performance Issues**
```bash
# Profile performance
cargo flamegraph --example slow_example

# Check memory usage
cargo run --example memory_check

# Analyze binary size
cargo bloat --release
```

---

## 📈 **Advanced Development**

### **Custom Runtime Development**
```rust
// Implement custom runtime engine
use toadstool_core::runtime::*;

#[async_trait]
impl RuntimeEngine for CustomRuntime {
    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult> {
        // Custom runtime implementation
    }
}
```

### **Plugin Development**
```rust
// Develop ToadStool plugins
use toadstool_plugin_api::*;

#[plugin_main]
pub fn plugin_main() -> Box<dyn Plugin> {
    Box::new(CustomPlugin::new())
}
```

### **Ecosystem Service Development**
```rust
// Develop ecosystem service integration
use toadstool_ecosystem::*;

#[async_trait]
impl EcosystemService for CustomService {
    async fn discover(&self) -> Result<ServiceCapabilities> {
        // Service discovery implementation
    }
}
```

---

## 🎉 **Success Metrics**

### **Development Environment Health**
- ✅ **Build Time**: < 5 minutes for full workspace build
- ✅ **Test Time**: < 2 minutes for full test suite
- ✅ **Hot Reload**: < 5 seconds for code changes
- ✅ **Memory Usage**: < 2GB for development environment

### **Code Quality Metrics**
- ✅ **Test Coverage**: > 80% for core modules
- ✅ **Lint Warnings**: < 10 warnings across workspace
- ✅ **Security Audit**: Zero high-severity vulnerabilities
- ✅ **Performance**: No performance regressions

---

*This guide consolidates and replaces the following documents:*
- *DEVELOPMENT_SETUP.md*
- *ENHANCED_COMPUTE_PLATFORM_SPEC.md*
- *COMPREHENSIVE_TESTING_ROADMAP.md*
- *SPRINT_5_ZERO_TOUCH_FRIENDLY.md* 