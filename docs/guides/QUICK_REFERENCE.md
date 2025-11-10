# 📖 Quick Reference Guide

**Last Updated**: October 9, 2025  
**Current Coverage**: 21.86%  
**Target Coverage**: 90%  
**Status**: Active Development

---

## 🚀 **COMMON COMMANDS**

### **Build & Test**
```bash
# Build all libraries
cargo build --workspace --lib

# Run all tests
cargo test --workspace --lib

# Run tests for specific crate
cargo test -p toadstool-runtime-native

# Run tests matching pattern
cargo test --lib test_execution

# Run tests with output
cargo test --lib -- --nocapture
```

### **Coverage**
```bash
# Generate HTML coverage report
cargo tarpaulin --workspace --lib --out Html

# Generate coverage with specific output dir
cargo tarpaulin --workspace --lib --out Html --output-dir coverage

# View coverage report
firefox coverage/tarpaulin-report.html

# Quick coverage check
cargo tarpaulin --workspace --lib | grep "coverage"
```

### **Code Quality**
```bash
# Format all code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --workspace --lib

# Run clippy with all warnings as errors
cargo clippy --workspace --lib -- -D warnings

# Check for outdated dependencies
cargo outdated
```

### **Documentation**
```bash
# Generate docs
cargo doc --workspace --no-deps

# Generate and open docs
cargo doc --workspace --no-deps --open

# Check for doc errors
cargo doc --workspace --no-deps 2>&1 | grep warning
```

---

## 📁 **KEY FILES**

### **Documentation**
- `EXECUTIVE_AUDIT_SUMMARY.md` - Start here!
- `AUDIT_ANSWERS_TO_USER_QUESTIONS.md` - All questions answered
- `COMPREHENSIVE_CODEBASE_AUDIT_REPORT.md` - Full technical analysis
- `START_HERE.md` - Developer quick start
- `STATUS.md` - Current project status
- `coverage/tarpaulin-report.html` - Interactive coverage

### **Configuration**
- `Cargo.toml` - Workspace configuration
- `biome.yaml` - Biome configuration (if used)
- `.github/workflows/` - CI/CD pipelines (create this!)

### **Code**
- `crates/` - All source code
- `tests/` - Integration and E2E tests
- `examples/` - Working examples

---

## 🎯 **PRIORITY MODULES** (0% Coverage)

### **Critical Modules to Test First**
```
1. crates/core/toadstool/src/universal.rs (411 lines)
2. crates/distributed/src/cloud.rs (430 lines)
3. crates/distributed/src/songbird_integration.rs (558 lines)
4. crates/cli/src/zero_config.rs (456 lines)
5. crates/core/toadstool/src/performance_hardening.rs (283 lines)
6. crates/core/toadstool/src/security_hardening.rs (223 lines)
7. crates/core/toadstool/src/production_hardening.rs (203 lines)
8. crates/distributed/src/substrate_detection.rs (232 lines)
```

### **Well-Tested Modules** (Reference for patterns)
```
1. crates/testing/src/integration.rs (89.4% coverage)
2. crates/security/sandbox/src/lib.rs (86.7% coverage)
3. crates/core/common/src/infant_discovery/engine.rs (87.5% coverage)
```

---

## 📊 **METRICS AT A GLANCE**

### **Current State**
```
Coverage:       21.86% (3,927 / 17,962 lines)
Tests:          195 passing (100% pass rate)
Build:          ✅ All crates compile
Formatting:     ✅ 100% compliant
Clippy:         ✅ 0 warnings (lib)
Unsafe Code:    ✅ 0 instances
Security:       ✅ A+ grade
Sovereignty:    ✅ 100/100
```

### **Targets**
```
Coverage:       90% (need +68.14%)
Tests:          ~1,200 (need +1,000)
Timeline:       6-8 months
Files:          All <1000 lines
```

---

## 🔧 **DEBUGGING**

### **Test Failures**
```bash
# Run single test with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --exact

# Run tests serially (avoid parallel conflicts)
cargo test --lib -- --test-threads=1

# Show test output
cargo test --lib -- --nocapture
```

### **Build Errors**
```bash
# Clean build
cargo clean && cargo build --workspace --lib

# Check specific crate
cargo check -p crate-name

# Verbose build
cargo build --workspace --lib --verbose
```

### **Coverage Issues**
```bash
# Clean coverage cache
rm -rf target/tarpaulin

# Run with verbose output
cargo tarpaulin --workspace --lib --verbose

# Generate JSON for debugging
cargo tarpaulin --workspace --lib --out Json
```

---

## 📈 **PROGRESS TRACKING**

### **Weekly Check-In**
```bash
# 1. Current coverage
cargo tarpaulin --workspace --lib | grep "coverage"

# 2. Test count
cargo test --workspace --lib 2>&1 | grep "test result"

# 3. Build status
cargo build --workspace --lib 2>&1 | grep -E "Finished|error"

# 4. Code quality
cargo fmt --check && cargo clippy --workspace --lib
```

### **Create Progress Log**
```bash
# Add to COVERAGE_PROGRESS.md
echo "$(date +%Y-%m-%d): Coverage: $(cargo tarpaulin --workspace --lib 2>&1 | grep -oP '\d+\.\d+(?=% coverage)')" >> COVERAGE_PROGRESS.md
```

---

## 🎯 **DAILY WORKFLOW**

### **Morning Routine**
```bash
# 1. Pull latest changes
git pull origin main

# 2. Verify build
cargo build --workspace --lib

# 3. Run tests
cargo test --workspace --lib

# 4. Check coverage
cargo tarpaulin --workspace --lib | tail -5
```

### **Writing Tests**
```bash
# 1. Choose module
MODULE="crates/core/toadstool/src/universal.rs"

# 2. Check current coverage
cargo tarpaulin --workspace --lib | grep universal

# 3. Edit and add tests
# Add tests to #[cfg(test)] mod tests

# 4. Run new tests
cargo test --lib universal

# 5. Verify improvement
cargo tarpaulin --workspace --lib --out Html
```

### **Evening Commit**
```bash
# 1. Format code
cargo fmt

# 2. Run all checks
cargo build --workspace --lib
cargo test --workspace --lib
cargo clippy --workspace --lib

# 3. Commit progress
git add -A
git commit -m "test: add tests for MODULE (+X lines coverage)"
git push
```

---

## 🆘 **COMMON ISSUES**

### **Test Timeout**
```rust
// Add timeout annotation
#[tokio::test(flavor = "multi_thread")]
#[timeout(5000)] // 5 seconds
async fn test_something() {
    // ...
}
```

### **Parallel Test Conflicts**
```rust
// Use serial_test for tests that conflict
use serial_test::serial;

#[test]
#[serial]
fn test_shared_resource() {
    // ...
}
```

### **Coverage Not Updating**
```bash
# Clean and regenerate
cargo clean
rm -rf target/tarpaulin
cargo tarpaulin --workspace --lib --out Html
```

---

## 📞 **QUICK HELP**

### **Stuck?**
1. Check existing tests in `crates/testing/`
2. Review audit reports
3. Look at well-tested modules for patterns
4. Check Rust book: https://doc.rust-lang.org/book/ch11-00-testing.html

### **Coverage Questions?**
- View: `coverage/tarpaulin-report.html`
- Check: `COVERAGE_REALITY_UPDATE.md`
- Details: `COMPREHENSIVE_CODEBASE_AUDIT_REPORT.md`

### **General Questions?**
- Start: `EXECUTIVE_AUDIT_SUMMARY.md`
- Answers: `AUDIT_ANSWERS_TO_USER_QUESTIONS.md`
- Status: `STATUS.md`

---

## 🎊 **CELEBRATE PROGRESS**

### **Small Wins**
- ✅ Every 1% coverage improvement
- ✅ Every 10 tests added
- ✅ Every module reaching >50% coverage
- ✅ Every 0% module getting first tests

### **Big Wins**
- 🎉 Reaching 30% coverage (first milestone)
- 🎉 Reaching 50% coverage (halfway!)
- 🎉 Reaching 70% coverage (almost there!)
- 🎉 Reaching 90% coverage (PRODUCTION READY!)

---

**Keep going! You've got this!** 🚀

**Reality > Hype. Truth > Marketing. Safety > Speed.** ✅

