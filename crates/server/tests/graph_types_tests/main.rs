// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Unit tests for execution graph types — split by concern for WateringHole line limits

mod builders;
mod graph_methods;
mod serialization;
mod validation;
