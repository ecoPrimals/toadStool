// SPDX-License-Identifier: AGPL-3.0-or-later
//! API handler utilities.
//!
//! REST handlers have been removed (Session 90). All API functionality is
//! served exclusively via JSON-RPC 2.0 at `/jsonrpc`.
//!
//! Only shared helpers remain here for use by JSON-RPC or other internal code.

pub mod helpers;
