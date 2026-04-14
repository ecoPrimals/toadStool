// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[tokio::test]
async fn test_with_timeout_success() {
    let result = with_timeout(Duration::from_secs(1), async {}).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_with_timeout_failure() {
    let result = with_timeout(Duration::from_millis(10), std::future::pending::<()>()).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_retry_with_backoff_success() {
    let mut attempts = 0;
    let result = retry_with_backoff(3, || {
        attempts += 1;
        async move { Ok::<_, std::io::Error>(attempts) }
    })
    .await;

    assert_eq!(result.unwrap(), 1);
}

#[tokio::test]
async fn test_retry_with_backoff_eventual_success() {
    let mut attempts = 0;
    let result = retry_with_backoff(3, || {
        attempts += 1;
        async move {
            if attempts < 3 {
                Err(std::io::Error::other("temporary"))
            } else {
                Ok(attempts)
            }
        }
    })
    .await;

    assert_eq!(result.unwrap(), 3);
}

#[test]
fn test_maybe_clone_str_borrow() {
    let result = maybe_clone_str("short", 10);
    assert!(matches!(result, Cow::Borrowed(_)));
}

#[test]
fn test_maybe_clone_str_owned() {
    let result = maybe_clone_str("this is very long", 5);
    assert!(matches!(result, Cow::Owned(_)));
}

#[test]
fn test_safe_divide() {
    assert_eq!(safe_divide(10, 2).unwrap(), 5);
    assert_eq!(safe_divide(10, 3).unwrap(), 3);
    assert!(safe_divide(10, 0).is_err());
}

#[test]
fn test_safe_percentage() {
    assert!((safe_percentage(50, 100).unwrap() - 50.0).abs() < f64::EPSILON);
    assert!((safe_percentage(1, 4).unwrap() - 25.0).abs() < f64::EPSILON);
    assert!(safe_percentage(1, 0).is_err());
}

#[tokio::test]
async fn test_batch_process() {
    let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let results = batch_process(items, 3, |batch| async move {
        Ok::<_, std::io::Error>(batch.len())
    })
    .await
    .unwrap();

    assert_eq!(results, vec![3, 3, 3, 1]);
}

#[test]
fn test_clamp() {
    assert_eq!(clamp(5, 0, 10), 5);
    assert_eq!(clamp(-5, 0, 10), 0);
    assert_eq!(clamp(15, 0, 10), 10);
    assert_eq!(clamp(0, 0, 10), 0);
    assert_eq!(clamp(10, 0, 10), 10);
}

#[test]
fn test_normalize() {
    assert!((normalize(5.0, 0.0, 10.0).unwrap() - 0.5).abs() < f64::EPSILON);
    assert!((normalize(0.0, 0.0, 10.0).unwrap() - 0.0).abs() < f64::EPSILON);
    assert!((normalize(10.0, 0.0, 10.0).unwrap() - 1.0).abs() < f64::EPSILON);
    assert!(normalize(5.0, 0.0, 0.0).is_err());
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
fn test_lerp() {
    assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
    assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
    assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
    assert_eq!(lerp(0.0, 10.0, 1.5), 10.0); // Clamped
}

#[test]
fn test_in_range() {
    assert!(in_range(&5, &0, &10));
    assert!(in_range(&0, &0, &10));
    assert!(in_range(&10, &0, &10));
    assert!(!in_range(&-1, &0, &10));
    assert!(!in_range(&11, &0, &10));
}

#[tokio::test]
async fn test_retry_with_backoff_exhaustion() {
    let mut attempts = 0;
    let result = retry_with_backoff(3, || {
        attempts += 1;
        async move { Err::<i32, std::io::Error>(std::io::Error::other("persistent")) }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(attempts, 3);
}

#[tokio::test]
async fn test_retry_with_backoff_single_attempt() {
    let result = retry_with_backoff(1, || async { Err::<i32, &str>("fail") }).await;
    assert!(result.is_err());
}

#[test]
fn test_util_error_display() {
    let timeout_err = UtilError::Timeout(Duration::from_secs(5));
    let display = format!("{timeout_err}");
    assert!(display.contains("timeout") || display.contains('5'));

    let op_err = UtilError::OperationFailed("test op failed".to_string());
    let display = format!("{op_err}");
    assert!(display.contains("test op failed"));

    let input_err = UtilError::InvalidInput("bad value".to_string());
    let display = format!("{input_err}");
    assert!(display.contains("bad value"));
}

#[test]
fn test_util_error_debug() {
    let err = UtilError::InvalidInput("x".to_string());
    let debug_str = format!("{err:?}");
    assert!(!debug_str.is_empty());
}

#[tokio::test]
async fn test_with_timeout_returns_timeout_error() {
    let result = with_timeout(Duration::from_millis(10), std::future::pending::<()>()).await;

    assert!(matches!(result, Err(UtilError::Timeout(_))));
}

#[test]
fn test_safe_divide_error_variant() {
    let err = safe_divide(10, 0).unwrap_err();
    assert!(matches!(err, UtilError::InvalidInput(_)));
}

#[test]
fn test_safe_percentage_error_variant() {
    let err = safe_percentage(1, 0).unwrap_err();
    assert!(matches!(err, UtilError::InvalidInput(_)));
}

#[test]
fn test_normalize_error_variant() {
    let err = normalize(5.0, 0.0, 0.0).unwrap_err();
    assert!(matches!(err, UtilError::InvalidInput(_)));
}

#[tokio::test]
async fn test_batch_process_empty() {
    let items: Vec<i32> = vec![];
    let results = batch_process(items, 3, |batch| async move {
        Ok::<_, std::io::Error>(batch.len())
    })
    .await
    .unwrap();

    assert!(results.is_empty());
}

#[tokio::test]
async fn test_batch_process_single_item() {
    let items = vec![42];
    let results = batch_process(items, 5, |batch| async move {
        Ok::<_, std::io::Error>(batch[0])
    })
    .await
    .unwrap();

    assert_eq!(results, vec![42]);
}

#[tokio::test]
async fn test_batch_process_exact_batch_size() {
    let items = vec![1, 2, 3];
    let results = batch_process(items, 3, |batch| async move {
        Ok::<_, std::io::Error>(batch.iter().sum::<i32>())
    })
    .await
    .unwrap();

    assert_eq!(results, vec![6]);
}

#[tokio::test]
async fn test_batch_process_error_propagates() {
    let items = vec![1, 2, 3, 4, 5];
    let result = batch_process(items, 2, |batch| async move {
        if batch.contains(&3) {
            Err(std::io::Error::other("batch 2 failed"))
        } else {
            Ok(batch.len())
        }
    })
    .await;

    assert!(result.is_err());
}

#[test]
fn test_maybe_clone_str_boundary() {
    // Exactly at boundary - len 10, max_borrow 10 -> borrow
    let result = maybe_clone_str("0123456789", 10);
    assert!(matches!(result, Cow::Borrowed(_)));

    // One over boundary -> owned
    let result = maybe_clone_str("01234567890", 10);
    assert!(matches!(result, Cow::Owned(_)));
}
