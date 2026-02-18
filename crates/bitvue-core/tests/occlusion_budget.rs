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
//! Tests for overlay stacking and alpha blending rules

use bitvue_core::occlusion_budget::{BlendMode, OcclusionBudget, OverlayConfig, OverlayLayer};

#[test]
fn test_overlay_layer_default_priority() {
    assert_eq!(OverlayLayer::BaseFrame.default_priority(), 0);
    assert!(OverlayLayer::Grid.default_priority() > OverlayLayer::QpHeatmap.default_priority());
    assert!(
        OverlayLayer::SelectionHighlight.default_priority() > OverlayLayer::Grid.default_priority()
    );
}

#[test]
fn test_overlay_layer_default_alpha() {
    let alpha = OverlayLayer::BaseFrame.default_alpha();
    assert_eq!(alpha, 1.0);

    let alpha = OverlayLayer::Grid.default_alpha();
    assert!(alpha > 0.0 && alpha < 1.0);
}

#[test]
fn test_overlay_config_creation() {
    let config = OverlayConfig::new(OverlayLayer::Grid);

    assert_eq!(config.layer, OverlayLayer::Grid);
    assert!(config.visible);
    assert!(!config.critical);
    assert_eq!(config.blend_mode, BlendMode::Normal);
}

#[test]
fn test_overlay_config_builder() {
    let config = OverlayConfig::new(OverlayLayer::MotionVectors)
        .with_priority(100)
        .with_alpha(0.8)
        .with_blend_mode(BlendMode::Additive)
        .critical();

    assert_eq!(config.priority, 100);
    assert_eq!(config.alpha, 0.8);
    assert_eq!(config.blend_mode, BlendMode::Additive);
    assert!(config.critical);
}

#[test]
fn test_occlusion_budget_basic() {
    let mut budget = OcclusionBudget::new();

    assert_eq!(budget.overlay_count(), 0);
    assert_eq!(budget.visible_count(), 0);
}

#[test]
fn test_register_and_retrieve_overlay() {
    let mut budget = OcclusionBudget::new();

    let config = OverlayConfig::new(OverlayLayer::Grid);
    budget.register_overlay(config);

    let retrieved = budget.get_overlay(&OverlayLayer::Grid);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().layer, OverlayLayer::Grid);
}

#[test]
fn test_visible_stack_ordering() {
    let mut budget = OcclusionBudget::new();

    // Register overlays with different priorities
    budget.register_overlay(OverlayConfig::new(OverlayLayer::SelectionHighlight).with_priority(70));
    budget.register_overlay(OverlayConfig::new(OverlayLayer::Grid).with_priority(40));
    budget.register_overlay(OverlayConfig::new(OverlayLayer::QpHeatmap).with_priority(20));

    let stack = budget.get_visible_stack();
    assert_eq!(stack.len(), 3);

    // Check ordering (should be sorted by priority)
    assert_eq!(stack[0].layer, OverlayLayer::QpHeatmap);
    assert_eq!(stack[1].layer, OverlayLayer::Grid);
    assert_eq!(stack[2].layer, OverlayLayer::SelectionHighlight);
}

#[test]
fn test_show_hide_overlay() {
    let mut budget = OcclusionBudget::new();

    budget.register_overlay(OverlayConfig::new(OverlayLayer::Grid));

    assert_eq!(budget.visible_count(), 1);

    budget.hide_overlay(&OverlayLayer::Grid);
    assert_eq!(budget.visible_count(), 0);

    budget.show_overlay(&OverlayLayer::Grid);
    assert_eq!(budget.visible_count(), 1);
}
