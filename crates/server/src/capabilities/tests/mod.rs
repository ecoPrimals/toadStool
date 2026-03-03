// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for capability-based primal discovery.
#![allow(deprecated, clippy::await_holding_lock)]

use super::*;
use std::collections::HashMap;
pub(crate) use std::path::PathBuf;
pub(crate) use tempfile::TempDir;
pub(crate) use toadstool_common::interned_strings::primals;

/// Mutex to serialize tests that modify XDG_RUNTIME_DIR.
pub(crate) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) async fn with_temp_discovery<F, Fut, R>(f: F) -> R
where
    F: FnOnce(PathBuf) -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let _temp = TempDir::new().expect("temp dir");
    let base = _temp.path().to_path_buf();
    let discovery_base = base.join("ecoPrimals").join("discovery");
    std::fs::create_dir_all(&discovery_base).expect("create discovery dir");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", &base);
    let out = f(discovery_base).await;
    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
    out
}

mod capability_query;
mod discovery_dir;
mod edge_cases;
mod peer_discovery;
mod serialization;
