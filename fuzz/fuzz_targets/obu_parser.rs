#![no_main]

use libfuzzer_sys::fuzz_target;
use bitvue_av1::parse_obu;

/// Fuzz target for AV1 OBU parser
///
/// This fuzz target attempts to parse arbitrary bytes as OBU data,
/// testing the robustness of the parser against malformed input.
fuzz_target!(|data: &[u8]| {
    // Attempt to parse the data as OBU
    // The parser should handle all input gracefully
    let _ = parse_obu(data, 0);
});
