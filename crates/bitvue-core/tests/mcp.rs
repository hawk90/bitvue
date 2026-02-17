#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for MCP Integration Layer

use bitvue_core::diagnostics_bands::DiagnosticsBands;
use bitvue_core::frame_identity::FrameMetadata;
use bitvue_core::mcp::McpIntegration;
use bitvue_core::selection::TemporalSelection;
use bitvue_core::{FrameIndexMap, InsightFeed, SelectionState, StreamId};

fn create_test_selection() -> SelectionState {
    let mut selection = SelectionState::new(StreamId::A);
    selection.temporal = Some(TemporalSelection::Point { frame_index: 42 });
    selection
}

fn create_test_diagnostics() -> DiagnosticsBands {
    DiagnosticsBands::new()
}

fn create_test_insights() -> InsightFeed {
    let frames = vec![
        FrameMetadata {
            pts: Some(0),
            dts: Some(0),
        },
        FrameMetadata {
            pts: Some(1000),
            dts: Some(1000),
        },
    ];
    let frame_map = FrameIndexMap::new(&frames);
    let diagnostics = DiagnosticsBands::new();
    InsightFeed::generate(&frame_map, &diagnostics)
}

#[test]
fn test_mcp_integration_creation() {
    let selection = create_test_selection();
    let insights = create_test_insights();
    let diagnostics = create_test_diagnostics();

    let mcp = McpIntegration::new(&selection, &insights, &diagnostics, None, None, None);

    assert!(mcp.resources.selection_state.is_some());
    assert!(mcp.resources.insight_feed.is_some());
    assert!(mcp.resources.diagnostics.is_some());
}

#[test]
fn test_list_resources() {
    let selection = create_test_selection();
    let insights = create_test_insights();
    let diagnostics = create_test_diagnostics();

    let mcp = McpIntegration::new(&selection, &insights, &diagnostics, None, None, None);

    let resources = mcp.list_resources();
    assert!(resources.contains(&"selection_state"));
    assert!(resources.contains(&"insight_feed"));
    assert!(resources.contains(&"diagnostics"));
}

#[test]
fn test_get_resource() {
    let selection = create_test_selection();
    let insights = create_test_insights();
    let diagnostics = create_test_diagnostics();

    let mcp = McpIntegration::new(&selection, &insights, &diagnostics, None, None, None);

    let selection_json = mcp.get_resource("selection_state");
    assert!(selection_json.is_some());

    let nonexistent = mcp.get_resource("nonexistent");
    assert!(nonexistent.is_none());
}

#[test]
fn test_resource_serialization() {
    let selection = create_test_selection();
    let insights = create_test_insights();
    let diagnostics = create_test_diagnostics();

    let mcp = McpIntegration::new(&selection, &insights, &diagnostics, None, None, None);

    // Test that resources can be serialized to JSON
    let json = serde_json::to_string(&mcp.resources).unwrap();
    assert!(json.contains("selection_state"));
}
