# toadstool-testing

**Testing utilities, mocks, and infrastructure for ToadStool**

Comprehensive testing support for ToadStool development, including mock implementations, test fixtures, and testing utilities.

## Features

- **Mock Implementations**: Mock runtimes, storage, and services
- **Test Fixtures**: Pre-built test data and scenarios
- **Testing Utilities**: Helpers for async testing, assertions, and more
- **Chaos Testing**: Infrastructure for fault injection and chaos engineering

## Usage

Add to your `dev-dependencies`:

```toml
[dev-dependencies]
toadstool-testing = "0.1"
```

## Mock Runtimes

Test your code without real runtime engines:

```rust
use toadstool_testing::mocks::MockRuntime;

#[tokio::test]
async fn test_execution() {
    let mut runtime = MockRuntime::new();
    runtime.expect_execute()
        .returning(|_| Ok(ExecutionResult::success()));
    
    // Test your code with the mock
    let result = runtime.execute(request).await?;
    assert!(result.is_success());
}
```

## Test Fixtures

Pre-built test data for common scenarios:

```rust
use toadstool_testing::fixtures;

#[test]
fn test_with_fixtures() {
    let config = fixtures::valid_config();
    let workload = fixtures::sample_workload();
    let result = process(config, workload)?;
    // ...
}
```

## Async Testing Utilities

Helpers for testing async code:

```rust
use toadstool_testing::async_utils::{timeout_test, retry_until};

#[tokio::test]
async fn test_with_timeout() {
    timeout_test(Duration::from_secs(5), async {
        // Your async test code
    }).await?;
}
```

## Chaos Testing

Fault injection for robustness testing:

```rust
use toadstool_testing::chaos::{inject_network_delay, inject_memory_pressure};

#[tokio::test]
async fn test_under_chaos() {
    let mut runtime = Runtime::new();
    inject_network_delay(&mut runtime, Duration::from_millis(100));
    
    // Test behavior under degraded conditions
    let result = runtime.execute(request).await;
    assert!(result.is_ok());  // Should handle delays gracefully
}
```

## Quality

This crate demonstrates testing best practices:

- ✅ Mocks isolated to testing (ZERO in production)
- ✅ Comprehensive test utilities
- ✅ Chaos engineering support
- ✅ Well-documented test patterns

## Architecture

Follows ToadStool's quality principles:

- **Isolated Mocks**: 95%+ isolation (only in tests)
- **Deep Solutions**: Real chaos testing, not toy examples
- **Modern Rust**: Idiomatic patterns throughout

## License

AGPL-3.0-only

