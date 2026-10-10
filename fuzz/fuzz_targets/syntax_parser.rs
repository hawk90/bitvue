#![no_main]

use bitvue_av1_codec::{parse_bitstream_syntax, parse_obu_syntax};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parse_bitstream_syntax(data);
    let _ = parse_obu_syntax(data, 0, 0);
});
