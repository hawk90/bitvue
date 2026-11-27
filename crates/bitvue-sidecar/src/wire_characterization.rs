//! Characterization of the sidecar's wire error replies (#556). Pins, byte for byte, the control
//! frame every command answers with when its params fail to parse, its stream id is unknown, or
//! its stream is not open -- captured on the pre-refactor code, so that moving the shared
//! parse/stream-bytes boilerplate into `command_support` provably changes nothing on the wire.
//! Goldens live in `wire_characterization.golden`, one `method|scenario|reply` line each.
//! Regenerate (only on purpose) with `BITVUE_DUMP_GOLDEN=1`.

use crate::request_dispatch::compute_frames;
use crate::test_support::{fresh_compare_state, fresh_debug_yuv_state};
use bitvue_engine::Core;
use bitvue_protocol::{FrameKind, Request};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const GOLDEN: &str = include_str!("wire_characterization.golden");

/// Every method that parses params, in a stable order.
const PARAM_METHODS: &[&str] = &[
    "hello",
    "open_stream",
    "close_stream",
    "select_frame",
    "select_unit",
    "select_syntax",
    "select_bit_range",
    "select_spatial_block",
    "index_stream",
    "get_stream_info",
    "get_frames_chunk",
    "get_frame_syntax",
    "get_timeline",
    "get_hex_range",
    "get_decoded_frame_yuv",
    "get_thumbnails",
    "get_frame_analysis",
    "get_av1_features",
    "get_coding_flow_analysis",
    "get_deblocking_analysis",
    "get_codec_extended_info",
    "get_residual_analysis",
    "get_context_menu_items",
    "export_evidence_bundle",
    "load_debug_yuv",
    "set_debug_yuv_offset",
    "set_debug_yuv_crop",
    "get_yuv_diff_metrics",
    "get_debug_yuv_frame",
    "get_aligned_frame",
    "set_sync_mode",
    "set_manual_offset",
    "get_diff_frame",
];

/// Methods that read stream bytes (stream A, or the `stream` param) and so can answer
/// "stream not open".
const BYTE_METHODS: &[&str] = &[
    "get_hex_range",
    "get_decoded_frame_yuv",
    "get_thumbnails",
    "get_frame_analysis",
    "get_av1_features",
    "get_hrd_info",
    "get_coding_flow_analysis",
    "get_deblocking_analysis",
    "get_codec_extended_info",
    "get_residual_analysis",
];

fn call(core: &Core, method: &str, params: serde_json::Value) -> String {
    let request = Request {
        id: 9,
        method: method.to_string(),
        params,
    };
    let frames = compute_frames(
        core,
        &fresh_debug_yuv_state(),
        &fresh_compare_state(),
        &Arc::new(crate::decode_session::DecodeSessions::new()),
        &request,
        &AtomicBool::new(false),
    );
    assert_eq!(frames.len(), 1, "{method}: error replies are one frame");
    assert_eq!(frames[0].0, FrameKind::Control);
    String::from_utf8(frames[0].1.clone()).unwrap()
}

fn all_lines() -> Vec<String> {
    let core = Core::new();
    let mut lines = Vec::new();
    for m in PARAM_METHODS {
        for (tag, params) in [
            ("junk-string", serde_json::json!("junk")),
            ("empty-object", serde_json::json!({})),
            (
                "wrong-type",
                serde_json::json!({"stream": 5, "frame_index": "x"}),
            ),
        ] {
            lines.push(format!("{m}|{tag}|{}", call(&core, m, params)));
        }
    }
    let valid = serde_json::json!({
        "stream": "A", "frame_index": 0, "offset": 0, "len": 4, "frame_indices": [0]
    });
    for m in BYTE_METHODS {
        lines.push(format!("{m}|not-open|{}", call(&core, m, valid.clone())));
        let mut bad = valid.clone();
        bad["stream"] = serde_json::json!("C");
        lines.push(format!("{m}|bad-stream-id|{}", call(&core, m, bad)));
    }
    lines
}

#[test]
fn wire_error_replies_match_golden() {
    let actual = all_lines();
    if std::env::var_os("BITVUE_DUMP_GOLDEN").is_some() {
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/wire_characterization.golden"
            ),
            actual.join("\n") + "\n",
        )
        .unwrap();
        return;
    }
    let golden: Vec<&str> = GOLDEN.lines().collect();
    assert_eq!(golden.len(), actual.len(), "case count changed");
    for (g, a) in golden.iter().zip(&actual) {
        assert_eq!(g, a);
    }
}
