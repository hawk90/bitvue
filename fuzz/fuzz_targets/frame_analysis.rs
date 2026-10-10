#![no_main]

use bitvue_av1_codec::overlay_extraction::ParsedFrame;
use bitvue_av1_codec::{
    extract_mv_grid_from_parsed, extract_partition_grid_from_parsed,
    extract_prediction_mode_grid_from_parsed, extract_qp_grid_from_parsed,
    extract_transform_grid_from_parsed,
};
use libfuzzer_sys::fuzz_target;

// Sequence header + frame header + tile data parsing and every overlay
// extractor (the symbol decoder / coding-unit parser path).
fuzz_target!(|data: &[u8]| {
    let Ok(parsed) = ParsedFrame::parse(data) else {
        return;
    };
    let _ = extract_qp_grid_from_parsed(&parsed, 0, 128);
    let _ = extract_mv_grid_from_parsed(&parsed);
    let _ = extract_partition_grid_from_parsed(&parsed);
    let _ = extract_prediction_mode_grid_from_parsed(&parsed);
    let _ = extract_transform_grid_from_parsed(&parsed);
});
