//! Corruption generators for testing error handling
//!
//! Provides functions to create corrupted/malformed video data
//! for testing robustness and security.

use crate::generators::*;

/// Types of corruption that can be applied to video data
#[derive(Debug, Clone, Copy)]
pub enum CorruptionType {
    TruncatedHeader,
    TruncatedFrame,
    InvalidMagic,
    FrameSizeOverflow,
    ZeroFrames,
    TooManyFrames,
    CorruptedChecksum,
    RandomGarbage,
    BitFlip,
    ByteFlip,
}

/// Create corrupted IVF with specified corruption type
pub fn create_corrupted_ivf(corruption: CorruptionType) -> Vec<u8> {
    let base = create_minimal_ivf();

    match corruption {
        CorruptionType::TruncatedHeader => {
            let mut truncated = base.clone();
            truncated.truncate(16); // Cut header in half
            truncated
        }
        CorruptionType::TruncatedFrame => {
            let mut truncated = base.clone();
            if truncated.len() > 50 {
                truncated.truncate(50); // Cut during frame data
            }
            truncated
        }
        CorruptionType::InvalidMagic => {
            let mut invalid = base.clone();
            // Corrupt "DKIF" magic
            invalid[0] = 0xFF;
            invalid[1] = 0xFF;
            invalid[2] = 0xFF;
            invalid[3] = 0xFF;
            invalid
        }
        CorruptionType::FrameSizeOverflow => {
            let mut overflow = base.clone();
            // Set frame size to u32::MAX in first frame
            if overflow.len() >= 36 {
                overflow[32] = 0xFF;
                overflow[33] = 0xFF;
                overflow[34] = 0xFF;
                overflow[35] = 0xFF;
            }
            overflow
        }
        CorruptionType::ZeroFrames => {
            let mut zero_frames = create_ivf_header();
            // Set frame count to 0
            zero_frames[24] = 0;
            zero_frames[25] = 0;
            zero_frames[26] = 0;
            zero_frames[27] = 0;
            zero_frames
        }
        CorruptionType::TooManyFrames => {
            let mut many_frames = create_ivf_header();
            // Set frame count to u32::MAX
            many_frames[24] = 0xFF;
            many_frames[25] = 0xFF;
            many_frames[26] = 0xFF;
            many_frames[27] = 0xFF;
            // Only provide 1 actual frame
            many_frames.extend_from_slice(&1000u32.to_le_bytes());
            many_frames.extend_from_slice(&0u64.to_le_bytes());
            many_frames.extend_from_slice(&vec![0u8; 1000]);
            many_frames
        }
        CorruptionType::CorruptedChecksum => {
            // IVF doesn't have checksums, but we can simulate
            let mut corrupted = base.clone();
            // Corrupt some data near the end
            if let Some(last) = corrupted.last_mut() {
                *last ^= 0xFF;
            }
            corrupted
        }
        CorruptionType::RandomGarbage => {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let size = 1000;
            (0..size).map(|_| rng.gen()).collect()
        }
        CorruptionType::BitFlip => {
            apply_bit_flips(&base, 0.01) // 1% of bits flipped
        }
        CorruptionType::ByteFlip => {
            apply_byte_flips(&base, 0.1) // 10% of bytes flipped
        }
    }
}

/// Apply random bit flips to data
pub fn apply_bit_flips(data: &[u8], flip_rate: f64) -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    data.iter()
        .map(|&byte| {
            let mut flipped = byte;
            for bit in 0..8 {
                if rng.gen_bool(flip_rate / 8.0) {
                    flipped ^= 1 << bit;
                }
            }
            flipped
        })
        .collect()
}

/// Apply random byte flips to data
pub fn apply_byte_flips(data: &[u8], flip_rate: f64) -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    data.iter()
        .map(|&byte| {
            if rng.gen_bool(flip_rate) {
                byte ^ 0xFF
            } else {
                byte
            }
        })
        .collect()
}

/// Create video with corrupted last frame
pub fn create_ivf_with_corrupted_last_frame() -> Vec<u8> {
    let mut data = create_ivf_with_n_frames(5, 320, 240);

    // Find position of last frame data and corrupt it
    if data.len() > 100 {
        let last_100 = data.len() - 100;
        for i in last_100..data.len() {
            data[i] ^= 0xFF;
        }
    }

    data
}

/// Create IVF with invalid checksum
pub fn create_ivf_with_invalid_checksum() -> Vec<u8> {
    let mut data = create_minimal_ivf();

    // IVF doesn't have checksums, but we can add corruption
    if let Some(last) = data.last_mut() {
        *last ^= 0xFF;
    }

    data
}

/// Create video with frame at specific offset corrupted
pub fn create_corrupted_video_at_offset(offset: usize) -> Vec<u8> {
    let mut data = create_minimal_ivf();

    if offset < data.len() {
        data[offset] ^= 0xFF;
        if offset + 1 < data.len() {
            data[offset + 1] ^= 0xFF;
        }
    }

    data
}

/// Create IVF with frame size mismatch
pub fn create_ivf_with_frame_size_mismatch() -> Vec<u8> {
    let mut data = create_ivf_header();

    // Claim frame is 1000 bytes
    data.extend_from_slice(&1000u32.to_le_bytes());
    data.extend_from_slice(&0u64.to_le_bytes());

    // But only provide 100 bytes
    data.extend_from_slice(&vec![0u8; 100]);

    data
}

/// Create video with transient errors
pub fn create_video_with_transient_errors() -> Vec<u8> {
    let mut data = create_ivf_with_n_frames(10, 320, 240);

    // Corrupt frames 3, 5, 7
    let frame_size = 1012; // 12 byte header + 1000 byte data

    for frame_idx in [3, 5, 7] {
        let offset = 32 + frame_idx * frame_size;
        if offset + 10 < data.len() {
            data[offset + 10] ^= 0xFF;
            data[offset + 11] ^= 0xFF;
        }
    }

    data
}

/// Create video with suspicious but valid data
pub fn create_video_with_suspicious_but_valid_data() -> Vec<u8> {
    // All zeros is valid but unusual
    let mut data = create_ivf_with_n_frames(10, 320, 240);

    // Zero out frame data
    for byte in data.iter_mut().skip(32) {
        *byte = 0;
    }

    data
}

/// Create deeply nested OBU for testing stack overflow
pub fn create_deeply_nested_obu(depth: usize) -> Vec<u8> {
    let mut data = Vec::new();

    // Create nested OBUs
    for _ in 0..depth {
        // Start a new OBU (not valid in reality, but tests nesting)
        data.push(0x01); // Sequence header type
        data.push(0x80); // Continuation bit
    }

    // Close all OBUs
    for _ in 0..depth {
        data.push(0x00);
    }

    data
}

/// Create Exp-Golomb value with excessive leading zeros
pub fn create_exp_golomb_with_excessive_zeros(num_zeros: u32) -> Vec<u8> {
    let mut data = Vec::new();

    // Add leading zeros
    for _ in 0..num_zeros {
        data.push(0x00); // Zero bit
    }

    // Add stop bit and some data
    data.push(0x80); // Stop bit (1) with 7 zeros

    data
}

/// Create OBU with length field overflow
pub fn create_obu_with_length_overflow() -> Vec<u8> {
    let mut data = vec![0x08]; // OBU type

    // Set length to very large value using leb128
    data.push(0x80); // Continuation
    data.push(0x80); // Continuation
    data.push(0x80); // Continuation
    data.push(0x01); // Value = 2^21

    // But only provide small amount of data
    data.extend_from_slice(&[0u8; 100]);

    data
}

/// Create malformed OBU header
pub fn create_malformed_obu_with_infinite_loop() -> Vec<u8> {
    let mut data = vec![0x08]; // OBU type

    // Invalid length field that never terminates
    data.push(0x80); // Continuation
    data.push(0x80); // Continuation
    data.push(0x80); // Continuation
    data.push(0x80); // Continuation
    // ... would continue indefinitely

    data.extend_from_slice(&[0u8; 100]);

    data
}

/// Create chroma plane with size mismatch attack
pub fn create_frame_with_chroma_mismatch() -> Vec<u8> {
    // This would need to be a full frame, not just raw data
    // Return the Y plane data that would be mismatched
    vec![128u8; 1920 * 1080]
}

/// Create VP9 superframe with corrupted index
pub fn create_vp9_superframe_with_corrupted_index() -> Vec<u8> {
    let frames = vec![
        create_vp9_frame(),
        create_vp9_frame(),
    ];

    let mut data = Vec::new();

    // Add frames
    for frame in &frames {
        data.extend_from_slice(frame);
    }

    // Corrupted superframe index
    data.extend_from_slice(&[0x00, 0x00, 0x01]); // Wrong magic
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF]); // Corrupted sizes

    data
}

/// Create MP4 with negative CTTS
pub fn create_mp4_with_negative_ctts() -> Vec<u8> {
    let mut data = create_mp4_ftyp(b"isom");

    // Add moov with CTTS box
    // CTTS = Composition Time to Sample
    // Can have negative values

    data
}

/// Create MP4 with extended ESDS
pub fn create_mp4_with_extended_esds() -> Vec<u8> {
    let mut data = create_mp4_ftyp(b"isom");

    // Add moov with extended ES_Desr tag

    data
}

/// Create H.264 slice with missing data
pub fn create_frame_with_missing_slices() -> Vec<u8> {
    let mut data = create_sps_with_params(77, 0);

    // Add PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x08]);

    // Add incomplete slice header
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x05]);
    data.extend_from_slice(&[0u8; 50]); // Truncated slice data

    data
}

/// Create AV1 with truncated SEI
pub fn create_av1_with_truncated_sei() -> Vec<u8> {
    let mut data = create_seq_header_with_profile_level(0, 0);

    // Add metadata OBU (SEI equivalent)
    let header = ObuHeader {
        obu_type: 5, // Metadata
        has_extension: false,
        has_size_field: true,
        forbidden: false,
    };

    data.extend_from_slice(&header.to_bytes(Some(100)));
    // But only provide 10 bytes
    data.extend_from_slice(&[0u8; 10]);

    data
}

/// Create test data for bitreader edge cases
pub mod bitreader {
    /// Create data that causes Exp-Golomb overflow
    pub fn exp_golomb_overflow() -> Vec<u8> {
        vec![0u8; 1000] // All zeros = infinite leading zeros
    }

    /// Create data for LEB128 overflow
    pub fn leb128_overflow() -> Vec<u8> {
        vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]
        // All continuation bits set, never terminates
    }

    /// Create data for skip overflow
    pub fn skip_overflow() -> Vec<u8> {
        vec![0u8; 100]
    }
}

/// Create frame with invalid dimensions
pub fn create_frame_with_invalid_dimensions() -> bitvue_decode::DecodedFrame {
    bitvue_decode::DecodedFrame {
        width: 0,
        height: 1080,
        bit_depth: 8,
        y_plane: vec![].into_boxed_slice().into(),
        y_stride: 0,
        u_plane: None,
        u_stride: 0,
        v_plane: None,
        v_stride: 0,
        timestamp: 0,
        frame_type: bitvue_decode::FrameType::Key,
        qp_avg: None,
        chroma_format: bitvue_decode::ChromaFormat::Monochrome,
    }
}

/// Create frame with strided planes
pub fn create_frame_with_strided_planes() -> bitvue_decode::DecodedFrame {
    bitvue_decode::DecodedFrame {
        width: 320,
        height: 240,
        bit_depth: 8,
        y_plane: vec![0u8; 320 * 300].into_boxed_slice().into(), // Stride of 320, but 300 rows
        y_stride: 320,
        u_plane: None,
        u_stride: 0,
        v_plane: None,
        v_stride: 0,
        timestamp: 0,
        frame_type: bitvue_decode::FrameType::Key,
        qp_avg: None,
        chroma_format: bitvue_decode::ChromaFormat::Monochrome,
    }
}

/// Create frame with null planes
pub fn create_frame_with_null_planes() -> bitvue_decode::DecodedFrame {
    bitvue_decode::DecodedFrame {
        width: 320,
        height: 240,
        bit_depth: 8,
        y_plane: vec![].into_boxed_slice().into(),
        y_stride: 0,
        u_plane: None,
        u_stride: 0,
        v_plane: None,
        v_stride: 0,
        timestamp: 0,
        frame_type: bitvue_decode::FrameType::Key,
        qp_avg: None,
        chroma_format: bitvue_decode::ChromaFormat::Monochrome,
    }
}
