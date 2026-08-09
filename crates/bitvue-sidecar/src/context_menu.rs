//! `get_context_menu_items` command backing -- right-click menu items + guard-evaluated
//! enabled/disabled state for one UI scope. Feeds a new frontend `ContextMenu` component (Phase
//! 7.6, `docs/DEVELOPMENT_PHASES.md`).
//!
//! Pure wiring: `bitvue_engine::build_context_menu`/`evaluate_context_menu_guard` (`export::
//! context_menu`) already implement the full catalog (5 scopes: Player/HexView/StreamView/
//! Timeline/DiagnosticsPanel -- more than the doc's original "3 scopes" starter set) and guard
//! engine (3 guards: always/has_selection/has_byte_range). Stateless -- doesn't touch `Core` at
//! all, unlike every other command in this crate.
//!
//! **Found while wiring this**: `bitvue_engine::parity_harness::context` defines an entirely
//! separate, never-actually-wired `ContextMenuScope`/`ContextMenuItem`/`evaluate_guard` (same
//! guard IDs and disabled-reason strings, different shapes -- `ContextMenuScope` there is a
//! `{id, items}` struct, not a fixed enum, and there's no catalog-building equivalent to
//! `build_context_menu`). Its only consumer anywhere in this workspace is its own unit test
//! (`bitvue-engine/src/tests/parity_harness.rs`) -- real, redundant dead code, not something this
//! command depends on. Imports here go through `bitvue_engine::export::` specifically (not the
//! crate root) to sidestep the resulting name-clash ambiguity; not otherwise touched, since
//! deduplicating it wasn't in scope for this pass.

// `export::context_menu`/`export::types` are private submodules (only their contents are
// re-exported); `bitvue_engine::export::` disambiguates the crate-root name clash between this
// module's `ContextMenuScope` (an enum) and `parity_harness::context`'s unrelated, never-wired
// struct of the same name (see this module's doc).
use bitvue_engine::export::{build_context_menu, ContextMenuScope, GuardEvalContext};
use bitvue_protocol::{Request, Response, WireError, WireErrorCode};

#[derive(serde::Deserialize)]
struct GetContextMenuItemsParams {
    scope: ContextMenuScope,
    #[serde(default)]
    has_selection: bool,
    #[serde(default)]
    has_byte_range: bool,
}

pub fn get_context_menu_items(request: &Request) -> Response {
    let params: GetContextMenuItemsParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };

    let context = GuardEvalContext {
        has_selection: params.has_selection,
        has_byte_range: params.has_byte_range,
    };
    let items = build_context_menu(params.scope, &context);

    Response::success(request.id, serde_json::json!({ "items": items }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_context_menu_items_player_scope_returns_real_items() {
        let request = Request {
            id: 1,
            method: "get_context_menu_items".to_string(),
            params: serde_json::json!({
                "scope": "Player",
                "has_selection": false,
                "has_byte_range": false,
            }),
        };
        let response = get_context_menu_items(&request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        let items = result["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        // "toggle_detail" guarded by has_selection=false -> disabled.
        let toggle_detail = items.iter().find(|i| i["id"] == "toggle_detail").unwrap();
        assert_eq!(toggle_detail["enabled"], false);
        assert!(!toggle_detail["disabled_reason"].is_null());
        // "export_bundle" guarded by "always" -> always enabled.
        let export_bundle = items.iter().find(|i| i["id"] == "export_bundle").unwrap();
        assert_eq!(export_bundle["enabled"], true);
        assert!(export_bundle["disabled_reason"].is_null());
    }

    #[test]
    fn get_context_menu_items_has_selection_enables_selection_guarded_items() {
        let request = Request {
            id: 2,
            method: "get_context_menu_items".to_string(),
            params: serde_json::json!({
                "scope": "Player",
                "has_selection": true,
                "has_byte_range": false,
            }),
        };
        let response = get_context_menu_items(&request);
        let result = response.result.unwrap();
        let items = result["items"].as_array().unwrap();
        let toggle_detail = items.iter().find(|i| i["id"] == "toggle_detail").unwrap();
        assert_eq!(toggle_detail["enabled"], true);
    }

    #[test]
    fn get_context_menu_items_hex_view_scope_returns_real_items() {
        let request = Request {
            id: 3,
            method: "get_context_menu_items".to_string(),
            params: serde_json::json!({
                "scope": "HexView",
                "has_selection": false,
                "has_byte_range": true,
            }),
        };
        let response = get_context_menu_items(&request);
        let result = response.result.unwrap();
        let items = result["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let copy_bytes = items.iter().find(|i| i["id"] == "copy_bytes").unwrap();
        assert_eq!(copy_bytes["enabled"], true);
    }

    #[test]
    fn get_context_menu_items_invalid_scope_is_invalid_data() {
        let request = Request {
            id: 4,
            method: "get_context_menu_items".to_string(),
            params: serde_json::json!({ "scope": "NotARealScope" }),
        };
        let response = get_context_menu_items(&request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }
}
