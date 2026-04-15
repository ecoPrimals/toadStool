// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_security_sandbox::*;
use std::time::Duration;

#[test]
fn test_sandbox_lifetime_ephemeral() {
    let lifetime = SandboxLifetime::Ephemeral;
    assert!(matches!(lifetime, SandboxLifetime::Ephemeral));
}

#[test]
fn test_sandbox_lifetime_persistent() {
    let ttl = Duration::from_secs(3600); // 1 hour
    let lifetime = SandboxLifetime::Persistent { ttl };

    match lifetime {
        SandboxLifetime::Persistent { ttl } => {
            assert_eq!(ttl, Duration::from_secs(3600));
        }
        _ => panic!("Expected Persistent variant"),
    }
}

#[test]
fn test_sandbox_lifetime_persistent_short() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(60), // 1 minute
    };

    match lifetime {
        SandboxLifetime::Persistent { ttl } => {
            assert!(ttl < Duration::from_secs(120));
        }
        _ => panic!("Expected Persistent"),
    }
}

#[test]
fn test_sandbox_lifetime_persistent_long() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(86400), // 24 hours
    };

    match lifetime {
        SandboxLifetime::Persistent { ttl } => {
            assert!(ttl > Duration::from_secs(3600));
        }
        _ => panic!("Expected Persistent"),
    }
}

#[test]
fn test_sandbox_lifetime_manual() {
    let lifetime = SandboxLifetime::Manual;
    assert!(matches!(lifetime, SandboxLifetime::Manual));
}

#[test]
fn test_sandbox_lifetime_clone() {
    let lifetime1 = SandboxLifetime::Ephemeral;
    let lifetime2 = lifetime1;
    assert!(matches!(lifetime2, SandboxLifetime::Ephemeral));
}

#[test]
fn test_sandbox_lifetime_persistent_with_ttl() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(300),
    };

    if let SandboxLifetime::Persistent { ttl } = lifetime {
        assert_eq!(ttl, Duration::from_secs(300));
    } else {
        panic!("Expected Persistent lifetime");
    }
}

#[test]
fn test_sandbox_lifetime_persistent_short_ttl() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(10),
    };

    if let SandboxLifetime::Persistent { ttl } = lifetime {
        assert_eq!(ttl.as_secs(), 10);
    } else {
        panic!("Expected Persistent lifetime");
    }
}

#[test]
fn test_sandbox_lifetime_persistent_long_ttl() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(3600),
    };

    if let SandboxLifetime::Persistent { ttl } = lifetime {
        assert_eq!(ttl.as_secs(), 3600);
    } else {
        panic!("Expected Persistent lifetime");
    }
}
