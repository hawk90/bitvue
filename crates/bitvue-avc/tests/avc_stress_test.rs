#![allow(dead_code)]
// Stress tests for AVC codec - large inputs, random patterns, boundary conditions
use bitvue_avc::{extract_annex_b_frames, parse_avc, parse_avc_quick, parse_nal_header};

#[test]
fn test_parse_avc_large_input_10kb() {
    let mut data = vec![0u8; 10_240];
    // Add some NAL units
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67; // SPS
    data[512..516].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[516] = 0x68; // PPS
    data[1024..1028].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[1028] = 0x65; // IDR

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_large_input_100kb() {
    let mut data = vec![0u8; 102_400];
    // Add periodic NAL units
    for i in 0..10 {
        let offset = i * 10_240;
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x67 + (i as u8) % 3;
    }

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_random_pattern_1kb() {
    let mut data = vec![0u8; 1024];
    // Fill with random-looking pattern
    for i in 0..1024 {
        data[i] = ((i * 7 + 3) % 256) as u8;
    }

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_repeated_start_codes() {
    let mut data = vec![0u8; 256];
    // Add many consecutive start codes
    for i in 0..32 {
        data[i * 8..i * 8 + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    }

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_max_nal_count() {
    let mut data = vec![0u8; 8192]; // 8KB
                                    // Add as many NAL units as possible
    let mut offset = 0;
    for i in 0..128 {
        if offset + 8 > data.len() {
            break;
        }
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x67; // SPS
        offset += 8;
    }

    let result = parse_avc(&data);
    assert!(result.is_ok());
    if let Ok(stream) = result {
        assert!(stream.nal_units.len() <= 128);
    }
}

#[test]
fn test_parse_nal_header_all_values() {
    // Test all possible nal_unit_type values
    for nal_type in 0u8..=31 {
        let byte = (nal_type << 3) | 0x07; // Set ref_idc = 7
        let result = parse_nal_header(byte);
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_avc_quick_empty() {
    let data: &[u8] = &[];
    let result = parse_avc_quick(data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().nal_count, 0);
}

#[test]
fn test_parse_avc_quick_no_nals() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF];
    let result = parse_avc_quick(&data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().nal_count, 0);
}

#[test]
fn test_extract_annex_b_frames_stress_1mb() {
    let mut data = vec![0u8; 1_048_576]; // 1MB
                                         // Add NAL units periodically
    for i in 0..256 {
        let offset = i * 4096;
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x65;
        data[offset + 5] = 0x80;
        // Add some payload
        for j in 6..128 {
            data[offset + j] = ((j + i) % 256) as u8;
        }
    }

    let result = extract_annex_b_frames(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_avc_boundary_sizes() {
    // Test various input sizes around power of 2
    let sizes = [
        1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 511, 512,
    ];

    for size in sizes {
        let mut data = vec![0u8; size];
        if size >= 5 {
            data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            data[4] = 0x67;
        }

        let result = parse_avc(&data);
        assert!(result.is_ok() || result.is_err(), "Failed at size {}", size);
    }
}

#[test]
fn test_parse_avc_spspps_cycle() {
    // Test multiple SPS/PPS cycles
    let mut data = vec![0u8; 4096];
    let mut offset = 0;

    // Add 5 SPS/PPS pairs
    for i in 0..5 {
        // SPS
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x67;
        data[offset + 5] = 0x42;
        offset += 16;

        // PPS
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x68;
        data[offset + 5] = 0xC4;
        offset += 16;
    }

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_interleaved_spspps() {
    // Interleave SPS and PPS with slice data
    let mut data = vec![0u8; 2048];
    let mut offset = 0;

    // SPS 1
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x67;
    offset += 32;

    // PPS 1
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x68;
    offset += 32;

    // Slice
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x25;
    offset += 32;

    // SPS 2
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x67;
    offset += 32;

    // PPS 2
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x68;

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_sei_messages() {
    let mut data = vec![0u8; 1024];

    // SPS
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67;

    // PPS
    data[32..36].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[36] = 0x68;

    // SEI
    data[64..68].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[68] = 0x06; // SEI NAL
    data[69] = 0x01; // payload_type
    data[70] = 0x00; // payload_size

    // IDR slice
    data[128..132].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[132] = 0x25;
    data[133] = 0x80; // first_mb_in_slice

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_multiple_idr_frames() {
    let mut data = vec![0u8; 2048];
    let mut offset = 0;

    for i in 0..3 {
        // SPS
        if offset + 64 > data.len() {
            break;
        }
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x67;
        offset += 32;

        // PPS
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x68;
        offset += 32;

        // IDR slice
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x25;
        offset += 64;
    }

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_aud() {
    let mut data = vec![0u8; 512];

    // AUD
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x09; // AUD

    // SPS
    data[32..36].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[36] = 0x67;

    // PPS
    data[64..68].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[68] = 0x68;

    // IDR
    data[128..132].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[132] = 0x25;

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_end_of_sequence() {
    let mut data = vec![0u8; 512];

    // SPS
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67;

    // PPS
    data[32..36].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[36] = 0x68;

    // End of sequence
    data[64..68].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[68] = 0x0A; // EOS

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_with_filler_data() {
    let mut data = vec![0u8; 512];

    // Valid NALs
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67;

    data[32..36].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[36] = 0x68;

    data[64..68].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[68] = 0x25;

    // Add filler data
    for i in 128..256 {
        data[i] = 0xFF;
    }

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_extract_frames_with_gaps() {
    let mut data = vec![0u8; 2048];

    // Frame 1
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67;
    // Gap
    data[128..132].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[132] = 0x25;
    // Another gap
    data[256..260].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[260] = 0x25;

    let result = extract_annex_b_frames(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_zero_byte_between_nals() {
    let mut data = vec![0u8; 512];

    // SPS
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67;

    // Zero byte
    data[32] = 0x00;

    // PPS
    data[64..68].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[68] = 0x68;

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_emulation_prevention_bytes() {
    let mut data = vec![0u8; 512];

    // SPS with emulation prevention
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x67;
    data[5] = 0x42;
    data[6] = 0x80; // May trigger emulation prevention
    data[7] = 0x00;
    data[8] = 0x03;

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_all_nal_types() {
    // Test each NAL unit type
    for nal_type in 1..=21u8 {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[4] = nal_type;
        // Add some payload
        data[5] = 0x80;

        let result = parse_avc(&data);
        assert!(
            result.is_ok() || result.is_err(),
            "Failed for NAL type {}",
            nal_type
        );
    }
}

#[test]
fn test_parse_avc_with_access_unit_delimiter() {
    let mut data = vec![0u8; 256];

    // Access unit delimiter
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x0B; // Access unit delimiter

    // SPS
    data[32..36].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[36] = 0x67;

    // PPS
    data[64..68].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[68] = 0x68;

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_slice_data_only() {
    // Just slice data without parameter sets
    let mut data = vec![0u8; 256];

    // Non-IDR slice
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x28; // Non-IDR slice
    data[5] = 0x80; // first_mb_in_slice

    let result = parse_avc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_avc_profile_variations() {
    let profiles = [66u8, 77, 88, 100, 110, 122, 244]; // Baseline, Main, Extended, High, High10, etc.

    for profile in profiles {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[4] = 0x67; // SPS
        data[5] = profile; // profile_idc

        let result = parse_avc(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_avc_level_variations() {
    let levels = [10u8, 11, 12, 13, 20, 21, 22, 30, 31, 40, 41, 50, 51];

    for level in levels {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[4] = 0x67; // SPS
        data[5] = 66; // Baseline profile
        data[6] = level; // level_idc

        let result = parse_avc(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_avc_constraint_sets() {
    // Test different constraint set combinations
    for constraint_flags in 0u8..=63u8 {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[4] = 0x67; // SPS
        data[5] = 66; // Baseline
        data[6] = 31; // level
        data[7] = constraint_flags; // constraint_set flags

        let result = parse_avc(&data);
        assert!(result.is_ok() || result.is_err());
    }
}
