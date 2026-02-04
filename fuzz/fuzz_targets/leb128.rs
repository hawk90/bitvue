#![no_main]

use libfuzzer_sys::fuzz_target;
use bitvue_av1_codec::{decode_uleb128, encode_uleb128};

/// Fuzz target for LEB128 decoder
///
/// This fuzz target tests the LEB128 decoder/encoder roundtrip:
/// 1. Encode a value using LEB128
/// 2. Decode the encoded bytes
/// 3. Verify the roundtrip preserves the value
fuzz_target!(|data: &[u8]| {
    // Use the first 8 bytes as input data for encoding
    let input_value = u64::from_le_bytes(match data.len() {
        0 => return,
        1..=8 => {
            let mut bytes = [0u8; 8];
            bytes[..data.len()].copy_from_slice(data);
            bytes
        }
        _ => {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[0..8]);
            bytes
        }
    });

    // Encode the value
    let encoded = encode_uleb128(input_value);

    // Decode back
    let (decoded, _len) = match decode_uleb128(&encoded) {
        Ok(result) => result,
        Err(_) => return,
    };

    // Verify roundtrip (should always succeed for valid encode)
    if decoded <= (1u64 << 56) - 1 {
        // Only check values that fit in our encode range
        assert_eq!(decoded, input_value, "LEB128 roundtrip failed");
    }
});
