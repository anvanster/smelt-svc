//! Semantic delta computation between graph snapshots

use crate::{
    types::{GraphSnapshot, ImpactSummary, IntentId, SemanticChange, SemanticDelta},
    Result, SmeltError,
};
use chrono::Utc;
use uuid::Uuid;

/// Compute the semantic delta between two snapshots
pub fn compute_delta(
    intent_id: IntentId,
    from: &GraphSnapshot,
    to: &GraphSnapshot,
) -> Result<SemanticDelta> {
    let mut changes = Vec::new();
    let mut impact = ImpactSummary::default();

    // Compute file changes
    compute_file_changes(from, to, &mut changes, &mut impact);

    // Compute node changes based on node counts by kind
    compute_node_changes(from, to, &mut changes, &mut impact);

    Ok(SemanticDelta {
        id: Uuid::new_v4(),
        intent_id,
        timestamp: Utc::now(),
        from_snapshot: from.id,
        to_snapshot: to.id,
        changes,
        impact_summary: impact,
    })
}

/// Compute file-level changes between snapshots
fn compute_file_changes(
    from: &GraphSnapshot,
    to: &GraphSnapshot,
    changes: &mut Vec<SemanticChange>,
    impact: &mut ImpactSummary,
) {
    use std::collections::HashSet;

    let from_files: HashSet<&String> = from.files.iter().collect();
    let to_files: HashSet<&String> = to.files.iter().collect();

    // Files added
    for file in to_files.difference(&from_files) {
        changes.push(SemanticChange::FileAdded {
            path: (*file).clone(),
            symbol_count: 0, // Will be populated when we have full graph analysis
        });
    }

    // Files removed
    for file in from_files.difference(&to_files) {
        changes.push(SemanticChange::FileRemoved {
            path: (*file).clone(),
            symbol_count: 0,
        });
    }

    // Count affected files
    let added = to_files.difference(&from_files).count();
    let removed = from_files.difference(&to_files).count();
    let modified = from_files.intersection(&to_files).count(); // Simplified - assumes all common files are modified

    impact.files_affected =
        added + removed + modified.min(to.node_count.saturating_sub(from.node_count));
}

/// Compute node-level changes between snapshots
fn compute_node_changes(
    from: &GraphSnapshot,
    to: &GraphSnapshot,
    _changes: &mut Vec<SemanticChange>,
    impact: &mut ImpactSummary,
) {
    // Compare node counts by kind
    for (kind, &to_count) in &to.nodes_by_kind {
        let from_count = from.nodes_by_kind.get(kind).copied().unwrap_or(0);

        if to_count > from_count {
            // Nodes added of this kind
            let added = to_count - from_count;
            match kind.as_str() {
                "function" | "method" => impact.functions_added += added,
                "class" | "struct" | "enum" | "type" => impact.types_added += added,
                _ => {}
            }
        } else if to_count < from_count {
            // Nodes removed of this kind
            let removed = from_count - to_count;
            match kind.as_str() {
                "function" | "method" => impact.functions_removed += removed,
                "class" | "struct" | "enum" | "type" => impact.types_removed += removed,
                _ => {}
            }
        }
    }

    // Check for kinds that existed in 'from' but not in 'to'
    for (kind, &from_count) in &from.nodes_by_kind {
        if !to.nodes_by_kind.contains_key(kind) {
            match kind.as_str() {
                "function" | "method" => impact.functions_removed += from_count,
                "class" | "struct" | "enum" | "type" => impact.types_removed += from_count,
                _ => {}
            }
        }
    }

    // Compute edge/dependency changes
    if to.edge_count > from.edge_count {
        impact.dependencies_added = to.edge_count - from.edge_count;
    } else if to.edge_count < from.edge_count {
        impact.dependencies_removed = from.edge_count - to.edge_count;
    }

    // Compute complexity delta based on node count change
    // This is a rough approximation - real complexity would need AST analysis
    let node_delta = to.node_count as i32 - from.node_count as i32;
    impact.complexity_delta = node_delta;
}

/// Detailed delta computation with full graph access (for future use)
#[allow(dead_code)]
pub fn compute_detailed_delta(
    _intent_id: IntentId,
    _from: &GraphSnapshot,
    _to: &GraphSnapshot,
    _graph: &super::SmeltGraph,
) -> Result<SemanticDelta> {
    // TODO: Implement detailed delta computation using CodeGraph queries
    // This will:
    // 1. Query nodes that changed between snapshots
    // 2. Determine if changes are to signatures vs bodies
    // 3. Detect breaking changes
    // 4. Track visibility changes
    // 5. Compute accurate complexity deltas

    Err(SmeltError::DeltaComputation(
        "Detailed delta computation not yet implemented".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_snapshot(node_count: usize, edge_count: usize, files: Vec<String>) -> GraphSnapshot {
        GraphSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            git_sha: None,
            node_count,
            edge_count,
            nodes_by_kind: HashMap::new(),
            files,
            checksum: String::new(),
        }
    }

    #[test]
    fn test_compute_delta_basic() {
        let intent_id = Uuid::new_v4();
        let from = make_snapshot(10, 5, vec!["a.rs".into(), "b.rs".into()]);
        let to = make_snapshot(15, 8, vec!["a.rs".into(), "b.rs".into(), "c.rs".into()]);

        let delta = compute_delta(intent_id, &from, &to).unwrap();

        assert_eq!(delta.intent_id, intent_id);
        assert_eq!(delta.from_snapshot, from.id);
        assert_eq!(delta.to_snapshot, to.id);
        assert!(delta.impact_summary.files_affected > 0);
    }

    #[test]
    fn test_file_changes() {
        let intent_id = Uuid::new_v4();
        let from = make_snapshot(0, 0, vec!["a.rs".into(), "b.rs".into()]);
        let to = make_snapshot(0, 0, vec!["b.rs".into(), "c.rs".into()]);

        let delta = compute_delta(intent_id, &from, &to).unwrap();

        // Should have FileAdded(c.rs) and FileRemoved(a.rs)
        let added = delta
            .changes
            .iter()
            .filter(|c| matches!(c, SemanticChange::FileAdded { .. }))
            .count();
        let removed = delta
            .changes
            .iter()
            .filter(|c| matches!(c, SemanticChange::FileRemoved { .. }))
            .count();

        assert_eq!(added, 1);
        assert_eq!(removed, 1);
    }
}
