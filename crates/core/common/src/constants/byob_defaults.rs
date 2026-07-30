// SPDX-License-Identifier: AGPL-3.0-or-later
//! Default BYOB (Bring Your Own Biome) web service ports and related literals.

/// Well-known HTTP (IANA).
pub const WEB_SERVICE_PORT_HTTP: u16 = 80;
/// Well-known HTTPS (IANA).
pub const WEB_SERVICE_PORT_HTTPS: u16 = 443;
/// Common alternate HTTPS (e.g. management UIs).
pub const WEB_SERVICE_PORT_HTTPS_ALT: u16 = 8443;
/// Typical frontend dev-server port.
pub const WEB_SERVICE_PORT_DEV_HTTP: u16 = 3000;
/// Common alternate HTTP (e.g. Python `http.server`).
pub const WEB_SERVICE_PORT_ALT_HTTP: u16 = 8000;
/// Common application / alternate service port.
pub const WEB_SERVICE_PORT_ALT_SERVICE: u16 = 9000;

/// Common web service ports probed for external IP allocation in BYOB deployments.
pub const COMMON_WEB_SERVICE_PORTS: &[u16] = &[
    WEB_SERVICE_PORT_HTTP,
    WEB_SERVICE_PORT_HTTPS,
    WEB_SERVICE_PORT_HTTPS_ALT,
    WEB_SERVICE_PORT_DEV_HTTP,
    WEB_SERVICE_PORT_ALT_HTTP,
    WEB_SERVICE_PORT_ALT_SERVICE,
];
