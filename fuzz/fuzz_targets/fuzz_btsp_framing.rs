// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fuzz target: BTSP length-prefixed frame parsing.
//!
//! Wraps arbitrary bytes in a `Cursor` and calls `read_frame`,
//! exercising the length-prefix decode, bounds checking, and EOF handling.
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Cursor;
use toadstool_common::btsp::framing::read_frame;

fuzz_target!(|data: &[u8]| {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        let mut cursor = Cursor::new(data);
        let _ = read_frame(&mut cursor).await;
    });
});
