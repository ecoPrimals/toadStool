// SPDX-License-Identifier: AGPL-3.0-or-later
//! Vector operations for the CPU backend: dot product, elementwise binary, gather, scatter.

mod dot_product;
mod elementwise;
mod gather_scatter;

#[cfg(test)]
mod tests;

pub(crate) use dot_product::execute_dot_product;
pub(crate) use elementwise::execute_elementwise_binary;
pub(crate) use gather_scatter::{execute_gather, execute_scatter};
