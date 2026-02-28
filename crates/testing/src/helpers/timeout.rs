//! Timeout helpers for testing
//!
//! Provides consistent timeout handling across all async tests,
//! ensuring no test hangs indefinitely.

use std::future::Future;
use std::time::Duration;
use tokio::time::timeout;

/// Default test timeout (5 seconds)
pub const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Short timeout for fast operations (1 second)
pub const SHORT_TIMEOUT: Duration = Duration::from_secs(1);

/// Long timeout for complex operations (10 seconds)
pub const LONG_TIMEOUT: Duration = Duration::from_secs(10);

/// Chaos test timeout (20 seconds; chaos tests are serialized and may need more)
pub const CHAOS_TIMEOUT: Duration = Duration::from_secs(20);

/// Error type for timeout operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutError {
    Elapsed,
    OperationFailed,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::Elapsed => write!(f, "operation timed out"),
            TimeoutError::OperationFailed => write!(f, "operation failed"),
        }
    }
}

impl std::error::Error for TimeoutError {}

/// Run a future with default timeout
pub async fn with_default_timeout<F, T>(future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    timeout(DEFAULT_TEST_TIMEOUT, future)
        .await
        .map_err(|_| TimeoutError::Elapsed)
}

/// Run a future with short timeout
pub async fn with_short_timeout<F, T>(future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    timeout(SHORT_TIMEOUT, future)
        .await
        .map_err(|_| TimeoutError::Elapsed)
}

/// Run a future with long timeout
pub async fn with_long_timeout<F, T>(future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    timeout(LONG_TIMEOUT, future)
        .await
        .map_err(|_| TimeoutError::Elapsed)
}

/// Run a future with chaos test timeout
pub async fn with_chaos_timeout<F, T>(future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    timeout(CHAOS_TIMEOUT, future)
        .await
        .map_err(|_| TimeoutError::Elapsed)
}

/// Run a future with custom timeout
pub async fn with_timeout_duration<F, T>(future: F, duration: Duration) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    timeout(duration, future)
        .await
        .map_err(|_| TimeoutError::Elapsed)
}

/// Macro to wrap test with automatic timeout
#[macro_export]
macro_rules! test_with_timeout {
    ($test_fn:expr) => {
        $crate::helpers::timeout::with_default_timeout($test_fn)
            .await
            .expect("test timed out")
    };
}

/// Macro for short timeout tests
#[macro_export]
macro_rules! test_with_short_timeout {
    ($test_fn:expr) => {
        $crate::helpers::timeout::with_short_timeout($test_fn)
            .await
            .expect("test timed out")
    };
}

/// Macro for long timeout tests
#[macro_export]
macro_rules! test_with_long_timeout {
    ($test_fn:expr) => {
        $crate::helpers::timeout::with_long_timeout($test_fn)
            .await
            .expect("test timed out")
    };
}

/// Helper to retry an operation with timeout
pub async fn retry_with_timeout<F, T, E>(
    mut operation: F,
    max_attempts: usize,
    timeout_per_attempt: Duration,
) -> Result<T, RetryError<E>>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Debug,
{
    for attempt in 1..=max_attempts {
        match timeout(timeout_per_attempt, operation()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) if attempt == max_attempts => {
                return Err(RetryError::Failed(e));
            }
            Ok(Err(_)) => continue,
            Err(_) if attempt == max_attempts => {
                return Err(RetryError::Timeout);
            }
            Err(_) => continue,
        }
    }
    Err(RetryError::MaxAttemptsExceeded)
}

#[derive(Debug)]
pub enum RetryError<E> {
    Timeout,
    Failed(E),
    MaxAttemptsExceeded,
}

impl<E: std::fmt::Display> std::fmt::Display for RetryError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetryError::Timeout => write!(f, "operation timed out"),
            RetryError::Failed(e) => write!(f, "operation failed: {e}"),
            RetryError::MaxAttemptsExceeded => write!(f, "max retry attempts exceeded"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for RetryError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RetryError::Failed(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_with_default_timeout_success() {
        let result = with_default_timeout(async { 42 }).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_with_default_timeout_failure() {
        let result = with_default_timeout(std::future::pending::<()>()).await;
        assert_eq!(result, Err(TimeoutError::Elapsed));
    }

    #[tokio::test]
    async fn test_with_short_timeout() {
        let result = with_short_timeout(async { 42 }).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_with_long_timeout() {
        let result = with_long_timeout(async { 42 }).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_timeout_macro() {
        let result = test_with_timeout!(async { 42 });
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_retry_with_timeout_success() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = Arc::clone(&attempts);

        let result = retry_with_timeout(
            move || {
                let current = attempts_clone.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move {
                    if current < 2 {
                        Err("not yet")
                    } else {
                        Ok(42)
                    }
                })
            },
            5,
            Duration::from_millis(100),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
