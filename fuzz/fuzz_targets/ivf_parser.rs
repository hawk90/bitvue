#![no_main]

use libfuzzer_sys::fuzz_target;
use bitvue_av1_codec::{is_ivf, parse_ivf_header, parse_ivf_frames};

/// Fuzz target for IVF parser
///
/// This fuzz target attempts to parse arbitrary bytes as IVF data,
/// testing the robustness of the parser against malformed input.
fuzz_target!(|data: &[u8]| {
    // Check if data looks like IVF format
    if data.len() < 32 {
        return;
    }

    // Attempt to parse IVF header
    let _ = parse_ivf_header(data);

    // Attempt to parse IVF frames
    let _ = parse_ivf_frames(data);

    // The is_ivf check should also not panic
    let _ = is_ivf(data);
});
