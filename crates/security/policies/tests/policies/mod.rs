// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive Security Policy Tests Module
//!
//! This module organizes extensive security policy tests into logical groupings
//! for better maintainability and navigation.
//!
//! ## Organization
//! - `creation` - Policy creation, structure, and basic configuration
//! - `rules` - Policy rule tests and complex rule scenarios
//! - `conditions` - Policy condition evaluation tests
//! - `actions` - Policy action and violation action tests
//! - `evaluation` - Policy evaluation, composition, and enforcement

mod creation;
mod rules;
mod conditions;
mod actions;
mod evaluation;

