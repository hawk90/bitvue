//! Context menu support and guard evaluation

use super::types::{ContextMenuItem, ContextMenuScope, GuardEvalContext};

/// Evaluate guard and return enabled state with reason
pub fn evaluate_context_menu_guard(
    guard: &str,
    context: &GuardEvalContext,
) -> (bool, Option<String>) {
    match guard {
        "always" => (true, None),
        "has_selection" => {
            if context.has_selection {
                (true, None)
            } else {
                (false, Some("No selection.".to_string()))
            }
        }
        "has_byte_range" => {
            if context.has_byte_range {
                (true, None)
            } else {
                (false, Some("No byte range selected.".to_string()))
            }
        }
        _ => (false, Some(format!("Unknown guard: {}", guard))),
    }
}

/// Build context menu items for a scope
pub fn build_context_menu(
    scope: ContextMenuScope,
    context: &GuardEvalContext,
) -> Vec<ContextMenuItem> {
    let items = match scope {
        ContextMenuScope::Player => vec![
            (
                "toggle_detail",
                "Details",
                "Toggle.DetailMode",
                "has_selection",
            ),
            (
                "export_bundle",
                "Export Evidence Bundle",
                "Export.EvidenceBundle",
                "always",
            ),
            (
                "copy_selection",
                "Copy Selection",
                "Copy.Selection",
                "has_selection",
            ),
        ],
        ContextMenuScope::HexView => vec![
            ("copy_bytes", "Copy Bytes", "Copy.Bytes", "has_byte_range"),
            (
                "export_bundle",
                "Export Evidence Bundle",
                "Export.EvidenceBundle",
                "always",
            ),
        ],
        ContextMenuScope::StreamView => vec![
            (
                "set_order_display",
                "Compare in Display Order",
                "Set.OrderType.Display",
                "always",
            ),
            (
                "set_order_decode",
                "Compare in Decode Order",
                "Set.OrderType.Decode",
                "always",
            ),
        ],
        ContextMenuScope::Timeline => vec![
            (
                "export_bundle",
                "Export Evidence Bundle",
                "Export.EvidenceBundle",
                "always",
            ),
            (
                "copy_selection",
                "Copy Selection",
                "Copy.Selection",
                "has_selection",
            ),
        ],
        ContextMenuScope::DiagnosticsPanel => vec![
            (
                "export_bundle",
                "Export Evidence Bundle",
                "Export.EvidenceBundle",
                "always",
            ),
            (
                "copy_selection",
                "Copy Selection",
                "Copy.Selection",
                "has_selection",
            ),
        ],
    };

    items
        .into_iter()
        .map(|(id, label, command, guard)| {
            let (enabled, disabled_reason) = evaluate_context_menu_guard(guard, context);
            ContextMenuItem {
                id: id.to_string(),
                label: label.to_string(),
                command: command.to_string(),
                guard: guard.to_string(),
                enabled,
                disabled_reason,
            }
        })
        .collect()
}
