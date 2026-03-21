// SPDX-License-Identifier: AGPL-3.0-only

use crate::ecosystem::management::ServiceManager;

#[tokio::test]
async fn test_service_manager_creation() {
    let manager = ServiceManager::new();
    let services = manager.get_all_services().await;
    assert!(services.is_empty());
}

#[tokio::test]
async fn test_service_manager_default() {
    let manager = ServiceManager::default();
    let services = manager.get_all_services().await;
    assert!(services.is_empty());
}
