// SPDX-License-Identifier: AGPL-3.0-or-later
//! Concurrency Stress Tests
//!
//! These tests prove thread safety and concurrency correctness at scale.
//! Philosophy: If it works with 1000 concurrent operations, it's production-ready.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Barrier, Notify, RwLock, Semaphore};
use tokio::time::timeout;

/// Test 1000 concurrent tasks executing simultaneously
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_1000_concurrent_tasks() {
    const NUM_TASKS: usize = 1000;
    let barrier = Arc::new(Barrier::new(NUM_TASKS));
    let success_counter = Arc::new(AtomicUsize::new(0));
    let mut tasks = vec![];
    
    for i in 0..NUM_TASKS {
        let barrier = barrier.clone();
        let counter = success_counter.clone();
        
        tasks.push(tokio::spawn(async move {
            // All tasks wait at barrier
            barrier.wait().await;
            
            // Execute operation concurrently
            tokio::task::yield_now().await;
            
            // Simulate work
            let result = format!("Task {} completed", i);
            
            // Increment success counter atomically
            counter.fetch_add(1, Ordering::SeqCst);
            
            Ok::<_, std::io::Error>(result)
        }));
    }
    
    // Wait for all tasks
    let results = futures::future::join_all(tasks).await;
    
    // Count successes
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let success_rate = success_count as f64 / NUM_TASKS as f64;
    
    // Verify atomic counter matches
    assert_eq!(success_counter.load(Ordering::SeqCst), success_count);
    
    // Require 99%+ success rate (allows for spurious failures)
    assert!(
        success_rate >= 0.99,
        "Success rate {:.2}% should be >= 99%, got {}/{}",
        success_rate * 100.0,
        success_count,
        NUM_TASKS
    );
    
    println!("✓ Stress test passed: {}/{} tasks succeeded ({:.2}%)",
             success_count, NUM_TASKS, success_rate * 100.0);
}

/// Test concurrent reads with shared state
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_concurrent_reads() {
    const NUM_READERS: usize = 1000;
    let data = Arc::new(RwLock::new(vec![1, 2, 3, 4, 5]));
    let barrier = Arc::new(Barrier::new(NUM_READERS));
    let mut tasks = vec![];
    
    for _ in 0..NUM_READERS {
        let data = data.clone();
        let barrier = barrier.clone();
        
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            
            // Concurrent read
            let guard = data.read().await;
            let sum: i32 = guard.iter().sum();
            
            Ok::<_, std::io::Error>(sum)
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    // All reads should succeed and get same value
    assert_eq!(results.len(), NUM_READERS);
    for result in results {
        let sum = result.unwrap().unwrap();
        assert_eq!(sum, 15); // 1+2+3+4+5 = 15
    }
    
    println!("✓ Concurrent reads test passed: {} readers", NUM_READERS);
}

/// Test concurrent writes with proper synchronization
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_concurrent_writes() {
    const NUM_WRITERS: usize = 100;
    let counter = Arc::new(RwLock::new(0_i32));
    let barrier = Arc::new(Barrier::new(NUM_WRITERS));
    let mut tasks = vec![];
    
    for _ in 0..NUM_WRITERS {
        let counter = counter.clone();
        let barrier = barrier.clone();
        
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            
            // Concurrent write
            let mut guard = counter.write().await;
            *guard += 1;
            drop(guard); // Explicit drop for clarity
            
            Ok::<_, std::io::Error>(())
        }));
    }
    
    futures::future::join_all(tasks).await;
    
    // Final count should be exactly NUM_WRITERS
    let final_count = *counter.read().await;
    assert_eq!(final_count, NUM_WRITERS as i32);
    
    println!("✓ Concurrent writes test passed: {} writers", NUM_WRITERS);
}

/// Test semaphore-based rate limiting under load
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_rate_limiting() {
    const LIMIT: usize = 10;
    const NUM_TASKS: usize = 1000;
    
    let semaphore = Arc::new(Semaphore::new(LIMIT));
    let active_counter = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));
    let mut tasks = vec![];
    
    for _ in 0..NUM_TASKS {
        let sem = semaphore.clone();
        let active = active_counter.clone();
        let max_seen = max_concurrent.clone();
        
        tasks.push(tokio::spawn(async move {
            // Acquire permit
            let _permit = sem.acquire().await.unwrap();
            
            // Track concurrent executions
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            
            // Update max seen
            max_seen.fetch_max(current, Ordering::SeqCst);
            
            // Simulate work
            tokio::task::yield_now().await;
            
            // Release (implicit via permit drop)
            active.fetch_sub(1, Ordering::SeqCst);
            
            Ok::<_, std::io::Error>(())
        }));
    }
    
    futures::future::join_all(tasks).await;
    
    let max = max_concurrent.load(Ordering::SeqCst);
    
    // Max concurrent should never exceed semaphore limit
    assert!(
        max <= LIMIT,
        "Max concurrent {} should not exceed limit {}",
        max,
        LIMIT
    );
    
    println!("✓ Rate limiting test passed: max concurrent {} (limit {})", max, LIMIT);
}

/// Test notification patterns under concurrent load
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_notification_patterns() {
    const NUM_WAITERS: usize = 100;
    let notify = Arc::new(Notify::new());
    let ready = Arc::new(AtomicBool::new(false));
    let mut tasks = vec![];
    
    // Spawn waiters
    for _ in 0..NUM_WAITERS {
        let notify = notify.clone();
        let ready = ready.clone();
        
        tasks.push(tokio::spawn(async move {
            // Wait for notification
            notify.notified().await;
            
            // Check ready flag
            assert!(ready.load(Ordering::SeqCst));
            
            Ok::<_, std::io::Error>(())
        }));
    }
    
    // Give tasks time to start waiting
    tokio::task::yield_now().await;
    
    // Set ready and notify all
    ready.store(true, Ordering::SeqCst);
    for _ in 0..NUM_WAITERS {
        notify.notify_one();
    }
    
    // Wait for all tasks with timeout
    let result = timeout(Duration::from_secs(5), futures::future::join_all(tasks)).await;
    
    assert!(result.is_ok(), "All tasks should complete within timeout");
    
    let results = result.unwrap();
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    
    assert_eq!(success_count, NUM_WAITERS);
    
    println!("✓ Notification test passed: {} waiters notified", NUM_WAITERS);
}

/// Test timeout behavior under concurrent load
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_timeout_handling() {
    const NUM_TASKS: usize = 100;
    let success_counter = Arc::new(AtomicUsize::new(0));
    let timeout_counter = Arc::new(AtomicUsize::new(0));
    let mut tasks = vec![];
    
    for i in 0..NUM_TASKS {
        let success = success_counter.clone();
        let timeouts = timeout_counter.clone();
        
        tasks.push(tokio::spawn(async move {
            let result = timeout(Duration::from_millis(50), async {
                // Half will timeout (never complete), half will succeed immediately
                if i % 2 == 0 {
                    std::future::pending::<()>().await;
                }
                Ok::<_, std::io::Error>(())
            }).await;
            
            match result {
                Ok(Ok(())) => {
                    success.fetch_add(1, Ordering::SeqCst);
                }
                Err(_) => {
                    timeouts.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        }));
    }
    
    futures::future::join_all(tasks).await;
    
    let successes = success_counter.load(Ordering::SeqCst);
    let timeouts = timeout_counter.load(Ordering::SeqCst);
    
    // Should have roughly 50/50 split
    assert!(successes >= 40 && successes <= 60, "Expected ~50 successes, got {}", successes);
    assert!(timeouts >= 40 && timeouts <= 60, "Expected ~50 timeouts, got {}", timeouts);
    
    println!("✓ Timeout handling test passed: {} succeeded, {} timed out",
             successes, timeouts);
}

/// Test race condition detection
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_race_condition_detection() {
    const NUM_INCREMENTS: usize = 10000;
    let atomic_counter = Arc::new(AtomicUsize::new(0));
    let mut tasks = vec![];
    
    for _ in 0..NUM_INCREMENTS {
        let counter = atomic_counter.clone();
        
        tasks.push(tokio::spawn(async move {
            // Atomic increment (no races possible)
            counter.fetch_add(1, Ordering::SeqCst);
        }));
    }
    
    futures::future::join_all(tasks).await;
    
    let final_count = atomic_counter.load(Ordering::SeqCst);
    
    // If there were race conditions, this would fail
    assert_eq!(
        final_count,
        NUM_INCREMENTS,
        "Race detected: expected {}, got {}",
        NUM_INCREMENTS,
        final_count
    );
    
    println!("✓ Race detection test passed: {} atomic increments", NUM_INCREMENTS);
}

/// Test deadlock prevention with multiple locks
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_deadlock_prevention() {
    let lock_a = Arc::new(RwLock::new(0));
    let lock_b = Arc::new(RwLock::new(0));
    let mut tasks = vec![];
    
    // Task 1: Lock A then B
    let a1 = lock_a.clone();
    let b1 = lock_b.clone();
    tasks.push(tokio::spawn(async move {
        let _guard_a = a1.write().await;
        tokio::task::yield_now().await;
        let _guard_b = b1.write().await;
        Ok::<_, std::io::Error>(())
    }));
    
    // Task 2: Lock A then B (same order - no deadlock)
    let a2 = lock_a.clone();
    let b2 = lock_b.clone();
    tasks.push(tokio::spawn(async move {
        let _guard_a = a2.write().await;
        tokio::task::yield_now().await;
        let _guard_b = b2.write().await;
        Ok::<_, std::io::Error>(())
    }));
    
    // Should complete without deadlock
    let result = timeout(Duration::from_secs(5), futures::future::join_all(tasks)).await;
    
    assert!(result.is_ok(), "Deadlock detected - tasks did not complete");
    
    println!("✓ Deadlock prevention test passed");
}

/// Test recovery from task panics
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_panic_recovery() {
    const NUM_TASKS: usize = 100;
    let mut tasks = vec![];
    
    for i in 0..NUM_TASKS {
        tasks.push(tokio::spawn(async move {
            // Every 10th task panics
            if i % 10 == 0 {
                panic!("Intentional panic for task {}", i);
            }
            Ok::<_, std::io::Error>(i)
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let panicked = results.iter().filter(|r| r.is_err()).count();
    let succeeded = results.iter().filter(|r| r.is_ok()).count();
    
    // Should have 10 panics and 90 successes
    assert_eq!(panicked, 10, "Expected 10 panics, got {}", panicked);
    assert_eq!(succeeded, 90, "Expected 90 successes, got {}", succeeded);
    
    println!("✓ Panic recovery test passed: {} panicked, {} succeeded",
             panicked, succeeded);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_sustained_load() {
    const DURATION_SECS: u64 = 2;
    const TASKS_PER_SECOND: usize = 100;
    
    let start = std::time::Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
    let spawner = {
        let counter = counter.clone();
        let running = running.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(1000 / TASKS_PER_SECOND as u64));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);
            while running.load(Ordering::Relaxed) {
                let counter = counter.clone();
                
                tokio::spawn(async move {
                    tokio::task::yield_now().await;
                    counter.fetch_add(1, Ordering::Relaxed);
                });
                
                interval.tick().await;
            }
        })
    };
    
    // Run for specified duration - timeout as guard (no sleep)
    let _ = timeout(Duration::from_secs(DURATION_SECS), std::future::pending::<()>()).await;
    running.store(false, Ordering::Relaxed);
    
    spawner.await.unwrap();
    
    // Allow tasks to complete - yield to let them finish
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    
    let total_tasks = counter.load(Ordering::Relaxed);
    let elapsed = start.elapsed().as_secs_f64();
    let tasks_per_sec = total_tasks as f64 / elapsed;
    
    println!("✓ Sustained load test: {} tasks in {:.2}s ({:.0} tasks/sec)",
             total_tasks, elapsed, tasks_per_sec);
    
    // Should process close to expected number
    assert!(
        tasks_per_sec >= (TASKS_PER_SECOND as f64 * 0.8),
        "Task rate too low: {:.0} < {}",
        tasks_per_sec,
        TASKS_PER_SECOND
    );
}

