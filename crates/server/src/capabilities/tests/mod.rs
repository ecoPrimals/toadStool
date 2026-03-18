// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for capability-based primal discovery.
#![allow(deprecated, clippy::await_holding_lock)]

use super::*;
use std::collections::HashMap;
pub(crate) use std::path::PathBuf;
pub(crate) use tempfile::TempDir;
pub(crate) use toadstool_common::interned_strings::primals;

pub(crate) fn with_temp_discovery<F, Fut, R>(f: F) -> R
where
    F: FnOnce(PathBuf) -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let _temp = TempDir::new().expect("temp dir");
    let base = _temp.path().to_path_buf();
    let discovery_base = base.join("ecoPrimals").join("discovery");
    std::fs::create_dir_all(&discovery_base).expect("create discovery dir");
    let base_str = base.to_string_lossy().to_string();
    temp_env::with_var("XDG_RUNTIME_DIR", Some(base_str.as_str()), || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(f(discovery_base))
    })
}

mod capability_query;
mod discovery_dir;
mod edge_cases;
mod peer_discovery;
mod serialization;
