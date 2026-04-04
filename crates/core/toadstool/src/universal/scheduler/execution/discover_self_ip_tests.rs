// SPDX-License-Identifier: AGPL-3.0-only
//! Unit tests for `discover_self_ip_address` (`execution::discover`).

use super::discover_self_ip_address;
use toadstool_config::defaults::network::BIND_ADDRESS_DEFAULT;

#[test]
fn discover_prefers_toadstool_bind_address_host() {
    temp_env::with_var("TOADSTOOL_BIND_ADDRESS", Some("192.168.1.10:9090"), || {
        assert_eq!(discover_self_ip_address(), "192.168.1.10");
    });
}

#[test]
fn discover_bind_address_empty_host_falls_through() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", Some(":8080")),
            ("TOADSTOOL_BIND_HOST", Some("from-bind-host")),
        ],
        || {
            assert_eq!(discover_self_ip_address(), "from-bind-host");
        },
    );
}

#[test]
fn discover_toadstool_bind_host() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_BIND_HOST", Some("bind-host-only")),
        ],
        || {
            assert_eq!(discover_self_ip_address(), "bind-host-only");
        },
    );
}

#[test]
fn discover_bind_host_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_BIND_HOST", None::<&str>),
            ("BIND_HOST", Some("bind-host-env")),
        ],
        || {
            assert_eq!(discover_self_ip_address(), "bind-host-env");
        },
    );
}

#[test]
fn discover_host_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_BIND_HOST", None::<&str>),
            ("BIND_HOST", None::<&str>),
            ("HOST", Some("host-env")),
        ],
        || {
            assert_eq!(discover_self_ip_address(), "host-env");
        },
    );
}

#[test]
fn discover_hostname_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_BIND_HOST", None::<&str>),
            ("BIND_HOST", None::<&str>),
            ("HOST", None::<&str>),
            ("HOSTNAME", Some("my-box")),
        ],
        || {
            assert_eq!(discover_self_ip_address(), "my-box");
        },
    );
}

#[test]
fn discover_fallback_to_default_bind_address() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", None::<&str>),
            ("TOADSTOOL_BIND_HOST", None::<&str>),
            ("BIND_HOST", None::<&str>),
            ("HOST", None::<&str>),
            ("HOSTNAME", None::<&str>),
        ],
        || {
            assert_eq!(discover_self_ip_address(), BIND_ADDRESS_DEFAULT);
        },
    );
}

#[test]
fn discover_bind_address_wins_over_bind_host() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_BIND_ADDRESS", Some("10.0.0.1:80")),
            ("TOADSTOOL_BIND_HOST", Some("should-not-win")),
        ],
        || {
            assert_eq!(discover_self_ip_address(), "10.0.0.1");
        },
    );
}
