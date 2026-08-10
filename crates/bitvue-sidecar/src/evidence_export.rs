//! `export_evidence_bundle` command backing -- writes a diagnostic evidence bundle (manifest,
//! env/version info, selection state, order type, backend fingerprint, warnings) to disk for one
//! of the 4 documented entry points (MainMenu, BottomBar, ContextMenu; CompareWorkspace's is
//! unreachable until that UI is un-excluded -- see `docs/DEVELOPMENT_PHASES.md`'s Phase 7.6
//! section). Feeds `frontend/utils/menu/creators/exportMenu.ts`'s already-defined but previously
//! dead `menu-export-evidence` item.
//!
//! `bitvue_engine::export_evidence_bundle` (`export::evidence`) already writes real files -- this
//! is orchestration (sourcing `EvidenceBundleExportRequest`'s fields from `Core`/`StreamState`
//! and wire params), not new logic. Known scope narrowing inherited from the engine function
//! itself (not introduced here): `include_interaction_trace`/`include_logs` request flags exist
//! but nothing captures interaction traces or logs yet (see `evidence.rs`'s own doc);
//! `render_snapshots` (per-panel render state) requires frontend capture infrastructure that
//! doesn't exist yet, so this command always passes an empty slice -- the bundle's
//! `render_snapshots/` directory is simply omitted, which `export_evidence_bundle` already
//! handles via its own `!render_snapshots.is_empty()` guard.
//!
//! Screenshots ARE captured now (2026-08-11): the frontend calls Electron's `capturePage()` (via
//! `bitvue:captureScreenshot`, `bitvue-desktop/electron/main.ts`) and passes the resulting
//! `data:image/png;base64,...` string as `screenshot_data_url`, decoded here into raw PNG bytes
//! before reaching `export_evidence_bundle`. Optional -- a missing/malformed data URL just means
//! no screenshot in the bundle, not a failed export (matches the frontend's own best-effort
//! capture-failure handling in `useExportEvidenceBundle.ts`).
//!
//! `stream_fingerprint` is a real, reproducible hash (not a placeholder) of the currently open
//! stream A's file path + byte length, using this crate's own established
//! `DefaultHasher`-based fingerprinting convention (see `bitvue_engine::qp_heatmap`/
//! `timeline_cache` for the same pattern) -- not a cryptographic content hash, which would need a
//! new dependency for a use case (tamper-evidence, not cache-keying) this command doesn't need.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use bitvue_engine::parity_harness::OrderType;
use bitvue_engine::{
    export_evidence_bundle, Core, EntityRef, EvidenceBundleExportRequest, SelectionSnapshot,
    StreamId,
};
use bitvue_protocol::{Request, Response, WireError, WireErrorCode};

fn default_workspace() -> String {
    "player".to_string()
}

fn default_mode() -> String {
    "normal".to_string()
}

fn default_order_type() -> OrderType {
    OrderType::Display
}

#[derive(serde::Deserialize)]
struct ExportEvidenceBundleParams {
    output_dir: String,
    #[serde(default = "default_workspace")]
    workspace: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_order_type")]
    order_type: OrderType,
    #[serde(default)]
    selected_entity: Option<EntityRef>,
    #[serde(default)]
    selected_byte_range: Option<(u64, u64)>,
    /// `data:image/png;base64,...` from the frontend's `captureScreenshot()`, or `None` if the
    /// capture failed/was skipped -- see this module's doc.
    #[serde(default)]
    screenshot_data_url: Option<String>,
}

/// Decodes a `data:image/png;base64,<data>` string into raw PNG bytes. Returns `None` (rather
/// than erroring) on any malformed input -- a screenshot is a nice-to-have bundle artifact, not
/// something that should fail the whole export over a decode error.
fn decode_screenshot_data_url(data_url: &str) -> Option<Vec<u8>> {
    let b64 = data_url.strip_prefix("data:image/png;base64,")?;
    STANDARD.decode(b64).ok()
}

/// Real, reproducible fingerprint of stream A's currently open file (path + byte length) --
/// empty string if no stream is open, matching `EvidenceBundleExportRequest::default()`.
fn stream_fingerprint(core: &Core) -> String {
    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let Some(path) = state.file_path.as_ref() else {
        return String::new();
    };
    let len = state.byte_cache.as_ref().map(|c| c.len()).unwrap_or(0);
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    len.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn export_evidence_bundle_command(core: &Core, request: &Request) -> Response {
    let params: ExportEvidenceBundleParams = match serde_json::from_value(request.params.clone()) {
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

    let fingerprint = stream_fingerprint(core);
    let selection_state = SelectionSnapshot {
        selected_entity: params.selected_entity,
        selected_byte_range: params.selected_byte_range,
        order_type: params.order_type,
    };

    let screenshots: Vec<Vec<u8>> = params
        .screenshot_data_url
        .as_deref()
        .and_then(decode_screenshot_data_url)
        .into_iter()
        .collect();

    let bundle_request = EvidenceBundleExportRequest {
        output_dir: std::path::PathBuf::from(params.output_dir),
        include_screenshots: !screenshots.is_empty(),
        include_render_snapshots: false,
        include_interaction_trace: false,
        include_logs: false,
        stream_fingerprint: fingerprint,
        selection_state,
        workspace: params.workspace,
        mode: params.mode,
        order_type: params.order_type,
    };

    let result = export_evidence_bundle(&bundle_request, &[], &screenshots);
    if !result.success {
        return Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::Internal,
                message: result.error.unwrap_or_else(|| "export failed".to_string()),
                offset: None,
            },
        );
    }

    Response::success(request.id, serde_json::json!(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_engine::Core;

    #[test]
    fn export_evidence_bundle_writes_real_files_to_a_temp_dir() {
        let core = Core::new();
        let temp = std::env::temp_dir().join(format!("bitvue_evidence_test_{:016x}", {
            let mut h = DefaultHasher::new();
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .hash(&mut h);
            h.finish()
        }));

        let request = Request {
            id: 1,
            method: "export_evidence_bundle".to_string(),
            params: serde_json::json!({
                "output_dir": temp.to_string_lossy(),
                "workspace": "player",
                "mode": "normal",
                "order_type": "display",
            }),
        };
        let response = export_evidence_bundle_command(&core, &request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["success"], true);
        let bundle_path = result["bundle_path"].as_str().unwrap();
        assert!(std::path::Path::new(bundle_path)
            .join("bundle_manifest.json")
            .exists());
        assert!(std::path::Path::new(bundle_path)
            .join("selection_state.json")
            .exists());

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn export_evidence_bundle_writes_a_real_screenshot_when_given_a_data_url() {
        let core = Core::new();
        let temp = std::env::temp_dir().join(format!("bitvue_evidence_screenshot_test_{:016x}", {
            let mut h = DefaultHasher::new();
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .hash(&mut h);
            h.finish()
        }));

        // Doesn't need to be a real decodable PNG for this test -- only that the exact bytes
        // survive the base64-decode -> file-write round trip unmodified.
        let fake_png_bytes = b"\x89PNG\r\n\x1a\nnot a real png, just recognizable bytes";
        let data_url = format!("data:image/png;base64,{}", STANDARD.encode(fake_png_bytes));

        let request = Request {
            id: 3,
            method: "export_evidence_bundle".to_string(),
            params: serde_json::json!({
                "output_dir": temp.to_string_lossy(),
                "workspace": "player",
                "mode": "normal",
                "order_type": "display",
                "screenshot_data_url": data_url,
            }),
        };
        let response = export_evidence_bundle_command(&core, &request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["success"], true);
        let bundle_path = result["bundle_path"].as_str().unwrap();
        let screenshot_path =
            std::path::Path::new(bundle_path).join("screenshots/screenshot_0000.png");
        assert!(screenshot_path.exists(), "expected a real screenshot file");
        let written_bytes = std::fs::read(&screenshot_path).unwrap();
        assert_eq!(written_bytes, fake_png_bytes);

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn export_evidence_bundle_without_a_screenshot_url_omits_the_screenshots_dir() {
        let core = Core::new();
        let temp =
            std::env::temp_dir().join(format!("bitvue_evidence_no_screenshot_test_{:016x}", {
                let mut h = DefaultHasher::new();
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .hash(&mut h);
                h.finish()
            }));

        let request = Request {
            id: 4,
            method: "export_evidence_bundle".to_string(),
            params: serde_json::json!({
                "output_dir": temp.to_string_lossy(),
                "workspace": "player",
                "mode": "normal",
                "order_type": "display",
            }),
        };
        let response = export_evidence_bundle_command(&core, &request);
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        let bundle_path = result["bundle_path"].as_str().unwrap();
        assert!(
            !std::path::Path::new(bundle_path)
                .join("screenshots")
                .exists(),
            "no screenshot_data_url was given, screenshots/ shouldn't exist"
        );

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn export_evidence_bundle_missing_output_dir_param_is_invalid_data() {
        let core = Core::new();
        let request = Request {
            id: 2,
            method: "export_evidence_bundle".to_string(),
            params: serde_json::json!({}),
        };
        let response = export_evidence_bundle_command(&core, &request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::InvalidData);
    }
}
