//! SmeltGraph - semantic code graph with versioning

mod diff;
mod smelt_graph;

pub use diff::compute_delta;
pub use smelt_graph::SmeltGraph;
