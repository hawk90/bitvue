//! Fuzzing target for AV1 decoding
//!
//! This fuzzing target tests AV1 decoder robustness against
//! malformed, corrupted, and malicious input data.

#![no_main]

use libfuzzer_sys::fuzz_target;
use bitvue_decode::Av1Decoder;

fuzz_target!(|data: &[u8]| {
    // Should never crash on any input
    if let Ok(mut decoder) = Av1Decoder::new() {
        // Try to decode - may succeed or fail, but must not crash
        let _ = decoder.decode_all(data);
    }

    // Test raw OBU path
    if data.len() > 4 {
        // Check if it might be IVF (has header)
        let is_ivf = data.len() >= 32 && &data[0..4] == b"DKIF";

        if !is_ivf {
            // Treat as raw OBU
            if let Ok(mut decoder) = Av1Decoder::new() {
                let _ = decoder.send_data(data, Some(0));
                let _ = decoder.get_frame();
            }
        }
    }

    // Test with empty data
    if data.is_empty() {
        if let Ok(mut decoder) = Av1Decoder::new() {
            let _ = decoder.decode_all(&[]);
        }
    }

    // Test with very small data
    if data.len() <= 4 {
        if let Ok(mut decoder) = Av1Decoder::new() {
            let _ = decoder.decode_all(data);
        }
    }
});
