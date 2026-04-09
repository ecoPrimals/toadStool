// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fuzz target: ToadStool config deserialization + validation.
//!
//! Feeds arbitrary bytes as TOML to `ToadStoolConfig` deserialization,
//! then calls `validate()` if parsing succeeds.
#![no_main]

use libfuzzer_sys::fuzz_target;
use toadstool_config::ToadStoolConfig;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(config) = toml::from_str::<ToadStoolConfig>(text) {
        let _ = config.validate();
    }
});
