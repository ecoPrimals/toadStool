// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashSet;

use toadstool_auto_config::intelligent::PlatformSupport;

#[test]
fn test_platform_support_containers() {
    let support = PlatformSupport::Containers;

    assert!(matches!(support, PlatformSupport::Containers));
}

#[test]
fn test_platform_support_sandboxing() {
    let support = PlatformSupport::Sandboxing;

    assert!(matches!(support, PlatformSupport::Sandboxing));
}

#[test]
fn test_platform_support_process_isolation() {
    let support = PlatformSupport::ProcessIsolation;

    assert!(matches!(support, PlatformSupport::ProcessIsolation));
}

#[test]
fn test_platform_support_network_isolation() {
    let support = PlatformSupport::NetworkIsolation;

    assert!(matches!(support, PlatformSupport::NetworkIsolation));
}

#[test]
fn test_platform_support_equality() {
    let support1 = PlatformSupport::Containers;
    let support2 = PlatformSupport::Containers;
    let support3 = PlatformSupport::Sandboxing;

    assert_eq!(support1, support2);
    assert_ne!(support1, support3);
}

#[test]
fn test_platform_support_clone() {
    let support = PlatformSupport::Containers;
    let cloned = support.clone();

    assert_eq!(support, cloned);
}

#[test]
fn test_platform_support_hash() {
    let mut set = HashSet::new();
    set.insert(PlatformSupport::Containers);
    set.insert(PlatformSupport::Containers);
    set.insert(PlatformSupport::Sandboxing);

    assert_eq!(set.len(), 2, "Should have 2 unique elements");
    assert!(set.contains(&PlatformSupport::Containers));
}
