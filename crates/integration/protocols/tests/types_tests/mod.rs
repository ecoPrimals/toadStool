// SPDX-License-Identifier: AGPL-3.0-only
//! Type system tests for protocol integration
//!
//! Tests for message types, health status, and service information structures.

mod helpers;

pub use helpers::*;

mod auth;
mod health_status;
mod message_format;
mod message_priority;
mod protocol_error;
mod protocol_message;
mod service_endpoint;
mod service_info;
mod transport;
