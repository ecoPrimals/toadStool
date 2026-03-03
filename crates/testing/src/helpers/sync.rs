// SPDX-License-Identifier: AGPL-3.0-or-later
//! Modern synchronization helpers for concurrent testing
//!
//! This module provides event-driven coordination primitives to replace
//! sleep-based synchronization in tests, making them faster, more reliable,
//! and truly concurrent.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::{Duration, Instant};
use toadstool::{ToadStoolError, ToadStoolResult as Result};
use tokio::sync::{broadcast, mpsc, oneshot, Barrier};
use uuid::Uuid;

/// Wait for a condition to become true with exponential backoff
///
/// # Examples
///
/// ```ignore
/// use toadstool_testing::helpers::sync::wait_for_condition;
/// use std::time::Duration;
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicU32, Ordering};
///
/// # async fn example() -> Result<()> {
/// let counter = Arc::new(AtomicU32::new(0));
/// let counter_clone = counter.clone();
/// wait_for_condition(
///     move || {
///         let counter = counter_clone.clone();
///         async move {
///             counter.fetch_add(1, Ordering::SeqCst);
///             counter.load(Ordering::SeqCst) > 10
///         }
///     },
///     Duration::from_secs(5),
///     Duration::from_millis(10),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn wait_for_condition<F, Fut>(
    mut check: F,
    timeout: Duration,
    initial_interval: Duration,
) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let deadline = Instant::now() + timeout;
    let mut interval = initial_interval;
    let max_interval = Duration::from_millis(500);

    loop {
        if check().await {
            return Ok(());
        }

        let now = Instant::now();
        if now >= deadline {
            return Err(ToadStoolError::runtime(format!(
                "Condition not met within {timeout:?}"
            )));
        }

        // Sleep for interval or remaining time, whichever is less
        let remaining = deadline.saturating_duration_since(now);
        let sleep_duration = interval.min(remaining);
        tokio::time::sleep(sleep_duration).await;

        // Exponential backoff
        interval = (interval * 2).min(max_interval);
    }
}

/// Wait for a condition with a custom error message
pub async fn wait_for_condition_with_message<F, Fut>(
    check: F,
    timeout: Duration,
    initial_interval: Duration,
    error_message: &str,
) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    wait_for_condition(check, timeout, initial_interval)
        .await
        .map_err(|e| ToadStoolError::runtime(format!("{error_message}: {e}")))
}

/// Wait for a service to become healthy
///
/// Polls a health check function until it returns true or timeout is reached.
/// The health check function receives ownership and must be cloneable.
pub async fn wait_for_service_ready<F>(check: F, timeout: Duration) -> Result<()>
where
    F: Fn() -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync,
{
    wait_for_condition(
        || async { check().await },
        timeout,
        Duration::from_millis(10),
    )
    .await
    .map_err(|e| ToadStoolError::runtime(format!("Service did not become ready: {e}")))
}

/// Wait for multiple conditions concurrently
pub async fn wait_for_all<F, Fut>(checks: Vec<F>, timeout: Duration) -> Result<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = bool> + Send,
{
    let mut tasks = vec![];

    for check in checks {
        let task = tokio::spawn(async move {
            wait_for_condition(check, timeout, Duration::from_millis(10)).await
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await
            .map_err(|e| ToadStoolError::runtime(format!("Task panicked: {e}")))??;
    }

    Ok(())
}

/// Create isolated test resources with unique names and ports
///
/// # Examples
///
/// ```no_run
/// use toadstool_testing::helpers::sync::TestIsolation;
///
/// #[tokio::test]
/// async fn my_test() {
///     let isolation = TestIsolation::new("my_test");
///     let port = isolation.get_port(0);
///     let temp_dir = &isolation.temp_dir;
///     // Test runs with isolated resources
/// } // Automatic cleanup on drop
/// ```
pub struct TestIsolation {
    /// Unique test name
    pub test_name: String,
    /// Temporary directory for this test (auto-cleaned)
    pub temp_dir: PathBuf,
    /// Base port number for this test
    pub port_base: u16,
    /// Unique test ID
    pub test_id: Uuid,
}

impl TestIsolation {
    /// Create new test isolation with unique resources
    pub fn new(test_name: &str) -> Self {
        let test_id = Uuid::new_v4();
        let temp_dir = std::env::temp_dir()
            .join("toadstool-tests")
            .join(test_name)
            .join(test_id.to_string());

        // Create temp directory
        std::fs::create_dir_all(&temp_dir).ok();

        // Allocate unique port range based on test name hash and UUID
        // This ensures different test instances get different ports
        let name_hash = test_name.bytes().map(u32::from).sum::<u32>();
        let uuid_hash = test_id.as_u128() as u32;
        let port_base = 10_000 + ((name_hash.wrapping_add(uuid_hash)) % 20_000) as u16;

        Self {
            test_name: test_name.to_string(),
            temp_dir,
            port_base,
            test_id,
        }
    }

    /// Get a unique port for this test (offset from base)
    pub fn get_port(&self, offset: u16) -> u16 {
        self.port_base.saturating_add(offset)
    }

    /// Get temporary file path within test directory
    pub fn temp_file(&self, name: &str) -> PathBuf {
        self.temp_dir.join(name)
    }

    /// Get unique resource name for this test
    pub fn resource_name(&self, prefix: &str) -> String {
        format!("{}_{}_{}", prefix, self.test_name, self.test_id)
    }
}

impl Drop for TestIsolation {
    fn drop(&mut self) {
        // Clean up test resources
        let _ = std::fs::remove_dir_all(&self.temp_dir);
    }
}

/// Event barrier for coordinating multiple async tasks
///
/// Similar to tokio::sync::Barrier but with more testing-friendly semantics
#[derive(Clone)]
pub struct TestBarrier {
    inner: std::sync::Arc<Barrier>,
}

impl TestBarrier {
    /// Create a new barrier that will block until `count` tasks reach it
    pub fn new(count: usize) -> Self {
        Self {
            inner: std::sync::Arc::new(Barrier::new(count)),
        }
    }

    /// Wait for all tasks to reach the barrier
    pub async fn wait(&self) -> tokio::sync::BarrierWaitResult {
        self.inner.wait().await
    }
}

/// Event coordinator for complex multi-phase testing
///
/// # Examples
///
/// ```ignore
/// use toadstool_testing::helpers::sync::EventCoordinator;
///
/// # async fn example() -> Result<()> {
/// let coordinator = EventCoordinator::new();
/// let mut rx = coordinator.subscribe();
///
/// // In background task
/// tokio::spawn({
///     let coordinator = coordinator.clone();
///     async move {
///         do_work().await;
///         coordinator.signal("work_done").await;
///     }
/// });
///
/// // Wait for event
/// coordinator.wait_for("work_done", std::time::Duration::from_secs(5)).await?;
/// # Ok(())
/// # }
/// # async fn do_work() {}
/// ```
#[derive(Clone)]
pub struct EventCoordinator {
    tx: broadcast::Sender<String>,
}

impl EventCoordinator {
    /// Create a new event coordinator
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { tx }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    /// Signal an event
    #[allow(clippy::unused_async)] // broadcast::Sender::send is sync; async for API consistency
    pub async fn signal(&self, event: &str) {
        let _ = self.tx.send(event.to_string());
    }

    /// Wait for a specific event with timeout
    pub async fn wait_for(&self, event: &str, timeout: Duration) -> Result<()> {
        let mut rx = self.subscribe();
        let target_event = event.to_string();

        tokio::time::timeout(timeout, async {
            loop {
                match rx.recv().await {
                    Ok(e) if e == target_event => return Ok(()),
                    Ok(_) => continue,
                    Err(_) => return Err(ToadStoolError::runtime("Event channel closed")),
                }
            }
        })
        .await
        .map_err(|_| ToadStoolError::runtime(format!("Timeout waiting for event: {event}")))?
    }
}

impl Default for EventCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Channel-based test coordination
pub struct TestChannels {
    /// One-shot channels for single events
    pub oneshot: Vec<(oneshot::Sender<()>, oneshot::Receiver<()>)>,
    /// MPSC channels for multiple messages
    pub mpsc: Vec<(mpsc::Sender<String>, mpsc::Receiver<String>)>,
}

impl TestChannels {
    /// Create new test channel set
    pub fn new() -> Self {
        Self {
            oneshot: vec![],
            mpsc: vec![],
        }
    }

    /// Add a oneshot channel
    pub fn add_oneshot(&mut self) -> (oneshot::Sender<()>, oneshot::Receiver<()>) {
        let (tx, rx) = oneshot::channel();
        self.oneshot.push((tx, rx));
        let idx = self.oneshot.len() - 1;
        let (tx, rx) = self.oneshot.swap_remove(idx);
        (tx, rx)
    }

    /// Add an MPSC channel
    pub fn add_mpsc(&mut self, buffer: usize) -> (mpsc::Sender<String>, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::channel(buffer);
        self.mpsc.push((tx, rx));
        let idx = self.mpsc.len() - 1;
        let (tx, rx) = self.mpsc.swap_remove(idx);
        (tx, rx)
    }
}

impl Default for TestChannels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wait_for_condition_success() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = wait_for_condition(
            move || {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    counter.load(Ordering::SeqCst) > 3
                }
            },
            Duration::from_secs(1),
            Duration::from_millis(10),
        )
        .await;

        assert!(result.is_ok());
        assert!(counter.load(Ordering::SeqCst) > 3);
    }

    #[tokio::test]
    async fn test_wait_for_condition_timeout() {
        let result = wait_for_condition(
            || async { false },
            Duration::from_millis(50),
            Duration::from_millis(10),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_isolation() {
        let iso1 = TestIsolation::new("test1");
        let iso2 = TestIsolation::new("test1"); // Same name

        // Different test IDs
        assert_ne!(iso1.test_id, iso2.test_id);

        // Different temp dirs
        assert_ne!(iso1.temp_dir, iso2.temp_dir);

        // Different port ranges (due to UUID hash)
        assert_ne!(iso1.port_base, iso2.port_base);
    }

    #[tokio::test]
    async fn test_event_coordinator() {
        let coordinator = EventCoordinator::new();

        tokio::spawn({
            let coordinator = coordinator.clone();
            async move {
                tokio::task::yield_now().await;
                coordinator.signal("ready").await;
            }
        });

        let result = coordinator.wait_for("ready", Duration::from_secs(1)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_barrier() {
        let barrier = TestBarrier::new(3);

        let tasks: Vec<_> = (0..3)
            .map(|_| {
                let barrier = barrier.clone();
                tokio::spawn(async move {
                    // All tasks reach barrier
                    barrier.wait().await;
                })
            })
            .collect();

        for task in tasks {
            task.await.unwrap();
        }
    }
}
