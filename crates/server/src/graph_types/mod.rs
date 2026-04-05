// SPDX-License-Identifier: AGPL-3.0-or-later
//! Graph types for collaborative intelligence resource planning

pub mod edges;
pub mod errors;
pub mod graph;
pub mod nodes;

pub use edges::{EdgeType, GraphEdge};
pub use errors::GraphValidationError;
pub use graph::{ExecutionGraph, ExecutionGraphBuilder};
pub use nodes::{GraphNode, GraphNodeBuilder, NodeResourceRequirements};
