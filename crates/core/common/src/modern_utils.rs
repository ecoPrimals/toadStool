// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Modern Utility Helpers
//!
//! Common utilities using modern Rust patterns that can be used throughout the codebase.
//! These demonstrate and provide reusable implementations of best practices.

use std::borrow::Cow;
use std::future::Future;
use std::time::Duration;

/// Result type for utility operations
pub type UtilResult<T> = Result<T, UtilError>;

/// Utility error types
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UtilError {
    /// Operation exceeded the specified duration
    #[error("Operation timeout after {0:?}")]
    Timeout(Duration),

    /// Operation failed with the given message
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    /// Input was invalid
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Execute an operation with a timeout
///
/// This is a common pattern for ensuring operations don't hang indefinitely.
///
/// # Errors
///
/// Returns `UtilError::Timeout` if the operation exceeds the specified duration.
///
/// # Examples
///
/// ```rust,no_run
/// use std::time::Duration;
/// use toadstool_common::modern_utils::with_timeout;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = with_timeout(
///     Duration::from_secs(5),
///     async { expensive_operation().await }
/// ).await?;
/// # Ok(())
/// # }
/// # async fn expensive_operation() -> Result<String, Box<dyn std::error::Error>> { Ok("done".to_string()) }
/// ```
pub async fn with_timeout<F, T>(duration: Duration, future: F) -> UtilResult<T>
where
    F: Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_or_else(|_| Err(UtilError::Timeout(duration)), Ok)
}

/// Retry an operation with exponential backoff
///
/// Modern pattern for handling transient failures.
///
/// # Errors
///
/// Returns the error from the last attempt if all retries are exhausted.
///
/// # Examples
///
/// ```rust,no_run
/// use toadstool_common::modern_utils::retry_with_backoff;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = retry_with_backoff(3, || async {
///     // Operation that might fail
///     Ok::<_, std::io::Error>(())
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn retry_with_backoff<F, Fut, T, E>(max_attempts: usize, mut operation: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    const INITIAL_BACKOFF_MS: u64 = 100;
    const MAX_BACKOFF_SECS: u64 = 30;
    let mut attempt = 0;
    let mut delay = Duration::from_millis(INITIAL_BACKOFF_MS);

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt >= max_attempts => return Err(e),
            Err(_) => {
                tokio::time::sleep(delay).await;
                delay = delay.saturating_mul(2).min(Duration::from_secs(MAX_BACKOFF_SECS));
            }
        }
    }
}

/// Zero-copy string handling with Cow
///
/// Use this when you might need to own or borrow a string.
///
/// # Examples
///
/// ```rust
/// use toadstool_common::modern_utils::maybe_clone_str;
///
/// let borrowed = maybe_clone_str("short", 10);
/// assert!(matches!(borrowed, std::borrow::Cow::Borrowed(_)));
///
/// let owned = maybe_clone_str("this is a very long string", 10);
/// assert!(matches!(owned, std::borrow::Cow::Owned(_)));
/// ```
#[must_use]
pub fn maybe_clone_str(s: &str, max_borrow_len: usize) -> Cow<'_, str> {
    if s.len() <= max_borrow_len {
        Cow::Borrowed(s)
    } else {
        // For very long strings, we might want to truncate or process
        Cow::Owned(s.to_string())
    }
}

/// Safe division with error handling (no panics)
///
/// # Errors
///
/// Returns `UtilError::InvalidInput` if the denominator is zero.
///
/// # Examples
///
/// ```rust
/// use toadstool_common::modern_utils::safe_divide;
///
/// assert_eq!(safe_divide(10, 2), Ok(5));
/// assert!(safe_divide(10, 0).is_err());
/// ```
pub fn safe_divide(numerator: i64, denominator: i64) -> UtilResult<i64> {
    if denominator == 0 {
        return Err(UtilError::InvalidInput("Division by zero".to_string()));
    }
    Ok(numerator / denominator)
}

/// Safe percentage calculation (no panics, handles overflow)
///
/// # Errors
///
/// Returns `UtilError::InvalidInput` if the total is zero.
///
/// # Examples
///
/// ```rust
/// use toadstool_common::modern_utils::safe_percentage;
///
/// assert_eq!(safe_percentage(50, 100), Ok(50.0));
/// let result = safe_percentage(1, 3).unwrap();
/// assert!((result - 33.33).abs() < 0.01); // Floating point comparison
/// assert!(safe_percentage(1, 0).is_err());
/// ```
#[expect(
    clippy::cast_precision_loss,
    reason = "precision loss acceptable for this conversion"
)] // Intentional for percentage calculation
pub fn safe_percentage(part: u64, total: u64) -> UtilResult<f64> {
    if total == 0 {
        return Err(UtilError::InvalidInput("Total cannot be zero".to_string()));
    }
    Ok((part as f64 / total as f64) * 100.0)
}

/// Batch process items with a maximum batch size
///
/// Modern pattern for processing large datasets efficiently.
///
/// # Errors
///
/// Returns the first error encountered during batch processing.
///
/// # Examples
///
/// ```rust
/// use toadstool_common::modern_utils::batch_process;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
/// let results = batch_process(items, 3, |batch| async move {
///     // Process batch
///     Ok::<_, std::io::Error>(batch.len())
/// }).await?;
/// assert_eq!(results, vec![3, 3, 3, 1]); // 4 batches
/// # Ok(())
/// # }
/// ```
pub async fn batch_process<T, F, Fut, R, E>(
    items: Vec<T>,
    batch_size: usize,
    mut processor: F,
) -> Result<Vec<R>, E>
where
    F: FnMut(Vec<T>) -> Fut,
    Fut: Future<Output = Result<R, E>>,
{
    let mut results = Vec::new();
    let mut batch = Vec::with_capacity(batch_size);

    for item in items {
        batch.push(item);

        if batch.len() >= batch_size {
            let result = processor(std::mem::replace(
                &mut batch,
                Vec::with_capacity(batch_size),
            ))
            .await?;
            results.push(result);
        }
    }

    // Process remaining items
    if !batch.is_empty() {
        let result = processor(batch).await?;
        results.push(result);
    }

    Ok(results)
}

/// Clamp a value between min and max (no panics)
///
/// # Examples
///
/// ```rust
/// use toadstool_common::modern_utils::clamp;
///
/// assert_eq!(clamp(5, 0, 10), 5);
/// assert_eq!(clamp(-5, 0, 10), 0);
/// assert_eq!(clamp(15, 0, 10), 10);
/// ```
#[must_use]
pub fn clamp<T: Ord>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Normalize a value to a 0-1 range
///
/// # Errors
///
/// Returns `UtilError::InvalidInput` if the range (max - min) is zero.
///
/// # Examples
///
/// ```rust
/// use toadstool_common::modern_utils::normalize;
///
/// assert_eq!(normalize(5.0, 0.0, 10.0), Ok(0.5));
/// assert_eq!(normalize(0.0, 0.0, 10.0), Ok(0.0));
/// assert_eq!(normalize(10.0, 0.0, 10.0), Ok(1.0));
/// ```
pub fn normalize(value: f64, min: f64, max: f64) -> UtilResult<f64> {
    if (max - min).abs() < f64::EPSILON {
        return Err(UtilError::InvalidInput("Range cannot be zero".to_string()));
    }
    Ok((value - min) / (max - min))
}

/// Linear interpolation between two values
///
/// # Examples
///
/// ```rust
/// use toadstool_common::modern_utils::lerp;
///
/// assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
/// assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
/// assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
/// ```
#[must_use]
pub fn lerp(start: f64, end: f64, t: f64) -> f64 {
    let clamped_t = t.clamp(0.0, 1.0);
    (end - start).mul_add(clamped_t, start)
}

/// Check if a value is within a range (inclusive)
///
/// # Examples
///
/// ```rust
/// use toadstool_common::modern_utils::in_range;
///
/// assert!(in_range(&5, &0, &10));
/// assert!(in_range(&0, &0, &10));
/// assert!(in_range(&10, &0, &10));
/// assert!(!in_range(&11, &0, &10));
/// ```
#[must_use]
pub fn in_range<T: Ord>(value: &T, min: &T, max: &T) -> bool {
    value >= min && value <= max
}

#[cfg(test)]
#[path = "modern_utils_tests.rs"]
mod tests;
