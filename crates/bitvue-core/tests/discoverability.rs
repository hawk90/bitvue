#![allow(dead_code)]
//! Tests for discoverability module

use bitvue_core::{
    ContextualHint, DiscoverabilitySystem, DiscoverableAction, HintContext, HintMode,
    KeyboardShortcut, MouseGesture,
};

#[test]
fn test_keyboard_shortcut_format() {
    let simple = KeyboardShortcut::simple("Space".to_string());
    assert_eq!(simple.format(), "Space");

    let complex = KeyboardShortcut::new("G".to_string(), vec!["Ctrl".to_string()]);
    assert_eq!(complex.format(), "Ctrl+G");
}

#[test]
fn test_mouse_gesture_format() {
    assert_eq!(MouseGesture::Click.format(), "Click");
    assert_eq!(MouseGesture::Drag.format(), "Drag");
    assert_eq!(
        MouseGesture::ClickWith(vec!["Ctrl".to_string()]).format(),
        "Ctrl+Click"
    );
}

#[test]
fn test_contextual_hint_creation() {
    let hint = ContextualHint::new(
        DiscoverableAction::TimelineFrameStep,
        "Step to next frame".to_string(),
        HintContext::Timeline,
    )
    .with_keyboard(KeyboardShortcut::simple("Right".to_string()));

    assert!(hint.available);
    assert!(hint.keyboard.is_some());
}

#[test]
fn test_discoverability_system_basic() {
    let mut system = DiscoverabilitySystem::new();

    assert_eq!(system.mode(), HintMode::Contextual);
    system.set_mode(HintMode::OnDemand);
    assert_eq!(system.mode(), HintMode::OnDemand);
}

#[test]
fn test_hint_registration_and_retrieval() {
    let mut system = DiscoverabilitySystem::new();

    let hint = ContextualHint::new(
        DiscoverableAction::TimelineFrameStep,
        "Step frame".to_string(),
        HintContext::Timeline,
    );

    system.register_hint(hint);

    let retrieved = system.get_hint(&DiscoverableAction::TimelineFrameStep);
    assert!(retrieved.is_some());
}

#[test]
fn test_usage_tracking() {
    let mut system = DiscoverabilitySystem::new();
    let action = DiscoverableAction::TimelineFrameStep;

    assert_eq!(system.action_usage_count(&action), 0);

    system.record_action_usage(&action);
    assert_eq!(system.action_usage_count(&action), 1);

    system.record_action_usage(&action);
    assert_eq!(system.action_usage_count(&action), 2);
}
