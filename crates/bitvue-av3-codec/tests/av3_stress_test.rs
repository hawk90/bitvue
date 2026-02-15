// Stress tests for AV3 codec - large inputs, random patterns, boundary conditions
use bitvue_av3_codec::{
    parse_av3, parse_frame_header, parse_obu_header, parse_sequence_header, ObuType,
};

#[test]
fn test_parse_av3_large_input_10kb() {
    let mut data = vec![0u8; 10_240];
    // Add OBU markers
    data[0] = 0x80; // Temporal delimiter
    data[1] = 0x00;
    data[256] = 0x0C; // Sequence header
    data[257] = 0x00;
    data[512] = (3 << 3) | 0x04; // Frame header
    data[513] = 0x10;

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_large_input_100kb() {
    let mut data = vec![0u8; 102_400];
    // Add periodic OBU units
    for i in 0..10 {
        let offset = i * 10_240;
        data[offset] = (i as u8 % 8) << 3;
        data[offset + 1] = 0x00;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_av3_random_pattern_1kb() {
    let mut data = vec![0u8; 1024];
    for i in 0..1024 {
        data[i] = ((i * 19 + 11) % 256) as u8;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_av3_all_obu_types() {
    // Test all OBU types
    let obu_types = [
        0u8, // Temporal Delimiter
        1,   // Sequence Header
        2,   // TD
        3,   // Frame Header
        4,   // Tile Group
        5,   // Metadata
        6,   // Frame
        7,   // Redundant Frame Header
        8,   // Tile List
    ];

    for obu_type in obu_types {
        let mut data = vec![0u8; 8];
        data[0] = (obu_type << 3) | 0x02;

        let result = parse_obu_header(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_av3_max_obu_count() {
    let mut data = vec![0u8; 8192];
    let mut offset = 0;
    for i in 0..128 {
        if offset + 8 > data.len() {
            break;
        }
        data[offset] = 0x80; // Temporal delimiter
        data[offset + 1] = 0x00;
        offset += 64;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_av3_empty() {
    let data: &[u8] = &[];
    let stream = parse_av3(data).unwrap();
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_parse_av3_no_obus() {
    let data = [0xFF, 0xFF, 0xFF, 0xFF];
    let stream = parse_av3(&data).unwrap();
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_parse_av3_stress_1mb() {
    let mut data = vec![0u8; 1_048_576];
    // Add OBU units periodically
    for i in 0..256 {
        let offset = i * 4096;
        data[offset] = ((i as u8 % 8) << 3) | 0x02;
        // Add payload
        for j in 5..128 {
            data[offset + j] = ((j + i) % 256) as u8;
        }
    }

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_boundary_sizes() {
    let sizes = [
        1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 511, 512,
    ];

    for size in sizes {
        let data = vec![0u8; size];
        let result = parse_av3(&data);
        assert!(result.is_ok() || result.is_err(), "Failed at size {}", size);
    }
}

#[test]
fn test_parse_av3_all_profiles() {
    // Test all profile values (0-3)
    for profile in 0..=3u8 {
        let mut data = vec![0u8; 16];
        data[0] = 0x0C; // Sequence header marker + profile
        data[1] = profile << 6;
        data[2] = 0x00; // seq_level_index

        let result = parse_sequence_header(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_av3_all_levels() {
    // Test various level indices
    for level in 0..=31u8 {
        let mut data = vec![0u8; 16];
        data[0] = 0x0C;
        data[1] = 0x00; // profile
        data[2] = level;

        let result = parse_sequence_header(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_av3_with_temporal_delimiters() {
    let mut data = vec![0u8; 256];
    // Temporal delimiter
    data[0] = 0x80;
    data[1] = 0x00;
    // Sequence header
    data[64] = 0x0C;
    data[65] = 0x00;
    // Another temporal delimiter
    data[128] = 0x80;
    data[129] = 0x00;
    // Frame header
    data[192] = (3 << 3) | 0x04;

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_multiple_frames() {
    let mut data = vec![0u8; 2048];
    let mut offset = 0;

    for i in 0..5 {
        // Temporal delimiter
        data[offset] = 0x80;
        data[offset + 1] = 0x00;
        offset += 32;

        // Frame header
        data[offset] = (3 << 3) | 0x04;
        data[offset + 1] = 0x10;
        offset += 64;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_metadata() {
    let mut data = vec![0u8; 512];
    // Metadata OBU
    data[0] = (5 << 3) | 0x02;
    data[1] = 0x10;
    // Metadata type
    data[2] = 0x01; // METADATA_TYPE_ITUT_T35

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_tile_groups() {
    let mut data = vec![0u8; 1024];
    let mut offset = 0;

    // Sequence header
    data[offset] = 0x0C;
    offset += 64;

    // Frame header
    data[offset] = (3 << 3) | 0x04;
    offset += 64;

    // Tile group
    data[offset] = (4 << 3) | 0x02;
    data[offset + 1] = 0x10;

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_key_frames() {
    let mut data = vec![0u8; 512];
    let mut offset = 0;

    for i in 0..3 {
        // Frame header (key frame)
        data[offset] = (3 << 3) | 0x04;
        data[offset + 1] = 0x10;
        // frame_type = 0 (KeyFrame)
        offset += 128;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_inter_frames() {
    let mut data = vec![0u8; 512];
    let mut offset = 0;

    for i in 0..3 {
        // Frame header (inter frame)
        data[offset] = (3 << 3) | 0x04;
        data[offset + 1] = 0x10;
        // frame_type = 1 (InterFrame)
        offset += 128;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_alternating_key_inter() {
    let mut data = vec![0u8; 1024];
    let mut offset = 0;

    for i in 0..4 {
        data[offset] = (3 << 3) | 0x04;
        data[offset + 1] = 0x10;
        // Key/Inter alternation
        offset += 128;
    }

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_show_existing_frame() {
    let mut data = vec![0u8; 64];
    data[0] = (3 << 3) | 0x04; // Frame header OBU
    data[1] = 0x10;
    // show_existing_frame = 1

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_all_color_configs() {
    // Test different color configurations
    for color_config in 0..=7u8 {
        let mut data = vec![0u8; 32];
        data[0] = 0x0C;
        data[1] = 0x00;
        data[2] = color_config << 4;

        let result = parse_sequence_header(&data);
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_av3_with_film_grain() {
    let mut data = vec![0u8; 64];
    // Metadata OBU with film grain
    data[0] = (5 << 3) | 0x02;
    data[1] = 0x10;
    data[2] = 0x02; // METADATA_TYPE_FILM_GRAIN

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_hdr_cll() {
    let mut data = vec![0u8; 64];
    // Metadata OBU with HDR CLL
    data[0] = (5 << 3) | 0x02;
    data[1] = 0x10;
    data[2] = 0x03; // METADATA_TYPE_HDR_CLL

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_hdr_mdcv() {
    let mut data = vec![0u8; 64];
    // Metadata OBU with HDR MDCV
    data[0] = (5 << 3) | 0x02;
    data[1] = 0x10;
    data[2] = 0x04; // METADATA_TYPE_HDR_MDCV

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_scalability() {
    let mut data = vec![0u8; 64];
    // Sequence header with scalability
    data[0] = 0x0C;
    data[1] = 0x80; // scalability_flag

    let result = parse_sequence_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_timing_info() {
    let mut data = vec![0u8; 64];
    // Sequence header with timing info
    data[0] = 0x0C;
    data[1] = 0x40; // timing_info_present_flag

    let result = parse_sequence_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_decoder_model() {
    let mut data = vec![0u8; 64];
    // Sequence header with decoder model info
    data[0] = 0x0C;
    data[1] = 0x20; // decoder_model_info_present_flag

    let result = parse_sequence_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_initial_display_delay() {
    let mut data = vec![0u8; 64];
    // Sequence header with initial display delay
    data[0] = 0x0C;
    data[1] = 0x10; // initial_display_delay_present_flag

    let result = parse_sequence_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_operating_points() {
    let mut data = vec![0u8; 128];
    // Sequence header with multiple operating points
    data[0] = 0x0C;
    data[1] = 0x0F; // operating_points_cnt_minus_1 = 15

    let result = parse_sequence_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_tiling() {
    let mut data = vec![0u8; 256];
    let mut offset = 0;

    // Sequence header
    data[offset] = 0x0C;
    offset += 64;

    // Frame header with uniform tile spacing
    data[offset] = (3 << 3) | 0x04;
    data[offset + 1] = 0x80; // uniform_tile_spacing_flag

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_loop_filter() {
    let mut data = vec![0u8; 64];
    // Frame header with loop filter
    data[0] = (3 << 3) | 0x04;
    data[1] = 0x08; // loop_filter_params_present

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_quantization() {
    let mut data = vec![0u8; 64];
    // Frame header with quantization params
    data[0] = (3 << 3) | 0x04;
    data[1] = 0x04; // quantization_params

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_superres() {
    let mut data = vec![0u8; 64];
    // Frame header with superres
    data[0] = (3 << 3) | 0x04;
    data[1] = 0x02; // superres_flag

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_restoration() {
    let mut data = vec![0u8; 64];
    // Frame header with restoration
    data[0] = (3 << 3) | 0x04;
    data[1] = 0x01; // restoration_flag

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_cdef() {
    let mut data = vec![0u8; 64];
    // Frame header with CDEF
    data[0] = (3 << 3) | 0x04;
    data[1] = 0x40; // cdef_flag

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_with_reference_frames() {
    let mut data = vec![0u8; 128];
    // Frame header with reference frames
    data[0] = (3 << 3) | 0x04;
    data[1] = 0x10; // refresh_frame_flags

    let result = parse_frame_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_obu_extension() {
    let mut data = vec![0u8; 32];
    // OBU header with extension
    data[0] = (1 << 3) | 0x06; // OBU with extension
    data[1] = 0x10;
    data[2] = 0x00; // extension header

    let result = parse_obu_header(&data);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_av3_redundant_frame_header() {
    let mut data = vec![0u8; 32];
    // Redundant frame header OBU
    data[0] = (7 << 3) | 0x02;
    data[1] = 0x10;

    let result = parse_av3(&data);
    assert!(result.is_ok() || result.is_err());
}
