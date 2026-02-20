#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for reference graph visualization

use crate::reference_graph::{
    GraphMode, GraphNode, ReferenceEdge, ReferenceFilter, ReferenceGraphView, ReferenceType,
    WorldBounds, ZoomBounds,
};

#[test]
fn test_reference_type_name() {
    assert_eq!(ReferenceType::L0.name(), "L0");
    assert_eq!(ReferenceType::L1.name(), "L1");
    assert_eq!(ReferenceType::LongTerm.name(), "Long-term");
}

#[test]
fn test_reference_edge_creation() {
    let edge = ReferenceEdge::new(10, 5, ReferenceType::L0);
    assert_eq!(edge.from_idx, 10);
    assert_eq!(edge.to_idx, 5);
    assert_eq!(edge.ref_type, ReferenceType::L0);
    assert!(edge.weight.is_none());

    let edge = edge.with_weight(0.8);
    assert_eq!(edge.weight, Some(0.8));
}

#[test]
fn test_world_bounds_clamp() {
    let bounds = WorldBounds::new(0.0, 0.0, 100.0, 100.0);

    let (x, y) = bounds.clamp_point(50.0, 50.0);
    assert_eq!((x, y), (50.0, 50.0)); // Within bounds

    let (x, y) = bounds.clamp_point(-10.0, 50.0);
    assert_eq!((x, y), (0.0, 50.0)); // Clamped to min_x

    let (x, y) = bounds.clamp_point(150.0, 50.0);
    assert_eq!((x, y), (100.0, 50.0)); // Clamped to max_x
}

#[test]
fn test_world_bounds_contains() {
    let bounds = WorldBounds::new(0.0, 0.0, 100.0, 100.0);

    assert!(bounds.contains(50.0, 50.0));
    assert!(!bounds.contains(-10.0, 50.0));
    assert!(!bounds.contains(150.0, 50.0));
}

#[test]
fn test_world_bounds_dimensions() {
    let bounds = WorldBounds::new(10.0, 20.0, 110.0, 120.0);
    assert_eq!(bounds.width(), 100.0);
    assert_eq!(bounds.height(), 100.0);
}

#[test]
fn test_zoom_bounds_clamp() {
    let bounds = ZoomBounds::new(0.5, 5.0);

    assert_eq!(bounds.clamp(1.0), 1.0); // Within bounds
    assert_eq!(bounds.clamp(0.1), 0.5); // Clamped to min
    assert_eq!(bounds.clamp(10.0), 5.0); // Clamped to max
}

#[test]
fn test_zoom_bounds_is_valid() {
    let bounds = ZoomBounds::new(0.5, 5.0);

    assert!(bounds.is_valid(1.0));
    assert!(!bounds.is_valid(0.1));
    assert!(!bounds.is_valid(10.0));
}

#[test]
fn test_reference_graph_creation() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let graph = ReferenceGraphView::new(bounds);

    assert_eq!(graph.nodes.len(), 0);
    assert_eq!(graph.edges.len(), 0);
    assert_eq!(graph.zoom, 1.0);
    assert_eq!(graph.mode, GraphMode::Full);
}

#[test]
fn test_reference_graph_add_nodes() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_node(GraphNode::new(0, "I".to_string(), (100.0, 100.0)));
    graph.add_node(GraphNode::new(1, "P".to_string(), (200.0, 100.0)));

    assert_eq!(graph.nodes.len(), 2);
}

#[test]
fn test_reference_graph_zoom() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.set_zoom(2.0);
    assert_eq!(graph.zoom, 2.0);

    graph.zoom_in(1.5);
    assert_eq!(graph.zoom, 3.0);

    graph.zoom_out(3.0);
    assert_eq!(graph.zoom, 1.0);

    // Test bounds clamping
    graph.set_zoom(100.0);
    assert_eq!(graph.zoom, 10.0); // Clamped to max
}

#[test]
fn test_reference_graph_pan() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.pan(50.0, 50.0);
    assert_eq!(graph.pan_offset, (50.0, 50.0));

    // Test bounds clamping
    graph.pan(2000.0, 0.0);
    assert_eq!(graph.pan_offset.0, 1000.0); // Clamped to max_x
}

#[test]
fn test_reference_graph_select_frame() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_node(GraphNode::new(0, "I".to_string(), (100.0, 100.0)));
    graph.add_node(GraphNode::new(1, "P".to_string(), (200.0, 100.0)));
    graph.add_edge(ReferenceEdge::new(1, 0, ReferenceType::L0));

    graph.select_frame(1);

    assert_eq!(graph.selected_frame, Some(1));
    assert!(graph.nodes[1].is_selected);
    assert!(graph.nodes[0].is_highlighted); // Frame 0 is referenced by frame 1
}

#[test]
fn test_reference_graph_get_references() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_edge(ReferenceEdge::new(2, 0, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(2, 1, ReferenceType::L1));

    let refs = graph.get_references(2);
    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&0));
    assert!(refs.contains(&1));
}

#[test]
fn test_reference_graph_get_dependents() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_edge(ReferenceEdge::new(2, 0, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(3, 0, ReferenceType::L0));

    let deps = graph.get_dependents(0);
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&2));
    assert!(deps.contains(&3));
}

#[test]
fn test_reference_graph_counts() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_edge(ReferenceEdge::new(2, 0, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(2, 1, ReferenceType::L1));

    assert_eq!(graph.reference_count(2), 2);
    assert_eq!(graph.dependent_count(0), 1);
}

#[test]
fn test_reference_graph_search() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_node(GraphNode::new(10, "I".to_string(), (100.0, 100.0)));
    graph.add_node(GraphNode::new(20, "P".to_string(), (200.0, 100.0)));

    graph.set_search_query(Some("10".to_string()));
    let filtered = graph.filtered_nodes();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].display_idx, 10);
}

#[test]
fn test_reference_depth() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    // Chain: 3 -> 2 -> 1 -> 0
    graph.add_edge(ReferenceEdge::new(1, 0, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(2, 1, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(3, 2, ReferenceType::L0));

    assert_eq!(graph.reference_depth(0), 1); // No refs
    assert_eq!(graph.reference_depth(1), 2); // 1 -> 0
    assert_eq!(graph.reference_depth(2), 3); // 2 -> 1 -> 0
    assert_eq!(graph.reference_depth(3), 4); // 3 -> 2 -> 1 -> 0
}

#[test]
fn test_reference_filter() {
    let mut filter = ReferenceFilter::new();
    filter.frame_types.insert("I".to_string());

    let node_i = GraphNode::new(0, "I".to_string(), (0.0, 0.0));
    let node_p = GraphNode::new(1, "P".to_string(), (0.0, 0.0));

    assert!(filter.matches_node(&node_i, true, false));
    assert!(!filter.matches_node(&node_p, true, false));
}

// UX Graph viz_core tests - Task 8 (S.T4-1.ALL.UX.Graph.impl.viz_core.005)

#[test]
fn test_ux_graph_node_selection_highlights_refs_deps() {
    // UX Graph: User clicks on a frame node to see its reference chain
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    // UX Graph: Setup graph with reference chain: 0 <- 1 <- 2 <- 3
    graph.add_node(GraphNode::new(0, "I".to_string(), (100.0, 100.0)));
    graph.add_node(GraphNode::new(1, "P".to_string(), (200.0, 100.0)));
    graph.add_node(GraphNode::new(2, "P".to_string(), (300.0, 100.0)));
    graph.add_node(GraphNode::new(3, "B".to_string(), (400.0, 100.0)));

    graph.add_edge(ReferenceEdge::new(1, 0, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(2, 1, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(3, 2, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(3, 0, ReferenceType::L1));

    // UX Graph: User clicks on frame 2
    graph.select_frame(2);

    // UX Graph: Frame 2 should be selected
    assert_eq!(graph.selected_frame, Some(2));
    assert!(graph.nodes[2].is_selected);

    // UX Graph: Frame 1 (reference) and frame 3 (dependent) should be highlighted
    assert!(graph.nodes[1].is_highlighted); // Referenced by frame 2
    assert!(graph.nodes[3].is_highlighted); // Depends on frame 2

    // UX Graph: Frame 0 should not be highlighted (not direct ref/dep)
    assert!(!graph.nodes[0].is_highlighted);
}

#[test]
fn test_ux_graph_pan_viewport_with_bounds() {
    // UX Graph: User pans the graph viewport with mouse drag
    let bounds = WorldBounds::new(-500.0, -500.0, 500.0, 500.0);
    let mut graph = ReferenceGraphView::new(bounds);

    // UX Graph: Initial pan at origin
    assert_eq!(graph.pan_offset, (0.0, 0.0));

    // UX Graph: User drags viewport right and down
    graph.pan(100.0, 150.0);
    assert_eq!(graph.pan_offset, (100.0, 150.0));

    // UX Graph: User drags further right
    graph.pan(250.0, 0.0);
    assert_eq!(graph.pan_offset, (350.0, 150.0));

    // UX Graph: User tries to pan beyond world bounds
    graph.pan(500.0, 500.0);

    // UX Graph: Should clamp to world bounds
    assert_eq!(graph.pan_offset, (500.0, 500.0)); // Clamped to max

    // UX Graph: User tries to pan in negative direction beyond bounds
    graph.pan(-2000.0, -2000.0);
    assert_eq!(graph.pan_offset, (-500.0, -500.0)); // Clamped to min
}

#[test]
fn test_ux_graph_zoom_wheel_with_bounds() {
    // UX Graph: User zooms in/out with mouse wheel
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    // UX Graph: Initial zoom is 1.0 (100%)
    assert_eq!(graph.zoom, 1.0);

    // UX Graph: User scrolls wheel up (zoom in 1.2x)
    graph.zoom_in(1.2);
    assert!((graph.zoom - 1.2).abs() < 0.001);

    // UX Graph: User scrolls wheel up again
    graph.zoom_in(1.5);
    assert!((graph.zoom - 1.8).abs() < 0.001);

    // UX Graph: User tries to zoom way in (should clamp to max)
    graph.set_zoom(50.0);
    assert_eq!(graph.zoom, 10.0); // Clamped to default max

    // UX Graph: User scrolls wheel down (zoom out)
    graph.zoom_out(2.0);
    assert_eq!(graph.zoom, 5.0);

    // UX Graph: User tries to zoom way out (should clamp to min)
    graph.set_zoom(0.01);
    assert_eq!(graph.zoom, 0.1); // Clamped to default min
}

#[test]
fn test_ux_graph_search_filter_by_frame_index() {
    // UX Graph: User types in search box to find specific frames
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    // UX Graph: Add frames to graph
    for i in 0..10 {
        let frame_type = if i % 5 == 0 { "I" } else { "P" };
        graph.add_node(GraphNode::new(
            i,
            frame_type.to_string(),
            (i as f32 * 100.0, 100.0),
        ));
    }

    // UX Graph: User types "5" in search box
    graph.set_search_query(Some("5".to_string()));

    // UX Graph: Should match frame 5 (exact match)
    let filtered = graph.filtered_nodes();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].display_idx, 5);

    // UX Graph: User types "0" in search box
    graph.set_search_query(Some("0".to_string()));

    // UX Graph: Should match frames 0 (since "0" appears in index)
    let filtered = graph.filtered_nodes();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].display_idx, 0);

    // UX Graph: User clears search box
    graph.set_search_query(None);

    // UX Graph: Should show all frames
    let filtered = graph.filtered_nodes();
    assert_eq!(filtered.len(), 10);
}

#[test]
fn test_ux_graph_toggle_summary_mode() {
    // UX Graph: User toggles between Full and Summary display modes
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    // UX Graph: Add many nodes to graph
    for i in 0..50 {
        let frame_type = if i % 10 == 0 {
            "I"
        } else if i % 3 == 0 {
            "B"
        } else {
            "P"
        };
        graph.add_node(GraphNode::new(
            i,
            frame_type.to_string(),
            (i as f32 * 20.0, 100.0),
        ));
    }

    // UX Graph: Initial mode is Full
    assert_eq!(graph.mode, GraphMode::Full);

    // UX Graph: User clicks "Summary Mode" button to reduce complexity
    graph.set_mode(GraphMode::Summary);
    assert_eq!(graph.mode, GraphMode::Summary);

    // UX Graph: User clicks "Full Mode" button to see all details
    graph.set_mode(GraphMode::Full);
    assert_eq!(graph.mode, GraphMode::Full);

    // UX Graph: Verify mode affects visualization (in actual impl, Summary would cluster/simplify)
    // For now, just verify the mode state is tracked correctly
    assert_eq!(graph.nodes.len(), 50);
}

// Feature Parity Phase D - Reference Graph UI tests

#[test]
fn test_reference_graph_statistics() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    // Add nodes
    graph.add_node(GraphNode::new(0, "I".to_string(), (100.0, 100.0)));
    graph.add_node(GraphNode::new(1, "P".to_string(), (200.0, 100.0)));
    graph.add_node(GraphNode::new(2, "P".to_string(), (300.0, 100.0)));
    graph.add_node(GraphNode::new(3, "B".to_string(), (400.0, 100.0)));
    graph.add_node(GraphNode::new(4, "B".to_string(), (500.0, 100.0)));

    // Add edges
    graph.add_edge(ReferenceEdge::new(1, 0, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(2, 1, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(3, 1, ReferenceType::L0));
    graph.add_edge(ReferenceEdge::new(3, 2, ReferenceType::L1));
    graph.add_edge(ReferenceEdge::new(4, 2, ReferenceType::L0));

    let stats = graph.statistics();

    assert_eq!(stats.total_nodes, 5);
    assert_eq!(stats.total_edges, 5);
    assert_eq!(stats.i_frames, 1);
    assert_eq!(stats.p_frames, 2);
    assert_eq!(stats.b_frames, 2);
    assert_eq!(stats.l0_edges, 4);
    assert_eq!(stats.l1_edges, 1);
    assert_eq!(stats.lt_edges, 0);
    assert_eq!(stats.max_depth, 4); // Longest chain: 3->1->0 = 3, or 3->2->1->0 = 4
}

#[test]
fn test_reference_graph_auto_layout() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_node(GraphNode::new(0, "I".to_string(), (0.0, 0.0)));
    graph.add_node(GraphNode::new(1, "P".to_string(), (0.0, 0.0)));
    graph.add_node(GraphNode::new(2, "B".to_string(), (0.0, 0.0)));

    graph.auto_layout_timeline(50.0, 30.0);

    // Check positions
    assert_eq!(graph.nodes[0].position, (0.0, 0.0)); // I at row 0
    assert_eq!(graph.nodes[1].position, (50.0, 30.0)); // P at row 1
    assert_eq!(graph.nodes[2].position, (100.0, 60.0)); // B at row 2
}

#[test]
fn test_reference_graph_summary_text() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_node(GraphNode::new(0, "I".to_string(), (0.0, 0.0)));
    graph.add_node(GraphNode::new(1, "P".to_string(), (0.0, 0.0)));
    graph.add_edge(ReferenceEdge::new(1, 0, ReferenceType::L0));

    let summary = graph.summary_text();
    assert!(summary.contains("2 frames"));
    assert!(summary.contains("1 I"));
    assert!(summary.contains("1 P"));
    assert!(summary.contains("1 refs"));
}

#[test]
fn test_reference_graph_node_at_position() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_node(GraphNode::new(0, "I".to_string(), (100.0, 100.0)));
    graph.add_node(GraphNode::new(1, "P".to_string(), (200.0, 100.0)));

    // Click near node 0
    let node = graph.node_at_position(105.0, 102.0, 20.0);
    assert!(node.is_some());
    assert_eq!(node.unwrap().display_idx, 0);

    // Click near node 1
    let node = graph.node_at_position(195.0, 105.0, 20.0);
    assert!(node.is_some());
    assert_eq!(node.unwrap().display_idx, 1);

    // Click in empty space
    let node = graph.node_at_position(500.0, 500.0, 20.0);
    assert!(node.is_none());
}

#[test]
fn test_reference_graph_center_on_node() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    graph.add_node(GraphNode::new(0, "I".to_string(), (100.0, 100.0)));
    graph.add_node(GraphNode::new(5, "P".to_string(), (500.0, 200.0)));

    graph.center_on_node(5);
    assert_eq!(graph.pan_offset, (500.0, 200.0));

    graph.center_on_node(0);
    assert_eq!(graph.pan_offset, (100.0, 100.0));

    // Non-existent node - should not change
    graph.center_on_node(999);
    assert_eq!(graph.pan_offset, (100.0, 100.0));
}

#[test]
fn test_reference_graph_isolated_count() {
    let bounds = WorldBounds::new(0.0, 0.0, 1000.0, 1000.0);
    let mut graph = ReferenceGraphView::new(bounds);

    // 3 nodes: 0 is isolated, 1->2 are connected
    graph.add_node(GraphNode::new(0, "I".to_string(), (0.0, 0.0)));
    graph.add_node(GraphNode::new(1, "P".to_string(), (0.0, 0.0)));
    graph.add_node(GraphNode::new(2, "P".to_string(), (0.0, 0.0)));

    graph.add_edge(ReferenceEdge::new(2, 1, ReferenceType::L0));

    let stats = graph.statistics();
    assert_eq!(stats.isolated_count, 1); // Only node 0 is isolated
}
