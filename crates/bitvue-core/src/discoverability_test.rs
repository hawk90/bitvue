// Discoverability module tests
use super::*;

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_action() -> DiscoverableAction {
    DiscoverableAction::TimelineFrameStep
}

fn create_test_hint() -> ContextualHint {
    ContextualHint::new(
        DiscoverableAction::TimelineFrameStep,
        "Step forward one frame".to_string(),
        HintContext::Timeline,
    )
}

fn create_test_system() -> DiscoverabilitySystem {
    DiscoverabilitySystem::new()
}

// ============================================================================
// HintMode Tests
// ============================================================================
#[cfg(test)]
mod hint_mode_tests {
    use super::*;

    #[test]
    fn test_default_is_contextual() {
        assert_eq!(HintMode::default(), HintMode::Contextual);
    }
}

// ============================================================================
// KeyboardShortcut Tests
// ============================================================================
#[cfg(test)]
mod keyboard_shortcut_tests {
    use super::*;

    #[test]
    fn test_new_creates_shortcut() {
        let kb = KeyboardShortcut::new("Space".to_string(), vec!["Ctrl".to_string()]);
        assert_eq!(kb.key, "Space");
        assert_eq!(kb.modifiers.len(), 1);
    }

    #[test]
    fn test_simple_creates_shortcut() {
        let kb = KeyboardShortcut::simple("G".to_string());
        assert_eq!(kb.key, "G");
        assert!(kb.modifiers.is_empty());
    }

    #[test]
    fn test_format() {
        let kb = KeyboardShortcut::new("S".to_string(), vec!["Ctrl".to_string(), "Shift".to_string()]);
        assert_eq!(kb.format(), "Ctrl+Shift+S");
    }

    #[test]
    fn test_format_simple() {
        let kb = KeyboardShortcut::simple("Escape".to_string());
        assert_eq!(kb.format(), "Escape");
    }
}

// ============================================================================
// ContextualHint Tests
// ============================================================================
#[cfg(test)]
mod contextual_hint_tests {
    use super::*;

    #[test]
    fn test_new_creates_hint() {
        let hint = create_test_hint();
        assert!(hint.available);
        assert!(hint.keyboard.is_none());
    }

    #[test]
    fn test_with_keyboard() {
        let hint = create_test_hint().with_keyboard(KeyboardShortcut::simple("G".to_string()));
        assert!(hint.keyboard.is_some());
    }

    #[test]
    fn test_unavailable() {
        let hint = create_test_hint().unavailable("File not loaded".to_string());
        assert!(!hint.available);
        assert_eq!(hint.unavailable_reason, Some("File not loaded".to_string()));
    }

    #[test]
    fn test_format() {
        let hint = create_test_hint()
            .with_keyboard(KeyboardShortcut::simple("G".to_string()))
            .with_mouse(MouseGesture::Click);
        let formatted = hint.format();
        assert!(formatted.contains("Step Frame"));
        assert!(formatted.contains("(G or Click)"));
    }
}

// ============================================================================
// DiscoverabilitySystem Tests
// ============================================================================
#[cfg(test)]
mod discoverability_system_tests {
    use super::*;

    #[test]
    fn test_new_creates_system() {
        let system = create_test_system();
        assert_eq!(system.mode(), HintMode::Contextual);
    }

    #[test]
    fn test_set_mode() {
        let mut system = create_test_system();
        system.set_mode(HintMode::Disabled);
        assert_eq!(system.mode(), HintMode::Disabled);
    }

    #[test]
    fn test_register_hint() {
        let mut system = create_test_system();
        let hint = create_test_hint();
        system.register_hint(hint);
        assert!(system.get_hint(&create_test_action()).is_some());
    }

    #[test]
    fn test_should_show_hint_contextual() {
        let mut system = create_test_system();
        let hint = create_test_hint();
        system.register_hint(hint);
        assert!(system.should_show_hint(&create_test_action()));
    }

    #[test]
    fn test_should_show_hint_disabled() {
        let mut system = create_test_system();
        system.set_mode(HintMode::Disabled);
        let hint = create_test_hint();
        system.register_hint(hint);
        assert!(!system.should_show_hint(&create_test_action()));
    }

    #[test]
    fn test_mark_hint_seen() {
        let mut system = create_test_system();
        system.mark_hint_seen(&create_test_action());
        // After seeing, should not show if usage count is high
        for _ in 0..5 {
            system.record_action_usage(&create_test_action());
        }
        assert!(!system.should_show_hint(&create_test_action()));
    }

    #[test]
    fn test_record_action_usage() {
        let mut system = create_test_system();
        assert_eq!(system.action_usage_count(&create_test_action()), 0);
        system.record_action_usage(&create_test_action());
        assert_eq!(system.action_usage_count(&create_test_action()), 1);
    }

    #[test]
    fn test_update_hint_availability() {
        let mut system = create_test_system();
        system.register_hint(create_test_hint());
        system.update_hint_availability(&create_test_action(), false, Some("Reason".to_string()));
        let hint = system.get_hint(&create_test_action()).unwrap();
        assert!(!hint.available);
    }

    #[test]
    fn test_get_available_hints() {
        let mut system = create_test_system();
        system.register_hint(create_test_hint());
        let hints = system.get_available_hints();
        assert!(!hints.is_empty());
    }
}
