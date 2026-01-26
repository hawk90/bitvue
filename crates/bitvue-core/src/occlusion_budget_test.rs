// Occlusion Budget module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test overlay config
fn create_test_config(layer: OverlayLayer, priority: u32) -> OverlayConfig {
    OverlayConfig::new(layer, priority)
}

/// Create a test overlay config with alpha
fn create_test_config_with_alpha(layer: OverlayLayer, priority: u32, alpha: f32) -> OverlayConfig {
    OverlayConfig::new(layer, priority).with_alpha(alpha)
}

/// Create a test overlay config with all options
fn create_test_config_full(
    layer: OverlayLayer,
    priority: u32,
    alpha: f32,
    critical: bool,
) -> OverlayConfig {
    let mut config = OverlayConfig::new(layer, priority);
    config.alpha = alpha;
    config.critical = critical;
    config
}

/// Create a test occlusion budget
fn create_test_budget() -> OcclusionBudget {
    OcclusionBudget::new()
}

// ============================================================================
// OverlayLayer Tests
// ============================================================================

#[cfg(test)]
mod overlay_layer_tests {
    use super::*;

    #[test]
    fn test_overlay_layer_values() {
        // Assert - all layer variants exist
        let _ = OverlayLayer::CodingFlow;
        let _ = OverlayLayer::PredictionModes;
        let _ = OverlayLayer::TransformTree;
        let _ = OverlayLayer::QPMap;
        let _ = OverlayLayer::MVField;
        let _ = OverlayLayer::ReferenceFrames;
        let _ = OverlayLayer::Deblocking;
        let _ = OverlayLayer::SAO;
        let _ = OverlayLayer::ALF;
        let _ = OverlayLayer::LoopRestoration;
    }

    #[test]
    fn test_overlay_layer_copy() {
        // Arrange & Act
        let layer = OverlayLayer::QPMap;

        // Assert - OverlayLayer should be Copy
        let _layer_copy = layer;
        let _ = layer; // Should still be usable
    }
}

// ============================================================================
// BlendMode Tests
// ============================================================================

#[cfg(test)]
mod blend_mode_tests {
    use super::*;

    #[test]
    fn test_blend_mode_values() {
        // Assert - all blend modes exist
        let _ = BlendMode::Normal;
        let _ = BlendMode::Multiply;
        let _ = BlendMode::Screen;
        let _ = BlendMode::Overlay;
    }

    #[test]
    fn test_blend_mode_copy() {
        // Arrange & Act
        let mode = BlendMode::Normal;

        // Assert - BlendMode should be Copy
        let _mode_copy = mode;
        let _ = mode;
    }
}

// ============================================================================
// OverlayConfig Tests
// ============================================================================

#[cfg(test)]
mod overlay_config_tests {
    use super::*;

    #[test]
    fn test_new_creates_config() {
        // Arrange & Act
        let config = OverlayConfig::new(OverlayLayer::QPMap, 5);

        // Assert
        assert_eq!(config.layer, OverlayLayer::QPMap);
        assert_eq!(config.priority, 5);
        assert_eq!(config.alpha, 1.0); // Default alpha
        assert!(!config.critical); // Default critical
        assert_eq!(config.blend_mode, BlendMode::Normal); // Default blend
    }

    #[test]
    fn test_with_alpha_sets_alpha() {
        // Arrange
        let config = OverlayConfig::new(OverlayLayer::CodingFlow, 10);

        // Act
        let config = config.with_alpha(0.5);

        // Assert
        assert_eq!(config.alpha, 0.5);
    }

    #[test]
    fn test_with_critical_sets_critical() {
        // Arrange
        let config = OverlayConfig::new(OverlayLayer::MVField, 1);

        // Act
        let config = config.with_critical(true);

        // Assert
        assert!(config.critical);
    }

    #[test]
    fn test_with_blend_mode_sets_blend_mode() {
        // Arrange
        let config = OverlayConfig::new(OverlayLayer::SAO, 3);

        // Act
        let config = config.with_blend_mode(BlendMode::Multiply);

        // Assert
        assert_eq!(config.blend_mode, BlendMode::Multiply);
    }

    #[test]
    fn test_config_clone() {
        // Arrange
        let config = OverlayConfig::new(OverlayLayer::QPMap, 5)
            .with_alpha(0.7)
            .with_critical(true);

        // Act
        let config_clone = config.clone();

        // Assert
        assert_eq!(config_clone.layer, config.layer);
        assert_eq!(config_clone.priority, config.priority);
        assert_eq!(config_clone.alpha, config.alpha);
        assert_eq!(config_clone.critical, config.critical);
    }
}

// ============================================================================
// OcclusionBudget Construction Tests
// ============================================================================

#[cfg(test)]
mod occlusion_budget_construction_tests {
    use super::*;

    #[test]
    fn test_new_creates_budget() {
        // Arrange & Act
        let budget = create_test_budget();

        // Assert
        assert!(budget.overlays.is_empty());
        assert!(!budget.auto_adjust_alpha);
        assert_eq!(budget.max_overlays, 10);
    }

    #[test]
    fn test_default_creates_budget() {
        // Arrange & Act
        let budget = OcclusionBudget::default();

        // Assert
        assert!(budget.overlays.is_empty());
    }

    #[test]
    fn test_with_max_overlays() {
        // Arrange
        let mut budget = OcclusionBudget::new();

        // Act
        budget.max_overlays = 5;

        // Assert
        assert_eq!(budget.max_overlays, 5);
    }

    #[test]
    fn test_with_auto_adjust_alpha() {
        // Arrange
        let mut budget = OcclusionBudget::new();

        // Act
        budget.auto_adjust_alpha = true;

        // Assert
        assert!(budget.auto_adjust_alpha);
    }
}

// ============================================================================
// OcclusionBudget Register Tests
// ============================================================================

#[cfg(test)]
mod occlusion_budget_register_tests {
    use super::*;

    #[test]
    fn test_register_overlay_adds() {
        // Arrange
        let mut budget = create_test_budget();
        let config = create_test_config(OverlayLayer::QPMap, 5);

        // Act
        budget.register_overlay(config);

        // Assert
        assert_eq!(budget.overlays.len(), 1);
        assert_eq!(budget.overlays[0].layer, OverlayLayer::QPMap);
    }

    #[test]
    fn test_register_multiple_overlays() {
        // Arrange
        let mut budget = create_test_budget();

        // Act
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 3));
        budget.register_overlay(create_test_config(OverlayLayer::CodingFlow, 1));

        // Assert
        assert_eq!(budget.overlays.len(), 3);
    }

    #[test]
    fn test_register_same_layer_replaces() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));

        // Act
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 10));

        // Assert
        assert_eq!(budget.overlays.len(), 1);
        assert_eq!(budget.overlays[0].priority, 10);
    }

    #[test]
    fn test_register_over_max_removes_lowest_priority() {
        // Arrange
        let mut budget = create_test_budget();
        budget.max_overlays = 3;

        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 3));
        budget.register_overlay(create_test_config(OverlayLayer::CodingFlow, 1));

        // Act - Add 4th overlay with low priority
        budget.register_overlay(create_test_config(OverlayLayer::TransformTree, 2));

        // Assert - Should have 3 overlays, lowest priority (CodingFlow:1) removed
        assert_eq!(budget.overlays.len(), 3);
        assert!(!budget.overlays.iter().any(|c| c.layer == OverlayLayer::CodingFlow));
    }
}

// ============================================================================
// OcclusionBudget Show/Hide Tests
// ============================================================================

#[cfg(test)]
mod occlusion_budget_visibility_tests {
    use super::*;

    #[test]
    fn test_show_overlay() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));

        // Act
        budget.show_overlay(OverlayLayer::QPMap);

        // Assert
        assert!(budget.overlays[0].visible);
    }

    #[test]
    fn test_hide_overlay() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.overlays[0].visible = true;

        // Act
        budget.hide_overlay(OverlayLayer::QPMap);

        // Assert
        assert!(!budget.overlays[0].visible);
    }

    #[test]
    fn test_show_nonexistent_overlay_no_panic() {
        // Arrange
        let mut budget = create_test_budget();

        // Act - Should not panic
        budget.show_overlay(OverlayLayer::QPMap);

        // Assert - No overlay added
        assert!(budget.overlays.is_empty());
    }

    #[test]
    fn test_hide_nonexistent_overlay_no_panic() {
        // Arrange
        let mut budget = create_test_budget();

        // Act - Should not panic
        budget.hide_overlay(OverlayLayer::QPMap);

        // Assert
        assert!(budget.overlays.is_empty());
    }

    #[test]
    fn test_toggle_visibility() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        let initial_visible = budget.overlays[0].visible;

        // Act
        budget.show_overlay(OverlayLayer::QPMap);
        let after_show = budget.overlays[0].visible;

        budget.hide_overlay(OverlayLayer::QPMap);
        let after_hide = budget.overlays[0].visible;

        // Assert
        assert!(!initial_visible);
        assert!(after_show);
        assert!(!after_hide);
    }
}

// ============================================================================
// OcclusionBudget Stack Tests
// ============================================================================

#[cfg(test)]
mod occlusion_budget_stack_tests {
    use super::*;

    #[test]
    fn test_get_visible_stack_empty() {
        // Arrange
        let budget = create_test_budget();

        // Act
        let stack = budget.get_visible_stack();

        // Assert
        assert!(stack.is_empty());
    }

    #[test]
    fn test_get_visible_stack_only_visible() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 3));
        budget.register_overlay(create_test_config(OverlayLayer::CodingFlow, 1));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = false;
        budget.overlays[2].visible = true;

        // Act
        let stack = budget.get_visible_stack();

        // Assert
        assert_eq!(stack.len(), 2);
        assert!(stack.iter().all(|c| c.visible));
    }

    #[test]
    fn test_get_visible_stack_sorted_by_priority() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 3));
        budget.register_overlay(create_test_config(OverlayLayer::CodingFlow, 1));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = true;
        budget.overlays[2].visible = true;

        // Act
        let stack = budget.get_visible_stack();

        // Assert - Should be sorted by priority (descending: 5, 3, 1)
        assert_eq!(stack[0].priority, 5);
        assert_eq!(stack[1].priority, 3);
        assert_eq!(stack[2].priority, 1);
    }

    #[test]
    fn test_get_compositing_stack_empty() {
        // Arrange
        let budget = create_test_budget();

        // Act
        let stack = budget.get_compositing_stack();

        // Assert
        assert!(stack.is_empty());
    }

    #[test]
    fn test_get_compositing_stack_single_no_adjustment() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::QPMap, 5, 0.8));
        budget.overlays[0].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0].alpha, 0.8); // No adjustment
    }

    #[test]
    fn test_get_compositing_stack_multiple_with_auto_adjust() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = true;

        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::QPMap, 5, 1.0));
        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::MVField, 3, 1.0));
        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::CodingFlow, 1, 1.0));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = true;
        budget.overlays[2].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert - Alpha should be adjusted (1.0 / sqrt(3) ≈ 0.577)
        let expected_alpha = (1.0 / (3.0_f32).sqrt()).clamp(0.1, 1.0);
        assert!((stack[0].alpha - expected_alpha).abs() < 0.01);
        assert!((stack[1].alpha - expected_alpha).abs() < 0.01);
        assert!((stack[2].alpha - expected_alpha).abs() < 0.01);
    }

    #[test]
    fn test_get_compositing_stack_critical_not_adjusted() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = true;

        budget.register_overlay(create_test_config_full(OverlayLayer::QPMap, 5, 1.0, true));
        budget.register_overlay(create_test_config_full(OverlayLayer::MVField, 3, 1.0, false));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert - Critical overlay should keep alpha=1.0
        assert_eq!(stack[0].alpha, 1.0); // Critical
        assert!(stack[1].alpha < 1.0); // Adjusted
    }

    #[test]
    fn test_get_compositing_stack_no_auto_adjust() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = false; // Disabled

        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::QPMap, 5, 0.9));
        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::MVField, 3, 0.8));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert - No alpha adjustment
        assert_eq!(stack[0].alpha, 0.9);
        assert_eq!(stack[1].alpha, 0.8);
    }

    #[test]
    fn test_get_compositing_stack_clamps_minimum() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = true;

        // Add 100 overlays - alpha would be very low
        for i in 0..100 {
            let mut config = create_test_config_with_alpha(OverlayLayer::QPMap, i, 1.0);
            config.visible = true;
            budget.overlays.push(config);
        }

        // Act
        let stack = budget.get_compositing_stack();

        // Assert - Alpha should be clamped to 0.1 minimum
        assert!(stack.iter().all(|c| c.alpha >= 0.1));
    }

    #[test]
    fn test_get_compositing_stack_clamps_maximum() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = true;

        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::QPMap, 5, 2.0));
        budget.overlays[0].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert - Alpha should be clamped to 1.0 maximum
        assert!(stack[0].alpha <= 1.0);
    }
}

// ============================================================================
// OcclusionBudget Query Tests
// ============================================================================

#[cfg(test)]
mod occlusion_budget_query_tests {
    use super::*;

    #[test]
    fn test_get_config_exists() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));

        // Act
        let config = budget.get_config(OverlayLayer::QPMap);

        // Assert
        assert!(config.is_some());
        assert_eq!(config.unwrap().layer, OverlayLayer::QPMap);
    }

    #[test]
    fn test_get_config_not_exists() {
        // Arrange
        let budget = create_test_budget();

        // Act
        let config = budget.get_config(OverlayLayer::QPMap);

        // Assert
        assert!(config.is_none());
    }

    #[test]
    fn test_is_overlay_registered_true() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));

        // Act
        let is_registered = budget.is_overlay_registered(OverlayLayer::QPMap);

        // Assert
        assert!(is_registered);
    }

    #[test]
    fn test_is_overlay_registered_false() {
        // Arrange
        let budget = create_test_budget();

        // Act
        let is_registered = budget.is_overlay_registered(OverlayLayer::QPMap);

        // Assert
        assert!(!is_registered);
    }

    #[test]
    fn test_is_visible_true() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.overlays[0].visible = true;

        // Act
        let is_visible = budget.is_visible(OverlayLayer::QPMap);

        // Assert
        assert!(is_visible);
    }

    #[test]
    fn test_is_visible_false() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.overlays[0].visible = false;

        // Act
        let is_visible = budget.is_visible(OverlayLayer::QPMap);

        // Assert
        assert!(!is_visible);
    }

    #[test]
    fn test_is_visible_not_registered() {
        // Arrange
        let budget = create_test_budget();

        // Act
        let is_visible = budget.is_visible(OverlayLayer::QPMap);

        // Assert
        assert!(!is_visible);
    }

    #[test]
    fn test_get_overlay_count() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 3));
        budget.register_overlay(create_test_config(OverlayLayer::CodingFlow, 1));

        // Act
        let count = budget.get_overlay_count();

        // Assert
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_visible_count() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 3));
        budget.register_overlay(create_test_config(OverlayLayer::CodingFlow, 1));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = false;
        budget.overlays[2].visible = true;

        // Act
        let count = budget.get_visible_count();

        // Assert
        assert_eq!(count, 2);
    }
}

// ============================================================================
// OcclusionBudget Remove Tests
// ============================================================================

#[cfg(test)]
mod occlusion_budget_remove_tests {
    use super::*;

    #[test]
    fn test_unregister_overlay_removes() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));

        // Act
        budget.unregister_overlay(OverlayLayer::QPMap);

        // Assert
        assert_eq!(budget.overlays.len(), 0);
    }

    #[test]
    fn test_unregister_nonexistent_no_panic() {
        // Arrange
        let mut budget = create_test_budget();

        // Act - Should not panic
        budget.unregister_overlay(OverlayLayer::QPMap);

        // Assert
        assert_eq!(budget.overlays.len(), 0);
    }

    #[test]
    fn test_clear_all_overlays() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 3));

        // Act
        budget.clear_all();

        // Assert
        assert!(budget.overlays.is_empty());
    }

    #[test]
    fn test_clear_all_resets_state() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = true;
        budget.max_overlays = 20;
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));

        // Act
        budget.clear_all();

        // Assert
        assert!(budget.overlays.is_empty());
        assert!(budget.auto_adjust_alpha); // Should preserve
        assert_eq!(budget.max_overlays, 20); // Should preserve
    }
}

// ============================================================================
// OcclusionBudget Statistics Tests
// ============================================================================

#[cfg(test)]
mod occlusion_budget_stats_tests {
    use super::*;

    #[test]
    fn test_get_stats_empty() {
        // Arrange
        let budget = create_test_budget();

        // Act
        let stats = budget.get_stats();

        // Assert
        assert_eq!(stats.total_overlays, 0);
        assert_eq!(stats.visible_overlays, 0);
        assert_eq!(stats.hidden_overlays, 0);
    }

    #[test]
    fn test_get_stats_with_overlays() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 3));
        budget.register_overlay(create_test_config(OverlayLayer::CodingFlow, 1));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = true;
        budget.overlays[2].visible = false;

        // Act
        let stats = budget.get_stats();

        // Assert
        assert_eq!(stats.total_overlays, 3);
        assert_eq!(stats.visible_overlays, 2);
        assert_eq!(stats.hidden_overlays, 1);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_zero_alpha_allowed() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::QPMap, 5, 0.0));
        budget.overlays[0].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert - Zero alpha should be allowed (overlay is invisible but present)
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0].alpha, 0.0);
    }

    #[test]
    fn test_negative_alpha_clamped() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config_with_alpha(OverlayLayer::QPMap, 5, -0.5));
        budget.overlays[0].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert - Negative alpha should be clamped to 0.1 minimum
        assert!(stack[0].alpha >= 0.1);
    }

    #[test]
    fn test_same_priority_multiple_overlays() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.register_overlay(create_test_config(OverlayLayer::MVField, 5));
        budget.register_overlay(create_test_config(OverlayLayer::CodingFlow, 5));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = true;
        budget.overlays[2].visible = true;

        // Act
        let stack = budget.get_visible_stack();

        // Assert - All should be in stack (order may vary for same priority)
        assert_eq!(stack.len(), 3);
    }

    #[test]
    fn test_max_overlays_zero() {
        // Arrange
        let mut budget = create_test_budget();
        budget.max_overlays = 0;

        // Act
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));

        // Assert - Behavior may vary, but shouldn't crash
        // (implementation may still add overlay or may enforce limit)
    }

    #[test]
    fn test_empty_visible_stack() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.overlays[0].visible = false;

        // Act
        let stack = budget.get_visible_stack();

        // Assert
        assert!(stack.is_empty());
    }

    #[test]
    fn test_all_critical_overlays_no_adjustment() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = true;

        budget.register_overlay(create_test_config_full(OverlayLayer::QPMap, 5, 1.0, true));
        budget.register_overlay(create_test_config_full(OverlayLayer::MVField, 3, 1.0, true));
        budget.register_overlay(create_test_config_full(OverlayLayer::CodingFlow, 1, 1.0, true));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = true;
        budget.overlays[2].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert - All critical overlays should keep alpha=1.0
        assert!(stack.iter().all(|c| c.alpha == 1.0));
    }

    #[test]
    fn test_mixed_critical_and_normal_adjustment() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = true;

        budget.register_overlay(create_test_config_full(OverlayLayer::QPMap, 5, 1.0, true));
        budget.register_overlay(create_test_config_full(OverlayLayer::MVField, 3, 1.0, false));
        budget.register_overlay(create_test_config_full(OverlayLayer::CodingFlow, 1, 1.0, false));

        budget.overlays[0].visible = true;
        budget.overlays[1].visible = true;
        budget.overlays[2].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert
        assert_eq!(stack[0].alpha, 1.0); // Critical
        assert!(stack[1].alpha < 1.0); // Normal - adjusted
        assert!(stack[2].alpha < 1.0); // Normal - adjusted
    }

    #[test]
    fn test_single_overlay_critical() {
        // Arrange
        let mut budget = create_test_budget();
        budget.auto_adjust_alpha = true;

        budget.register_overlay(create_test_config_full(OverlayLayer::QPMap, 5, 1.0, true));
        budget.overlays[0].visible = true;

        // Act
        let stack = budget.get_compositing_stack();

        // Assert
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0].alpha, 1.0);
    }

    #[test]
    fn test_re_register_after_unregister() {
        // Arrange
        let mut budget = create_test_budget();
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 5));
        budget.unregister_overlay(OverlayLayer::QPMap);

        // Act
        budget.register_overlay(create_test_config(OverlayLayer::QPMap, 10));

        // Assert
        assert_eq!(budget.overlays.len(), 1);
        assert_eq!(budget.overlays[0].priority, 10);
    }

    #[test]
    fn test_all_blend_modes() {
        // Arrange & Act
        let config1 = OverlayConfig::new(OverlayLayer::QPMap, 5)
            .with_blend_mode(BlendMode::Normal);
        let config2 = OverlayConfig::new(OverlayLayer::MVField, 3)
            .with_blend_mode(BlendMode::Multiply);
        let config3 = OverlayConfig::new(OverlayLayer::CodingFlow, 1)
            .with_blend_mode(BlendMode::Screen);
        let config4 = OverlayConfig::new(OverlayLayer::TransformTree, 2)
            .with_blend_mode(BlendMode::Overlay);

        // Assert
        assert_eq!(config1.blend_mode, BlendMode::Normal);
        assert_eq!(config2.blend_mode, BlendMode::Multiply);
        assert_eq!(config3.blend_mode, BlendMode::Screen);
        assert_eq!(config4.blend_mode, BlendMode::Overlay);
    }
}
