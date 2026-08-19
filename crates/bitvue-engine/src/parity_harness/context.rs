//! Context menus and guard evaluation

use serde::{Deserialize, Serialize};

// Re-export EntityRef from parent for use in this module
pub use super::EntityRef;

/// Context menu scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuScope {
    pub id: String,
    pub items: Vec<ContextMenuItem>,
}

/// Context menu item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuItem {
    pub id: String,
    pub label: String,
    pub command: String,
    pub guard: String,
}

/// Guard definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardDefinition {
    pub id: String,
    pub expr: String,
    pub disabled_reason: String,
}

/// Guard evaluation context
#[derive(Debug, Clone)]
pub struct GuardContext {
    pub selected_entity: Option<EntityRef>,
    pub selected_byte_range: Option<(u64, u64)>,
}

/// Evaluate a guard
pub fn evaluate_guard(guard_id: &str, context: &GuardContext) -> GuardResult {
    match guard_id {
        "always" => GuardResult {
            enabled: true,
            disabled_reason: None,
        },
        "has_selection" => {
            if context.selected_entity.is_some() {
                GuardResult {
                    enabled: true,
                    disabled_reason: None,
                }
            } else {
                GuardResult {
                    enabled: false,
                    disabled_reason: Some("No selection.".to_string()),
                }
            }
        }
        "has_byte_range" => {
            if context.selected_byte_range.is_some() {
                GuardResult {
                    enabled: true,
                    disabled_reason: None,
                }
            } else {
                GuardResult {
                    enabled: false,
                    disabled_reason: Some("No byte range selected.".to_string()),
                }
            }
        }
        _ => GuardResult {
            enabled: false,
            disabled_reason: Some(format!("Unknown guard: {}", guard_id)),
        },
    }
}

/// Guard evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResult {
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}
