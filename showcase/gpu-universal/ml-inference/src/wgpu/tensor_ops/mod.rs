// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tensor Manipulation Operations
//!
//! Essential tensor operations for neuromorphic workflows.
//! Split into domain modules for maintainability.

mod activation_ops;
mod cast_ops;
mod indexing_ops;
mod norm_ops;
mod reduction_ops;
mod shape_ops;
mod unary_ops;

// Re-export all public types and enums
pub use activation_ops::{LogSoftmax, ReLU, Sigmoid, Softmax, GELU};
pub use cast_ops::Cast;
pub use indexing_ops::{Argmax, TopK, Where};
pub use norm_ops::{LayerNorm, Norm};
pub use reduction_ops::{Cumsum, Max, Mean, Min, Prod, Std, Sum, Var};
pub use shape_ops::{Expand, Pad, PadMode, Reshape, Slice, Squeeze, Transpose, Unsqueeze};
pub use unary_ops::{Abs, Clamp, Exp, Pow, Sqrt};
