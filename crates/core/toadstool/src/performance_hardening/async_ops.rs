//! Async operation batching and optimization
//!
//! This module provides async operation batching with concurrency control
//! for improved throughput and resource utilization.

use super::types::AsyncOptimizationConfig;
use crate::{ToadStoolError, ToadStoolResult};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

/// Async operation batcher
pub struct AsyncBatcher<T, R> {
    /// Configuration
    config: AsyncOptimizationConfig,
    /// Pending operations
    pending: Arc<RwLock<Vec<BatchItem<T, R>>>>,
    /// Batch processor
    processor: Arc<dyn Fn(Vec<T>) -> futures::future::BoxFuture<'static, Vec<R>> + Send + Sync>,
    /// Semaphore for concurrency control
    semaphore: Arc<Semaphore>,
}

/// Batch item
struct BatchItem<T, R> {
    /// Input
    input: T,
    /// Response sender
    response_sender: tokio::sync::oneshot::Sender<R>,
}

impl<T, R> AsyncBatcher<T, R>
where
    T: Send + Clone + Sync + 'static,
    R: Send + 'static,
{
    /// Create new async batcher
    pub fn new<F>(config: AsyncOptimizationConfig, processor: F) -> Self
    where
        F: Fn(Vec<T>) -> futures::future::BoxFuture<'static, Vec<R>> + Send + Sync + 'static,
    {
        Self {
            config: config.clone(),
            pending: Arc::new(RwLock::new(Vec::new())),
            processor: Arc::new(processor),
            semaphore: Arc::new(Semaphore::new(config.concurrency_limit)),
        }
    }

    /// Submit operation for batching
    pub async fn submit(&self, input: T) -> ToadStoolResult<R> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let should_process = {
            let mut pending = self.pending.write().await;
            if pending.len() >= self.config.queue_size_limit {
                return Err(ToadStoolError::resource("Batch queue full".to_string()));
            }

            pending.push(BatchItem {
                input,
                response_sender: tx,
            });

            // Check if we should process batch
            pending.len() >= self.config.batch_size
        };

        // Process batch if threshold reached (spawn to avoid blocking)
        if should_process {
            let self_clone = AsyncBatcher {
                config: self.config.clone(),
                pending: Arc::clone(&self.pending),
                processor: Arc::clone(&self.processor),
                semaphore: Arc::clone(&self.semaphore),
            };
            tokio::spawn(async move {
                self_clone.process_batch().await;
            });
        }

        // Wait for response
        rx.await
            .map_err(|_| ToadStoolError::runtime("Batch operation cancelled".to_string()))
    }

    /// Process current batch
    async fn process_batch(&self) {
        // Try to acquire permit without blocking - if we can't, skip this batch
        // This prevents deadlocks in testing and production
        let Ok(_permit) = self.semaphore.try_acquire() else {
            // Semaphore at capacity - batch will be processed by another task
            return;
        };

        let batch = {
            let mut pending = self.pending.write().await;
            if pending.is_empty() {
                return;
            }

            let batch_size = pending.len().min(self.config.batch_size);
            pending.drain(..batch_size).collect::<Vec<_>>()
        };

        if batch.is_empty() {
            return;
        }

        let inputs: Vec<T> = batch.iter().map(|item| &item.input).cloned().collect();
        let results = (self.processor)(inputs).await;

        // Send results back
        for (item, result) in batch.into_iter().zip(results) {
            let _ = item.response_sender.send(result);
        }
    }

    /// Start batch processing task
    pub async fn start_batch_task(&self) {
        let pending = Arc::clone(&self.pending);
        let processor = Arc::clone(&self.processor);
        let semaphore = Arc::clone(&self.semaphore);
        let batch_timeout = self.config.batch_timeout;
        let batch_size = self.config.batch_size;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(batch_timeout);

            loop {
                interval.tick().await;

                let Ok(_permit) = semaphore.acquire().await else {
                    tracing::error!("Failed to acquire semaphore permit for batch timer");
                    continue;
                };

                let batch = {
                    let mut pending = pending.write().await;
                    if pending.is_empty() {
                        continue;
                    }

                    let batch_size = pending.len().min(batch_size);
                    pending.drain(..batch_size).collect::<Vec<_>>()
                };

                if batch.is_empty() {
                    continue;
                }

                let inputs: Vec<T> = batch.iter().map(|item| item.input.clone()).collect();
                let results = processor(inputs).await;

                // Send results back
                for (item, result) in batch.into_iter().zip(results) {
                    let _ = item.response_sender.send(result);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance_hardening::types::AsyncOptimizationConfig;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_async_batcher_new() {
        let config = AsyncOptimizationConfig {
            batch_size: 1,
            batch_timeout: Duration::from_millis(100),
            concurrency_limit: 4,
            queue_size_limit: 10,
        };
        let batcher = AsyncBatcher::new(config, |v: Vec<i32>| {
            Box::pin(async move { v.into_iter().map(|x| x * 2).collect() })
        });
        let result = batcher.submit(21).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_async_batcher_submit_batch() {
        let config = AsyncOptimizationConfig {
            batch_size: 2,
            batch_timeout: Duration::from_millis(100),
            concurrency_limit: 4,
            queue_size_limit: 10,
        };
        let batcher = AsyncBatcher::new(config, |v: Vec<String>| {
            Box::pin(async move { v.into_iter().map(|s| s.to_uppercase()).collect() })
        });
        let (r1, r2) = tokio::join!(
            batcher.submit("hello".to_string()),
            batcher.submit("world".to_string())
        );
        assert_eq!(r1.unwrap(), "HELLO");
        assert_eq!(r2.unwrap(), "WORLD");
    }

    #[tokio::test]
    async fn test_async_batcher_queue_full() {
        let config = AsyncOptimizationConfig {
            batch_size: 100,
            batch_timeout: Duration::from_millis(100),
            concurrency_limit: 2,
            queue_size_limit: 1,
        };
        let batcher = Arc::new(AsyncBatcher::new(config, |v: Vec<i32>| {
            Box::pin(async move { v.into_iter().map(|x| x + 1).collect() })
        }));
        let batcher2 = Arc::clone(&batcher);
        let second_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            batcher2.submit(2).await
        });
        let first_handle = tokio::spawn(async move { batcher.submit(1).await });
        let second_result = tokio::time::timeout(Duration::from_millis(200), second_handle)
            .await
            .unwrap()
            .unwrap();
        let first_result = tokio::time::timeout(Duration::from_millis(50), first_handle)
            .await
            .ok()
            .and_then(|h| h.ok());
        let first_err = first_result.as_ref().and_then(|r| match r {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        });
        let has_queue_full = first_err.map(|s| s.contains("queue full")).unwrap_or(false)
            || second_result
                .err()
                .map(|e| e.to_string().contains("queue full"))
                .unwrap_or(false);
        assert!(has_queue_full, "Expected at least one queue full error");
    }

    #[tokio::test]
    async fn test_async_batcher_default_config() {
        let config = AsyncOptimizationConfig {
            batch_size: 1,
            ..AsyncOptimizationConfig::default()
        };
        let batcher = AsyncBatcher::new(config, |v: Vec<u8>| {
            Box::pin(async move { v.into_iter().map(|b| b.wrapping_add(1)).collect() })
        });
        let result = batcher.submit(0u8).await.unwrap();
        assert_eq!(result, 1);
    }
}
