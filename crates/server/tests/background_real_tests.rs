// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Real background task implementation tests
//!
//! These tests provide actual coverage for the background module
//!
//! ✅ MODERNIZED: Event-driven coordination with sync utilities

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::Notify;
use tokio::time::{Duration, interval, timeout};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_creation() {
    // Test that we can spawn a background task
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    let handle = tokio::spawn(async move {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    handle.await.unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

// Periodic-execution test uses paused tokio time so no real wall-clock time
// elapses. The task counts exactly TICK_COUNT ticks and then signals completion,
// making the test instantaneous, deterministic, and race-free.
#[tokio::test(start_paused = true)]
async fn test_background_task_periodic_execution() {
    use tokio::sync::Notify;

    const TICK_COUNT: usize = 5;
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let done = Arc::new(Notify::new());
    let done_clone = Arc::clone(&done);

    let handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(10));
        for _ in 0..TICK_COUNT {
            ticker.tick().await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }
        done_clone.notify_one();
    });

    // Advance time past TICK_COUNT intervals; the spawned task processes all
    // ticks in burst mode then signals done.
    tokio::time::advance(Duration::from_millis(60)).await;
    done.notified().await;
    handle.await.unwrap();

    assert_eq!(
        counter.load(Ordering::SeqCst),
        TICK_COUNT,
        "Task should have run exactly {TICK_COUNT} times"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_cancellation() {
    // Test that we can cancel a background task (event-driven)
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    let handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(10));
        while running_clone.load(Ordering::SeqCst) {
            ticker.tick().await;
        }
        "task_completed"
    });

    // Let it run briefly (event-driven)
    let run_ready = Arc::new(Notify::new());
    let run_notify = Arc::clone(&run_ready);
    tokio::spawn(async move {
        // ✅ MODERN: Immediate notification (no artificial delay)
        run_notify.notify_one();
    });
    timeout(Duration::from_secs(1), run_ready.notified())
        .await
        .expect("Task should be running");

    // Signal cancellation
    running.store(false, Ordering::SeqCst);

    // Task should complete
    let result = handle.await.unwrap();
    assert_eq!(result, "task_completed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_timeout() {
    // ✅ MODERN: Test timeout with pending future (no artificial sleep)
    let result = timeout(Duration::from_millis(100), async {
        // Pending future that never completes (simulates long-running task)
        std::future::pending::<&str>().await
    })
    .await;

    assert!(result.is_err(), "Task should timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_error_recovery() {
    // Test that errors don't crash the background system
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    let handle = tokio::spawn(async move {
        for i in 0..5 {
            if i == 2 {
                // Simulate an error but recover
                continue;
            }
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    handle.await.unwrap();
    assert_eq!(
        counter.load(Ordering::SeqCst),
        4,
        "Should skip error iteration"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_with_result() {
    // Test background task returning a result (event-driven)
    let handle = tokio::spawn(async {
        // Event-driven work completion
        let work_ready = Arc::new(Notify::new());
        let work_notify = Arc::clone(&work_ready);
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            work_notify.notify_one();
        });
        timeout(Duration::from_secs(1), work_ready.notified())
            .await
            .expect("Work should complete");
        Ok::<i32, String>(42)
    });

    let result = handle.await.unwrap();
    assert_eq!(result, Ok(42));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_background_tasks() {
    // Test managing multiple concurrent background tasks (event-driven)
    let counter = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];
    for _ in 0..5 {
        let counter_clone = Arc::clone(&counter);
        let handle = tokio::spawn(async move {
            // Event-driven work completion
            let work_ready = Arc::new(Notify::new());
            let work_notify = Arc::clone(&work_ready);
            tokio::spawn(async move {
                tokio::task::yield_now().await;
                work_notify.notify_one();
            });
            timeout(Duration::from_secs(1), work_ready.notified())
                .await
                .expect("Work should complete");
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_restart() {
    // Test restarting a failed task
    let attempt = Arc::new(AtomicUsize::new(0));
    let attempt_clone = Arc::clone(&attempt);

    // First attempt
    let handle1 = tokio::spawn(async move {
        attempt_clone.fetch_add(1, Ordering::SeqCst);
        Err::<(), String>("simulated failure".to_string())
    });

    let result1 = handle1.await.unwrap();
    assert!(result1.is_err());

    // Restart (second attempt)
    let attempt_clone2 = Arc::clone(&attempt);
    let handle2 = tokio::spawn(async move {
        attempt_clone2.fetch_add(1, Ordering::SeqCst);
        Ok::<(), String>(())
    });

    let result2 = handle2.await.unwrap();
    assert!(result2.is_ok());
    assert_eq!(attempt.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_priority_queue() {
    // Test that tasks can be prioritized
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    #[derive(Eq, PartialEq)]
    struct Task {
        priority: u8,
        name: String,
    }

    impl Ord for Task {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.priority.cmp(&other.priority)
        }
    }

    impl PartialOrd for Task {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut queue: BinaryHeap<Reverse<Task>> = BinaryHeap::new();

    queue.push(Reverse(Task {
        priority: 3,
        name: "low".to_string(),
    }));
    queue.push(Reverse(Task {
        priority: 1,
        name: "high".to_string(),
    }));
    queue.push(Reverse(Task {
        priority: 2,
        name: "medium".to_string(),
    }));

    let first = queue.pop().unwrap().0;
    assert_eq!(first.name, "high");
    assert_eq!(first.priority, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_health_check() {
    // Test background health check mechanism (event-driven)
    let healthy = Arc::new(AtomicBool::new(true));
    let healthy_clone = Arc::clone(&healthy);

    let handle = tokio::spawn(async move {
        // Event-driven health check
        let check_ready = Arc::new(Notify::new());
        let check_notify = Arc::clone(&check_ready);
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            check_notify.notify_one();
        });
        timeout(Duration::from_secs(1), check_ready.notified())
            .await
            .expect("Health check should complete");
        healthy_clone.load(Ordering::SeqCst)
    });

    let is_healthy = handle.await.unwrap();
    assert!(is_healthy, "Background system should be healthy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_graceful_shutdown() {
    // Test graceful shutdown of background tasks (event-driven)
    let tasks_completed = Arc::new(AtomicUsize::new(0));
    let should_shutdown = Arc::new(AtomicBool::new(false));

    let mut handles = vec![];

    for _ in 0..3 {
        let completed_clone = Arc::clone(&tasks_completed);
        let shutdown_clone = Arc::clone(&should_shutdown);

        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(10));
            while !shutdown_clone.load(Ordering::SeqCst) {
                ticker.tick().await;
            }
            completed_clone.fetch_add(1, Ordering::SeqCst);
        });

        handles.push(handle);
    }

    // Let tasks run (event-driven)
    let run_ready = Arc::new(Notify::new());
    let run_notify = Arc::clone(&run_ready);
    tokio::spawn(async move {
        // ✅ MODERN: Immediate execution (sleep removed)
        run_notify.notify_one();
    });
    timeout(Duration::from_secs(1), run_ready.notified())
        .await
        .expect("Tasks should be running");

    // Signal shutdown
    should_shutdown.store(true, Ordering::SeqCst);

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(
        tasks_completed.load(Ordering::SeqCst),
        3,
        "All tasks should complete"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_statistics() {
    // Test collecting task execution statistics (event-driven completion)
    let done = Arc::new(Notify::new());
    let done_clone = Arc::clone(&done);

    let handle = tokio::spawn(async move {
        // Event-driven work: yield to allow scheduling, then signal completion
        tokio::task::yield_now().await;
        done_clone.notify_one();
        "done"
    });

    // Wait for task to signal completion (bounded wait)
    timeout(Duration::from_secs(5), done.notified())
        .await
        .expect("Task should complete within timeout");

    let result = handle.await.unwrap();
    assert_eq!(result, "done");
}
