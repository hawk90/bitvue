//! Discoverability System - discoverability.001
//!
//! Per FRAME_IDENTITY_CONTRACT:
//! - Contextual hints for keyboard/mouse shortcuts
//! - Never blocks workflow (non-intrusive)
//! - Respects disable-with-reason matrix
//! - Does not reduce pro power

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Hint display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HintMode {
    /// Show hints contextually (default for new users)
    #[default]
    Contextual,

    /// Show hints on explicit request only (for pros)
    OnDemand,

    /// Never show hints (expert mode)
    Disabled,
}

/// Input action that can have a hint
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscoverableAction {
    // Timeline navigation
    TimelineFrameStep,
    TimelineFrameJump,
    TimelinePan,
    TimelineZoom,
    TimelineMarkerToggle,

    // Selection
    SelectFrame,
    SelectRange,
    SelectBlock,
    SelectionClear,

    // Overlays
    OverlayToggleGrid,
    OverlayToggleMv,
    OverlayToggleQp,
    OverlayTogglePartition,

    // Compare
    CompareSideBySide,
    CompareDiffHeatmap,

    // View control
    ViewFitToWindow,
    ViewActualSize,
    ViewZoomIn,
    ViewZoomOut,

    // Custom action
    Custom(String),
}

impl DiscoverableAction {
    /// Get display name for action
    pub fn name(&self) -> String {
        match self {
            DiscoverableAction::TimelineFrameStep => "Step Frame".to_string(),
            DiscoverableAction::TimelineFrameJump => "Jump to Frame".to_string(),
            DiscoverableAction::TimelinePan => "Pan Timeline".to_string(),
            DiscoverableAction::TimelineZoom => "Zoom Timeline".to_string(),
            DiscoverableAction::TimelineMarkerToggle => "Toggle Marker".to_string(),
            DiscoverableAction::SelectFrame => "Select Frame".to_string(),
            DiscoverableAction::SelectRange => "Select Range".to_string(),
            DiscoverableAction::SelectBlock => "Select Block".to_string(),
            DiscoverableAction::SelectionClear => "Clear Selection".to_string(),
            DiscoverableAction::OverlayToggleGrid => "Toggle Grid Overlay".to_string(),
            DiscoverableAction::OverlayToggleMv => "Toggle Motion Vectors".to_string(),
            DiscoverableAction::OverlayToggleQp => "Toggle QP Heatmap".to_string(),
            DiscoverableAction::OverlayTogglePartition => "Toggle Partition Grid".to_string(),
            DiscoverableAction::CompareSideBySide => "Side-by-Side Compare".to_string(),
            DiscoverableAction::CompareDiffHeatmap => "Diff Heatmap".to_string(),
            DiscoverableAction::ViewFitToWindow => "Fit to Window".to_string(),
            DiscoverableAction::ViewActualSize => "Actual Size".to_string(),
            DiscoverableAction::ViewZoomIn => "Zoom In".to_string(),
            DiscoverableAction::ViewZoomOut => "Zoom Out".to_string(),
            DiscoverableAction::Custom(name) => name.clone(),
        }
    }
}

/// Keyboard shortcut
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardShortcut {
    /// Key (e.g., "Space", "Left", "G")
    pub key: String,

    /// Modifiers (Ctrl, Shift, Alt, Cmd)
    pub modifiers: Vec<String>,
}

impl KeyboardShortcut {
    /// Create new shortcut
    pub fn new(key: String, modifiers: Vec<String>) -> Self {
        Self { key, modifiers }
    }

    /// Create simple shortcut (no modifiers)
    pub fn simple(key: String) -> Self {
        Self {
            key,
            modifiers: vec![],
        }
    }

    /// Format as human-readable string
    pub fn format(&self) -> String {
        if self.modifiers.is_empty() {
            self.key.clone()
        } else {
            format!("{}+{}", self.modifiers.join("+"), self.key)
        }
    }
}

/// Mouse gesture
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseGesture {
    /// Click
    Click,

    /// Double click
    DoubleClick,

    /// Right click
    RightClick,

    /// Middle click
    MiddleClick,

    /// Drag
    Drag,

    /// Scroll
    Scroll,

    /// Click with modifiers
    ClickWith(Vec<String>),

    /// Drag with modifiers
    DragWith(Vec<String>),
}

impl MouseGesture {
    /// Format as human-readable string
    pub fn format(&self) -> String {
        match self {
            MouseGesture::Click => "Click".to_string(),
            MouseGesture::DoubleClick => "Double Click".to_string(),
            MouseGesture::RightClick => "Right Click".to_string(),
            MouseGesture::MiddleClick => "Middle Click".to_string(),
            MouseGesture::Drag => "Drag".to_string(),
            MouseGesture::Scroll => "Scroll".to_string(),
            MouseGesture::ClickWith(mods) => format!("{}+Click", mods.join("+")),
            MouseGesture::DragWith(mods) => format!("{}+Drag", mods.join("+")),
        }
    }
}

/// Contextual hint for an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualHint {
    /// Action this hint is for
    pub action: DiscoverableAction,

    /// Human-readable description
    pub description: String,

    /// Keyboard shortcut (if any)
    pub keyboard: Option<KeyboardShortcut>,

    /// Mouse gesture (if any)
    pub mouse: Option<MouseGesture>,

    /// Context where this hint applies
    pub context: HintContext,

    /// Whether this action is currently available
    pub available: bool,

    /// Reason why action is unavailable (if any)
    pub unavailable_reason: Option<String>,
}

impl ContextualHint {
    /// Create new hint
    pub fn new(action: DiscoverableAction, description: String, context: HintContext) -> Self {
        Self {
            action,
            description,
            keyboard: None,
            mouse: None,
            context,
            available: true,
            unavailable_reason: None,
        }
    }

    /// Set keyboard shortcut
    pub fn with_keyboard(mut self, shortcut: KeyboardShortcut) -> Self {
        self.keyboard = Some(shortcut);
        self
    }

    /// Set mouse gesture
    pub fn with_mouse(mut self, gesture: MouseGesture) -> Self {
        self.mouse = Some(gesture);
        self
    }

    /// Mark as unavailable with reason
    pub fn unavailable(mut self, reason: String) -> Self {
        self.available = false;
        self.unavailable_reason = Some(reason);
        self
    }

    /// Format hint as display string
    pub fn format(&self) -> String {
        let mut parts = vec![self.action.name()];

        // Add shortcuts
        let mut shortcuts = vec![];
        if let Some(ref kb) = self.keyboard {
            shortcuts.push(kb.format());
        }
        if let Some(ref mouse) = self.mouse {
            shortcuts.push(mouse.format());
        }
        if !shortcuts.is_empty() {
            parts.push(format!("({})", shortcuts.join(" or ")));
        }

        // Add availability
        if !self.available {
            if let Some(ref reason) = self.unavailable_reason {
                parts.push(format!("- {}", reason));
            } else {
                parts.push("- Unavailable".to_string());
            }
        }

        parts.join(" ")
    }
}

/// Context where hint applies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HintContext {
    /// Timeline panel
    Timeline,

    /// Player panel
    Player,

    /// Compare panel
    Compare,

    /// Tree panel
    Tree,

    /// Metrics panel
    Metrics,

    /// Global (anywhere)
    Global,

    /// Custom context
    Custom(String),
}

/// Discoverability system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverabilitySystem {
    /// Current hint mode
    mode: HintMode,

    /// Registered hints
    hints: HashMap<DiscoverableAction, ContextualHint>,

    /// Whether user has seen each hint
    seen_hints: HashMap<DiscoverableAction, bool>,

    /// Number of times each action was used
    action_usage_count: HashMap<DiscoverableAction, u32>,
}

impl DiscoverabilitySystem {
    /// Create new discoverability system
    pub fn new() -> Self {
        Self {
            mode: HintMode::default(),
            hints: HashMap::new(),
            seen_hints: HashMap::new(),
            action_usage_count: HashMap::new(),
        }
    }

    /// Set hint mode
    pub fn set_mode(&mut self, mode: HintMode) {
        self.mode = mode;
    }

    /// Get current mode
    pub fn mode(&self) -> HintMode {
        self.mode
    }

    /// Register a hint
    pub fn register_hint(&mut self, hint: ContextualHint) {
        self.hints.insert(hint.action.clone(), hint);
    }

    /// Get hint for action
    pub fn get_hint(&self, action: &DiscoverableAction) -> Option<&ContextualHint> {
        self.hints.get(action)
    }

    /// Get all hints for a context
    pub fn get_context_hints(&self, context: &HintContext) -> Vec<&ContextualHint> {
        self.hints
            .values()
            .filter(|hint| &hint.context == context)
            .collect()
    }

    /// Check if hint should be shown
    pub fn should_show_hint(&self, action: &DiscoverableAction) -> bool {
        match self.mode {
            HintMode::Disabled => false,
            HintMode::OnDemand => false, // Only show on explicit request
            HintMode::Contextual => {
                // Show if hint exists, is available, and hasn't been seen many times
                if let Some(hint) = self.hints.get(action) {
                    if !hint.available {
                        return false;
                    }

                    // Show hint if user hasn't seen it or hasn't used the action much
                    let seen = self.seen_hints.get(action).copied().unwrap_or(false);
                    let usage = self.action_usage_count.get(action).copied().unwrap_or(0);

                    // Show hint if not seen, or if user hasn't used action (they might have forgotten)
                    !seen || usage < 3
                } else {
                    false
                }
            }
        }
    }

    /// Mark hint as seen
    pub fn mark_hint_seen(&mut self, action: &DiscoverableAction) {
        self.seen_hints.insert(action.clone(), true);
    }

    /// Record action usage
    pub fn record_action_usage(&mut self, action: &DiscoverableAction) {
        let count = self.action_usage_count.entry(action.clone()).or_insert(0);
        *count += 1;
    }

    /// Get action usage count
    pub fn action_usage_count(&self, action: &DiscoverableAction) -> u32 {
        self.action_usage_count.get(action).copied().unwrap_or(0)
    }

    /// Get all available hints (respects hint mode)
    pub fn get_available_hints(&self) -> Vec<&ContextualHint> {
        match self.mode {
            HintMode::Disabled => vec![],
            HintMode::OnDemand => {
                // Return all hints (user requested them explicitly)
                self.hints.values().collect()
            }
            HintMode::Contextual => {
                // Return hints that should be shown
                self.hints
                    .iter()
                    .filter(|(action, _)| self.should_show_hint(action))
                    .map(|(_, hint)| hint)
                    .collect()
            }
        }
    }

    /// Update hint availability based on feature availability
    pub fn update_hint_availability(
        &mut self,
        action: &DiscoverableAction,
        available: bool,
        reason: Option<String>,
    ) {
        if let Some(hint) = self.hints.get_mut(action) {
            hint.available = available;
            hint.unavailable_reason = reason;
        }
    }

    /// Clear all seen hints (reset learning state)
    pub fn reset_seen_hints(&mut self) {
        self.seen_hints.clear();
    }

    /// Clear usage statistics
    pub fn reset_usage_stats(&mut self) {
        self.action_usage_count.clear();
    }
}

impl Default for DiscoverabilitySystem {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("discoverability_test.rs");
