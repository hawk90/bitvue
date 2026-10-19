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

/// One menu entry before its guard is evaluated: `(id, label, command, guard)`.
type MenuSpec = (&'static str, &'static str, &'static str, &'static str);

const EXPORT_BUNDLE: MenuSpec = (
    "export_bundle",
    "Export Evidence Bundle",
    "Export.EvidenceBundle",
    "always",
);
const COPY_SELECTION: MenuSpec = (
    "copy_selection",
    "Copy Selection",
    "Copy.Selection",
    "has_selection",
);

const PLAYER_MENU: [MenuSpec; 3] = [
    (
        "toggle_detail",
        "Details",
        "Toggle.DetailMode",
        "has_selection",
    ),
    EXPORT_BUNDLE,
    COPY_SELECTION,
];

const HEX_VIEW_MENU: [MenuSpec; 4] = [
    ("copy_bytes", "Copy Bytes", "Copy.Bytes", "has_byte_range"),
    (
        "copy_offset",
        "Copy Offset",
        "Copy.Offset",
        "has_byte_range",
    ),
    (
        "copy_bit_range",
        "Copy Bit Range",
        "Copy.BitRange",
        "has_byte_range",
    ),
    EXPORT_BUNDLE,
];

const STREAM_VIEW_MENU: [MenuSpec; 2] = [
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
];

/// Timeline and diagnostics panel share the same two entries.
const EXPORT_AND_COPY_MENU: [MenuSpec; 2] = [EXPORT_BUNDLE, COPY_SELECTION];

fn menu_spec(scope: ContextMenuScope) -> &'static [MenuSpec] {
    match scope {
        ContextMenuScope::Player => &PLAYER_MENU,
        ContextMenuScope::HexView => &HEX_VIEW_MENU,
        ContextMenuScope::StreamView => &STREAM_VIEW_MENU,
        ContextMenuScope::Timeline | ContextMenuScope::DiagnosticsPanel => &EXPORT_AND_COPY_MENU,
    }
}

/// Build context menu items for a scope
pub fn build_context_menu(
    scope: ContextMenuScope,
    context: &GuardEvalContext,
) -> Vec<ContextMenuItem> {
    menu_spec(scope)
        .iter()
        .map(|&(id, label, command, guard)| {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// (id, label, command, guard) of every item, in menu order.
    type Row = (&'static str, &'static str, &'static str, &'static str);

    const EXPORT: Row = (
        "export_bundle",
        "Export Evidence Bundle",
        "Export.EvidenceBundle",
        "always",
    );
    const COPY_SELECTION: Row = (
        "copy_selection",
        "Copy Selection",
        "Copy.Selection",
        "has_selection",
    );

    fn expected(scope: ContextMenuScope) -> Vec<Row> {
        match scope {
            ContextMenuScope::Player => vec![
                (
                    "toggle_detail",
                    "Details",
                    "Toggle.DetailMode",
                    "has_selection",
                ),
                EXPORT,
                COPY_SELECTION,
            ],
            ContextMenuScope::HexView => vec![
                ("copy_bytes", "Copy Bytes", "Copy.Bytes", "has_byte_range"),
                (
                    "copy_offset",
                    "Copy Offset",
                    "Copy.Offset",
                    "has_byte_range",
                ),
                (
                    "copy_bit_range",
                    "Copy Bit Range",
                    "Copy.BitRange",
                    "has_byte_range",
                ),
                EXPORT,
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
            ContextMenuScope::Timeline | ContextMenuScope::DiagnosticsPanel => {
                vec![EXPORT, COPY_SELECTION]
            }
        }
    }

    const SCOPES: [ContextMenuScope; 5] = [
        ContextMenuScope::Player,
        ContextMenuScope::HexView,
        ContextMenuScope::StreamView,
        ContextMenuScope::Timeline,
        ContextMenuScope::DiagnosticsPanel,
    ];

    fn ctx(has_selection: bool, has_byte_range: bool) -> GuardEvalContext {
        GuardEvalContext {
            has_selection,
            has_byte_range,
        }
    }

    #[test]
    fn every_scope_lists_its_items_in_order_with_every_field() {
        for scope in SCOPES {
            for (sel, range) in [(false, false), (true, false), (false, true), (true, true)] {
                let items = build_context_menu(scope, &ctx(sel, range));
                let rows = expected(scope);
                assert_eq!(items.len(), rows.len(), "{scope:?}");
                for (item, (id, label, command, guard)) in items.iter().zip(&rows) {
                    let tag = format!("{scope:?}/{id} sel={sel} range={range}");
                    assert_eq!(
                        (
                            item.id.as_str(),
                            item.label.as_str(),
                            item.command.as_str(),
                            item.guard.as_str()
                        ),
                        (*id, *label, *command, *guard),
                        "{tag}"
                    );
                    let (enabled, reason) = evaluate_context_menu_guard(guard, &ctx(sel, range));
                    assert_eq!(
                        (item.enabled, &item.disabled_reason),
                        (enabled, &reason),
                        "{tag}"
                    );
                }
            }
        }
    }

    #[test]
    fn disabled_items_carry_the_reason_for_their_guard() {
        let none = ctx(false, false);
        let player = build_context_menu(ContextMenuScope::Player, &none);
        let by_id = |id: &str| player.iter().find(|i| i.id == id).unwrap();
        assert!(!by_id("toggle_detail").enabled);
        assert_eq!(
            by_id("toggle_detail").disabled_reason.as_deref(),
            Some("No selection.")
        );
        assert!(by_id("export_bundle").enabled && by_id("export_bundle").disabled_reason.is_none());

        let hex = build_context_menu(ContextMenuScope::HexView, &none);
        for id in ["copy_bytes", "copy_offset", "copy_bit_range"] {
            let item = hex.iter().find(|i| i.id == id).unwrap();
            assert_eq!(
                item.disabled_reason.as_deref(),
                Some("No byte range selected."),
                "{id}"
            );
        }
    }

    #[test]
    fn a_selection_enables_selection_items_but_not_byte_range_items() {
        let items = build_context_menu(ContextMenuScope::Player, &ctx(true, false));
        assert!(items
            .iter()
            .all(|i| i.enabled && i.disabled_reason.is_none()));
        let hex = build_context_menu(ContextMenuScope::HexView, &ctx(true, false));
        assert_eq!(
            hex.iter().filter(|i| i.enabled).count(),
            1,
            "only export_bundle"
        );
    }

    #[test]
    fn stream_view_items_are_always_enabled() {
        let items = build_context_menu(ContextMenuScope::StreamView, &ctx(false, false));
        assert!(items
            .iter()
            .all(|i| i.enabled && i.disabled_reason.is_none()));
    }

    #[test]
    fn guard_evaluation_covers_every_guard_and_unknown_names() {
        assert_eq!(
            evaluate_context_menu_guard("always", &ctx(false, false)),
            (true, None)
        );
        assert_eq!(
            evaluate_context_menu_guard("has_selection", &ctx(true, false)),
            (true, None)
        );
        assert_eq!(
            evaluate_context_menu_guard("has_selection", &ctx(false, true)),
            (false, Some("No selection.".to_string()))
        );
        assert_eq!(
            evaluate_context_menu_guard("has_byte_range", &ctx(false, true)),
            (true, None)
        );
        assert_eq!(
            evaluate_context_menu_guard("has_byte_range", &ctx(true, false)),
            (false, Some("No byte range selected.".to_string()))
        );
        assert_eq!(
            evaluate_context_menu_guard("nope", &ctx(true, true)),
            (false, Some("Unknown guard: nope".to_string()))
        );
        // Names are matched exactly.
        assert!(!evaluate_context_menu_guard("Always", &ctx(true, true)).0);
        assert!(!evaluate_context_menu_guard("", &ctx(true, true)).0);
    }

    #[test]
    fn ids_are_unique_within_a_menu() {
        for scope in SCOPES {
            let items = build_context_menu(scope, &ctx(true, true));
            let mut ids: Vec<_> = items.iter().map(|i| i.id.clone()).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), items.len(), "{scope:?}");
        }
    }
}
