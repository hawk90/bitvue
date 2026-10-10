#![no_main]

use bitvue_av1_codec::{parse_all_obus, parse_all_obus_resilient, ObuIterator};
use bitvue_engine::StreamId;
use libfuzzer_sys::fuzz_target;

// Walks arbitrary bytes as an OBU stream. Must terminate and never panic,
// including on truncated or inconsistent OBU sizes (the `ObuIterator`
// infinite-loop regression class).
fuzz_target!(|data: &[u8]| {
    let mut steps = 0usize;
    for _ in ObuIterator::new(data) {
        steps += 1;
        // Every step must consume input, so it can never exceed the byte count.
        assert!(steps <= data.len() + 1, "ObuIterator did not advance");
    }
    let _ = parse_all_obus(data);
    let _ = parse_all_obus_resilient(data, StreamId::A);
});
