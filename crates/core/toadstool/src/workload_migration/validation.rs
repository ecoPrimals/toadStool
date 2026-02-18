// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Migration validation and rollback support.
//!
//! Placeholder for pre-migration validation and rollback logic.
//! Extended in future iterations for constraint validation and state rollback.

use super::MigrationRecommendation;

/// Validate that a migration recommendation is actionable
#[must_use]
pub fn validate_recommendation(recommendation: &MigrationRecommendation) -> bool {
    if recommendation.should_migrate {
        recommendation.target.is_some() && recommendation.confidence >= 0.5
    } else {
        true
    }
}
