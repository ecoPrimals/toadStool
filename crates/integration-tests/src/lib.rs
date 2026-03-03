// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]

//! Integration test crate for ToadStool.
//!
//! Contains integration and end-to-end tests previously living as orphan files
//! in the workspace-root `tests/` directory (D-S16-004 resolution).
//!
//! Run all integration tests:
//! ```bash
//! cargo test -p toadstool-integration-tests
//! ```
//!
//! Run a specific suite:
//! ```bash
//! cargo test -p toadstool-integration-tests --test e2e_tests
//! ```
