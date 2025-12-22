//! Concurrent testing helpers
//!
//! This module provides modern, signal-based synchronization primitives for tests,
//! replacing arbitrary sleep() calls with proper concurrent coordination.
//!
//! # Philosophy
//! **"Test issues ARE production issues. We test concurrently because we run concurrently."**
//!
//! # Modern Patterns
//! - Event-driven coordination (channels, barriers, notifiers)
//! - Exponential backoff for polling (when absolutely necessary)
//! - Yield-first checking (minimal latency)
//! - Timeout-bounded operations (bounded waiting)

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Barrier, Mutex, Notify, RwLock};
use tokio::time::timeout;

/// Test barrier for coordinating multiple concurrent test tasks
#[derive(Clone)]
pub struct TestBarrier {
    barrier: Arc<Barrier>,
}

impl TestBarrier {
    pub fn new(n: usize) -> Self {
        Self {
            barrier: Arc::new(Barrier::new(n)),
        }
    }

    pub async fn wait(&self) {
        self.barrier.wait().await;
    }
}

/// Test notification primitive for signal-based coordination
#[derive(Clone)]
pub struct TestNotify {
    notify: Arc<Notify>,
}

impl TestNotify {
    pub fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn notify_one(&self) {
        self.notify.notify_one();
    }

    pub fn notify_waiters(&self) {
        self.notify.notify_waiters();
    }

    pub async fn notified(&self) {
        self.notify.notified().await;
    }

    /// Wait for notification with timeout
    pub async fn notified_timeout(
        &self,
        duration: Duration,
    ) -> Result<(), tokio::time::error::Elapsed> {
        timeout(duration, self.notified()).await
    }
}

impl Default for TestNotify {
    fn default() -> Self {
        Self::new()
    }
}

/// Test channel for message passing between concurrent tasks
pub struct TestChannel<T> {
    tx: mpsc::Sender<T>,
    rx: Arc<Mutex<mpsc::Receiver<T>>>,
}

impl<T> TestChannel<T> {
    pub fn new(buffer: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer);
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
        }
    }

    pub fn sender(&self) -> mpsc::Sender<T> {
        self.tx.clone()
    }

    pub async fn recv(&self) -> Option<T> {
        self.rx.lock().await.recv().await
    }

    pub async fn recv_timeout(
        &self,
        duration: Duration,
    ) -> Result<Option<T>, tokio::time::error::Elapsed> {
        timeout(duration, self.recv()).await
    }
}

impl<T> Clone for TestChannel<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: Arc::clone(&self.rx),
        }
    }
}

/// Shared state for concurrent testing
pub struct TestState<T> {
    state: Arc<RwLock<T>>,
}

impl<T> TestState<T> {
    pub fn new(initial: T) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
        }
    }

    pub async fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.state.read().await;
        f(&guard)
    }

    pub async fn write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.state.write().await;
        f(&mut guard)
    }

    pub async fn read_timeout<F, R>(
        &self,
        duration: Duration,
        f: F,
    ) -> Result<R, tokio::time::error::Elapsed>
    where
        F: FnOnce(&T) -> R,
    {
        let guard = timeout(duration, self.state.read()).await?;
        Ok(f(&guard))
    }

    pub async fn write_timeout<F, R>(
        &self,
        duration: Duration,
        f: F,
    ) -> Result<R, tokio::time::error::Elapsed>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = timeout(duration, self.state.write()).await?;
        Ok(f(&mut guard))
    }
}

impl<T> Clone for TestState<T> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

/// Helper to wait for a condition to become true
///
/// **Modern Pattern**: Event-driven polling with exponential backoff
/// Starts with fast polls and backs off to reduce CPU usage while maintaining responsiveness.
pub async fn wait_for_condition<F>(
    mut condition: F,
    timeout_duration: Duration,
    check_interval: Duration,
) -> Result<(), WaitError>
where
    F: FnMut() -> bool,
{
    let start = tokio::time::Instant::now();
    let mut current_interval = check_interval;
    let max_interval = check_interval * 4; // Cap backoff

    while !condition() {
        if start.elapsed() > timeout_duration {
            return Err(WaitError::Timeout);
        }

        // Yield to scheduler first (zero-cost if condition is ready)
        tokio::task::yield_now().await;

        // Check again immediately after yield
        if condition() {
            return Ok(());
        }

        // Sleep with exponential backoff
        tokio::time::sleep(current_interval).await;
        current_interval = std::cmp::min(current_interval * 2, max_interval);
    }

    Ok(())
}

/// Helper to wait for an async condition to become true
///
/// **Modern Pattern**: Event-driven polling with exponential backoff
/// Optimized for async conditions with minimal latency and reduced CPU usage.
pub async fn wait_for_async_condition<F, Fut>(
    mut condition: F,
    timeout_duration: Duration,
    check_interval: Duration,
) -> Result<(), WaitError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = tokio::time::Instant::now();
    let mut current_interval = check_interval;
    let max_interval = check_interval * 4; // Cap backoff

    while !condition().await {
        if start.elapsed() > timeout_duration {
            return Err(WaitError::Timeout);
        }

        // Yield to scheduler first (zero-cost if condition is ready)
        tokio::task::yield_now().await;

        // Check again immediately after yield
        if condition().await {
            return Ok(());
        }

        // Sleep with exponential backoff
        tokio::time::sleep(current_interval).await;
        current_interval = std::cmp::min(current_interval * 2, max_interval);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitError {
    Timeout,
}

impl std::fmt::Display for WaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaitError::Timeout => write!(f, "condition did not become true within timeout"),
        }
    }
}

impl std::error::Error for WaitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_barrier_coordination() {
        let barrier = TestBarrier::new(3);
        let mut handles = vec![];

        for _ in 0..3 {
            let b = barrier.clone();
            handles.push(tokio::spawn(async move {
                // All tasks wait at barrier
                b.wait().await;
            }));
        }

        // All should complete together
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_notify_signal() {
        let notify = TestNotify::new();
        let n = notify.clone();

        let handle = tokio::spawn(async move {
            n.notified().await;
        });

        // Give task time to start waiting
        tokio::task::yield_now().await;

        notify.notify_one();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_channel_communication() {
        let channel = TestChannel::new(10);
        let sender = channel.sender();

        sender.send(42).await.unwrap();
        let result = channel.recv().await;
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_state_concurrent_access() {
        let state = TestState::new(0);
        let mut handles = vec![];

        for _ in 0..10 {
            let s = state.clone();
            handles.push(tokio::spawn(async move {
                s.write(|val| *val += 1).await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let final_val = state.read(|val| *val).await;
        assert_eq!(final_val, 10);
    }

    #[tokio::test]
    async fn test_wait_for_condition() {
        let state = Arc::new(Mutex::new(false));
        let s = Arc::clone(&state);

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            *s.lock().await = true;
        });

        wait_for_async_condition(
            || {
                let s = Arc::clone(&state);
                async move { *s.lock().await }
            },
            Duration::from_secs(1),
            Duration::from_millis(5),
        )
        .await
        .unwrap();
    }
}
