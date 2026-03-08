// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ecosystem::management::ServiceManager;
use crate::ecosystem::types::ServiceStatus;

#[tokio::test]
async fn test_initial_service_status_empty() {
    let manager = ServiceManager::new();
    let status = manager.get_service_status("nonexistent").await;
    assert!(status.is_none());
}

#[tokio::test]
async fn test_service_status_after_registration() {
    let manager = ServiceManager::new();
    let svc = super::create_test_service("status-test", true);
    manager.register_service(svc).await.expect("registration should succeed");
    let all = manager.get_all_services().await;
    assert_eq!(all.len(), 1);
}
