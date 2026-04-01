// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! SmeltGraph - semantic code graph with versioning

mod diff;
mod smelt_graph;

pub use diff::compute_delta;
pub use smelt_graph::SmeltGraph;
