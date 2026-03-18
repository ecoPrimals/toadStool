// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Composition Constraint System
//!
//! Constraint-based dynamic workload composition.
//! **Constraint Over Prescription**: Describe what we NEED, not HOW.

mod constraint;
mod evaluation;
mod request;

pub use constraint::Constraint;
pub use evaluation::{ConstraintEvaluation, ConstraintSatisfaction};
pub use request::{CompositionRequest, ConstraintPriority};

#[cfg(test)]
mod tests;
