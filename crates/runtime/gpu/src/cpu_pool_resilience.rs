// SPDX-License-Identifier: AGPL-3.0-or-later
//! Rayon thread pool resilience layer for CPU compute.
//!
//! Contains retry/fallback strategies for constructing a Rayon pool under
//! resource-constrained environments. Extracted from `cpu_resource.rs` to
//! keep the main resource module focused on capabilities and execution.

use std::sync::Arc;
use std::time::Duration;

const POOL_RETRY_BACKOFF: Duration = Duration::from_millis(10);

/// Last-resort pool when the degraded cascade cannot build a zero-thread delegate.
///
/// `num_threads(0)` mirrors the global Rayon pool and should not fail on supported hosts;
/// this function only runs when prior builders already failed.
pub(crate) fn build_last_resort_degraded_pool() -> rayon::ThreadPool {
    tracing::error!("zero-thread pool failed; entering last-resort degraded pool construction");
    let _ = rayon::ThreadPoolBuilder::new().build_global();
    for build in [
        || rayon::ThreadPoolBuilder::new().num_threads(0).build(),
        || rayon::ThreadPoolBuilder::new().use_current_thread().build(),
        || {
            rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .stack_size(256 * 1024)
                .build()
        },
        || rayon::ThreadPoolBuilder::new().build(),
    ] {
        if let Ok(pool) = build() {
            return pool;
        }
    }
    tracing::error!("all last-resort pool builders failed; retrying current-thread pool");
    for _ in 0..8 {
        if let Ok(pool) = rayon::ThreadPoolBuilder::new().use_current_thread().build() {
            return pool;
        }
        std::thread::yield_now();
    }
    tracing::error!("degraded pool construction exhausted retries; using default builder");
    rayon::ThreadPoolBuilder::new()
        .build()
        .or_else(|_| rayon::ThreadPoolBuilder::new().num_threads(0).build())
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "default degraded pool builder failed");
            rayon::ThreadPoolBuilder::new()
                .use_current_thread()
                .build()
                .unwrap_or_else(|e2| {
                    tracing::error!(error = %e2, "current-thread degraded pool failed");
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(0)
                        .build()
                        .unwrap_or_else(|e3| {
                            tracing::error!(
                                error = %e3,
                                "cannot construct degraded CPU pool without OS threads"
                            );
                            rayon::ThreadPoolBuilder::new()
                                .num_threads(1)
                                .stack_size(256 * 1024)
                                .build()
                                .unwrap_or_else(|e4| {
                                    tracing::error!(
                                        error = %e4,
                                        "minimal degraded pool failed; yielding and retrying"
                                    );
                                    std::thread::yield_now();
                                    rayon::ThreadPoolBuilder::new()
                                        .use_current_thread()
                                        .build()
                                        .unwrap_or_else(|e5| {
                                            tracing::error!(
                                                error = %e5,
                                                "degraded CPU pool unavailable"
                                            );
                                            rayon::ThreadPoolBuilder::new().build().unwrap_or_else(
                                                |e6| {
                                                    tracing::error!(
                                                        error = %e6,
                                                        "terminal degraded pool construction failed"
                                                    );
                                                    rayon::ThreadPoolBuilder::new()
                                                        .num_threads(0)
                                                        .build()
                                                        .unwrap_or_else(|e7| {
                                                            tracing::error!(
                                                                error = %e7,
                                                                "terminal zero-thread pool failed"
                                                            );
                                                            rayon::ThreadPoolBuilder::new()
                                                                .use_current_thread()
                                                                .build()
                                                                .unwrap_or_else(|e8| {
                                                                    tracing::error!(
                                                                        error = %e8,
                                                                        "all degraded pool strategies exhausted"
                                                                    );
                                                                    blocking_degraded_pool()
                                                                })
                                                        })
                                                },
                                            )
                                        })
                                })
                        })
                })
        })
}

/// Blocks until a current-thread pool can be constructed (transient resource exhaustion).
pub(crate) fn blocking_degraded_pool() -> rayon::ThreadPool {
    loop {
        if let Ok(pool) = rayon::ThreadPoolBuilder::new().use_current_thread().build() {
            return pool;
        }
        if let Ok(pool) = rayon::ThreadPoolBuilder::new().num_threads(0).build() {
            return pool;
        }
        std::thread::sleep(POOL_RETRY_BACKOFF);
    }
}

/// Process-wide degraded pool used only when all runtime construction paths fail.
pub(crate) fn degraded_pool() -> Arc<rayon::ThreadPool> {
    static DEGRADED_CPU_POOL: std::sync::LazyLock<Arc<rayon::ThreadPool>> =
        std::sync::LazyLock::new(|| {
            Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .use_current_thread()
                    .build()
                    .unwrap_or_else(|e| {
                        tracing::error!(
                            error = %e,
                            "degraded current-thread CPU pool failed; retrying with num_threads(1)"
                        );
                        rayon::ThreadPoolBuilder::new()
                            .num_threads(1)
                            .build()
                            .unwrap_or_else(|e2| {
                                tracing::error!(
                                    error = %e2,
                                    "minimal single-thread CPU pool failed; using zero-thread pool"
                                );
                                build_last_resort_degraded_pool()
                            })
                    }),
            )
        });

    Arc::clone(&DEGRADED_CPU_POOL)
}
