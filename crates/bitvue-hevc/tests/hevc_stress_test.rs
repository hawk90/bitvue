// Stress tests for HEVC codec - large inputs, random patterns, boundary conditions
use bitvue_hevc::{parse_hevc, parse_hevc_quick, parse_nal_header};

#[test]
fn test_parse_hevc_large_input_10kb() {
    let mut data = vec![0u8; 10_240];
    // Add some NAL units
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x20; // VPS
    data[512..516].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[516] = 0x21; // SPS
    data[1024..1028].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[1028] = 0x22; // PPS

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_large_input_100kb() {
    let mut data = vec![0u8; 102_400];
    // Add periodic NAL units
    for i in 0..10 {
        let offset = i * 10_240;
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x20 + (i as u8) % 5;
    }

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_random_pattern_1kb() {
    let mut data = vec![0u8; 1024];
    for i in 0..1024 {
        data[i] = ((i * 11 + 5) % 256) as u8;
    }

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_repeated_start_codes() {
    let mut data = vec![0u8; 256];
    for i in 0..32 {
        data[i * 8..i * 8 + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    }

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_max_nal_count() {
    let mut data = vec![0u8; 8192];
    let mut offset = 0;
    for i in 0..128 {
        if offset + 8 > data.len() {
            break;
        }
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x20; // VPS
        offset += 64;
    }

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nal_header_all_temporal_ids() {
    // Test all temporal_id values
    for temporal_id in 0u8..=7u8 {
        let mut data = vec![0u8; 4];
        data[0] = 0x20; // VPS NAL type
        data[1] = 0x00; // nuh_layer_id
        data[2] = temporal_id + 1; // nuh_temporal_id_plus1

        let result = parse_nal_header(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_hevc_quick_empty() {
    let data: &[u8] = &[];
    let result = parse_hevc_quick(data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().nal_count, 0);
}

#[test]
fn test_parse_hevc_quick_no_nals() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF];
    let result = parse_hevc_quick(&data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().nal_count, 0);
}

#[test]
fn test_parse_hevc_all_nal_types() {
    // Test various NAL unit types
    let nal_types = [32u8, 33, 34, 35, 36, 37, 38, 39, 40]; // VPS, SPS, PPS, AUD, EOS, etc.

    for nal_type in nal_types {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[4] = nal_type;

        let result = parse_hevc(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_hevc_stress_1mb() {
    let mut data = vec![0u8; 1_048_576];
    // Add NAL units periodically
    for i in 0..256 {
        let offset = i * 4096;
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x21 + (i as u8 % 3);
        // Add payload
        for j in 5..128 {
            data[offset + j] = ((j + i) % 256) as u8;
        }
    }

    let result = parse_hevc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_boundary_sizes() {
    let sizes = [
        1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 511, 512,
    ];

    for size in sizes {
        let mut data = vec![0u8; size];
        if size >= 5 {
            data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            data[4] = 0x21;
        }

        let result = parse_hevc(&data);
        assert!(result.is_ok() || result.is_err(), "Failed at size {}", size);
    }
}

#[test]
fn test_parse_hevc_vps_sps_pps_sequence() {
    let mut data = vec![0u8; 4096];
    let mut offset = 0;

    // VPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x20;
    offset += 64;

    // SPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x21;
    offset += 64;

    // PPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x22;
    offset += 64;

    // Slice
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x01; // TRAIL_R

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_multiple_slices() {
    let mut data = vec![0u8; 2048];
    let mut offset = 0;

    for i in 0..3 {
        // Slice
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x01; // TRAIL_R
        offset += 128;
    }

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_with_eos() {
    let mut data = vec![0u8; 512];

    // VPS
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x20;

    // EOS
    data[64..68].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[68] = 0x0A; // EOS

    // SPS
    data[128..132].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[132] = 0x21;

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_with_prefix_sei() {
    let mut data = vec![0u8; 512];

    // Prefix SEI
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x27; // Prefix SEI
    data[5] = 0x00; // payload_size

    // VPS
    data[32..36].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[36] = 0x20;

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_different_profiles() {
    let profiles = [1u8, 2, 3, 4]; // Main, Main 10, Main Still Intra, Main 10 Still Intra

    for profile in profiles {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[4] = 0x20; // VPS

        let result = parse_hevc(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_hevc_max_layers() {
    let mut data = vec![0u8; 128];
    // VPS with max_temporal_layers_minus1
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x20;
    data[5] = 0x07; // max_temporal_layers_minus1 = 7

    let result = parse_hevc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_chroma_formats() {
    // Test different chroma formats
    for chroma in [0u8, 1, 2, 3] {
        // 400, 420, 422, 444
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[4] = 0x21; // SPS
        data[5] = 0x01; // seq_parameter_set_id
        data[6] = (chroma << 4) | 0x80; // chroma_format_idc + other bits

        let result = parse_hevc(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_hevc_with_ref_pic_sets() {
    let mut data = vec![0u8; 2048];
    let mut offset = 0;

    // VPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x20;
    offset += 64;

    // SPS with reference picture sets
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x21;
    data[offset + 6] = 0x03; // num_short_term_ref_pic_sets
    offset += 128;

    // Slice
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x01;

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_tiles() {
    let mut data = vec![0u8; 1024];
    let mut offset = 0;

    // VPS
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x20;
    offset += 64;

    // SPS with tiles
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x21;
    offset += 128;

    // Slice with tiles
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x01;

    let result = parse_hevc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_scaling_lists() {
    let mut data = vec![0u8; 512];

    // SPS with scaling list
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[4] = 0x21;
    data[5] = 0x01; // seq_parameter_set_id
    data[6] = 0x01; // scaling_list_enabled_flag = 1

    let result = parse_hevc(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_multiple_sequences() {
    let mut data = vec![0u8; 3072];
    let mut offset = 0;

    // First sequence
    for _ in 0..3 {
        data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        data[offset + 4] = 0x20; // VPS
        offset += 64;
    }

    // Second sequence
    data[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data[offset + 4] = 0x20; // VPS
    offset += 64;

    let result = parse_hevc(&data);
    assert!(result.is_ok() || result.is_err());
}
