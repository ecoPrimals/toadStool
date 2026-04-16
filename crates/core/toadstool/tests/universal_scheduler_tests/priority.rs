// SPDX-License-Identifier: AGPL-3.0-or-later
//! Priority handling tests — all priority levels and ordering invariants.

use std::sync::Arc;

use toadstool::universal::{
    JobPriority, UniversalPrimalProviderDispatch, UniversalPrimalRegistry, UniversalScheduler,
};

use super::helpers::create_test_native_job;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_emergency_priority_job() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::<
        UniversalPrimalProviderDispatch,
    >::new()))
    .await
    .unwrap();
    assert!(
        scheduler
            .schedule_job(create_test_native_job(JobPriority::Emergency))
            .await
            .is_ok()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_critical_priority_job() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::<
        UniversalPrimalProviderDispatch,
    >::new()))
    .await
    .unwrap();
    assert!(
        scheduler
            .schedule_job(create_test_native_job(JobPriority::Critical))
            .await
            .is_ok()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_high_priority_job() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::<
        UniversalPrimalProviderDispatch,
    >::new()))
    .await
    .unwrap();
    assert!(
        scheduler
            .schedule_job(create_test_native_job(JobPriority::High))
            .await
            .is_ok()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_low_priority_job() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::<
        UniversalPrimalProviderDispatch,
    >::new()))
    .await
    .unwrap();
    assert!(
        scheduler
            .schedule_job(create_test_native_job(JobPriority::Low))
            .await
            .is_ok()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scheduler_background_priority_job() {
    let scheduler = UniversalScheduler::new(Arc::new(UniversalPrimalRegistry::<
        UniversalPrimalProviderDispatch,
    >::new()))
    .await
    .unwrap();
    assert!(
        scheduler
            .schedule_job(create_test_native_job(JobPriority::Background))
            .await
            .is_ok()
    );
}

#[test]
fn test_job_priority_ordering() {
    use std::cmp::Ordering;
    assert!(JobPriority::Emergency < JobPriority::Critical);
    assert!(JobPriority::Critical < JobPriority::High);
    assert!(JobPriority::High < JobPriority::Normal);
    assert!(JobPriority::Normal < JobPriority::Low);
    assert!(JobPriority::Low < JobPriority::Background);
    assert_eq!(
        JobPriority::Emergency.cmp(&JobPriority::Emergency),
        Ordering::Equal
    );
    assert_eq!(
        JobPriority::Background.cmp(&JobPriority::Background),
        Ordering::Equal
    );
}
