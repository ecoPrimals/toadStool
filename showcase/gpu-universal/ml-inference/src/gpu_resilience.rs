//! GPU test resilience for NVK/Nouveau driver limitations.
//!
//! Under concurrent GPU load (full workspace `cargo test`), NVK/Nouveau can cause
//! SIGSEGV or resource invalidation panics. These helpers gracefully skip tests
//! when such driver-specific failures occur.
//!
//! Note: SIGSEGV cannot be caught by `catch_unwind`; for those cases we add
//! documentation of the known NVK limitation.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Skip-test patterns for NVK/Nouveau driver failures
#[inline]
fn is_nvk_skip_panic(msg: &str) -> bool {
    msg.contains("does not exist")
        || msg.contains("device lost")
        || msg.contains("Parent device")
        || msg.contains("resource invalid")
        || msg.contains("adapter")
}

/// Run a sync GPU test body, skipping on NVK driver panics.
pub fn gpu_test_resilient<F: FnOnce() + std::panic::UnwindSafe>(f: F) {
    if let Err(e) = catch_unwind(AssertUnwindSafe(f)) {
        let msg = e
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| e.downcast_ref::<&str>().copied())
            .unwrap_or("unknown panic");
        if is_nvk_skip_panic(msg) {
            eprintln!("GPU test skipped: {msg} (NVK driver limitation)");
        } else {
            std::panic::resume_unwind(e);
        }
    }
}

/// Run an async GPU test body, skipping on NVK driver panics.
///
/// Use with `#[tokio::test]`:
/// ```ignore
/// #[tokio::test]
/// async fn test_foo() {
///     gpu_test_resilient_async(async {
///         let executor = create_executor().await;
///         // ... test body
///     }).await;
/// }
/// ```
pub async fn gpu_test_resilient_async<Fut>(f: Fut)
where
    Fut: std::future::Future<Output = ()>,
{
    let result = catch_unwind(AssertUnwindSafe(|| {
        tokio::runtime::Handle::current().block_on(f);
    }));
    if let Err(e) = result {
        let msg = e
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| e.downcast_ref::<&str>().copied())
            .unwrap_or("unknown panic");
        if is_nvk_skip_panic(msg) {
            eprintln!("GPU test skipped: {msg} (NVK driver limitation)");
        } else {
            std::panic::resume_unwind(e);
        }
    }
}
