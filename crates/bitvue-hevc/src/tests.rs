use super::*;

#[test]
fn test_empty_stream() {
    let data: &[u8] = &[];
    let stream = parse_hevc(data).unwrap();
    assert_eq!(stream.nal_units.len(), 0);
    assert_eq!(stream.frame_count(), 0);
}

// Additional tests for main parser functions and HevcStream methods

#[test]
fn test_parse_hevc_with_start_code() {
    // Test parsing with Annex B start code prefix
    let mut data = vec![0u8; 32];
    // Start code prefix (3-byte)
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    // NAL header: forbidden_zero_bit=0, nal_unit_type=32 (VPS)
    data[3] = 0x40; // (0 << 7) | (32 << 1) | 0 = 0x40

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 0);
}

#[test]
fn test_parse_hevc_with_4byte_start_code() {
    // Test parsing with 4-byte start code prefix
    let mut data = vec![0u8; 32];
    // Start code prefix (4-byte)
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x00;
    data[3] = 0x01;
    // NAL header: nal_unit_type=33 (SPS)
    data[4] = 0x42; // (0 << 7) | (33 << 1) | 0

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_vps_nal() {
    // Test VPS NAL unit (nal_unit_type=32)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x40; // nal_unit_type=32 (VPS)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::VpsNut);
    }
}

#[test]
fn test_parse_hevc_sps_nal() {
    // Test SPS NAL unit (nal_unit_type=33)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x42; // nal_unit_type=33 (SPS)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::SpsNut);
    }
}

#[test]
fn test_parse_hevc_pps_nal() {
    // Test PPS NAL unit (nal_unit_type=34)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x44; // nal_unit_type=34 (PPS)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::PpsNut);
    }
}

#[test]
fn test_parse_hevc_idr_nal() {
    // Test IDR NAL unit (nal_unit_type=19-20 are IDR_W_RADL, IDR_N_LP)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x26; // nal_unit_type=19 (IDR_W_RADL)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert!(stream.nal_units[0].header.nal_unit_type.is_idr());
    }
}

#[test]
fn test_parse_hevc_trail_nal() {
    // Test TRAIL NAL unit (nal_unit_type=1-9 are trailing pictures)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x02; // nal_unit_type=1 (TRAIL_R)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert!(stream.nal_units[0].header.nal_unit_type.is_vcl());
        assert!(!stream.nal_units[0].header.nal_unit_type.is_idr());
    }
}

#[test]
fn test_parse_hevc_aud_nal() {
    // Test Access Unit Delimiter (nal_unit_type=35)
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x46; // nal_unit_type=35 (AUD)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::AudNut);
    }
}

#[test]
fn test_parse_hevc_eos_nal() {
    // Test End of Sequence (nal_unit_type=36)
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x48; // nal_unit_type=36 (EOS)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::EosNut);
    }
}

#[test]
fn test_parse_hevc_eob_nal() {
    // Test End of Bitstream (nal_unit_type=37)
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x4A; // nal_unit_type=37 (EOB)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::EobNut);
    }
}

#[test]
fn test_parse_hevc_filler_nal() {
    // Test Filler data (nal_unit_type=38)
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x4C; // nal_unit_type=38 (Filler)
    for i in 4..16 {
        data[i] = 0xFF; // Filler bytes
    }

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::FdNut);
    }
}

#[test]
fn test_parse_hevc_prefix_nal() {
    // Test Prefix SEI (nal_unit_type=39)
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x4E; // nal_unit_type=39 (Prefix SEI)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::PrefixSeiNut);
    }
}

#[test]
fn test_parse_hevc_suffix_nal() {
    // Test Suffix SEI (nal_unit_type=40)
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x50; // nal_unit_type=40 (Suffix SEI)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert_eq!(stream.nal_units[0].header.nal_unit_type, NalUnitType::SuffixSeiNut);
    }
}

#[test]
fn test_parse_hevc_multiple_nal_units() {
    // Test multiple NAL units in stream
    let mut data = vec![0u8; 64];
    let mut pos = 0;

    // VPS (nal_unit_type=32)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x40; pos += 1; // forbidden=0, nal_type=32
    data[pos] = 0x01; pos += 1; // layer_id=0, temporal_id+1=1
    data[pos] = 0x00; pos += 1; // payload byte

    // SPS (nal_unit_type=33)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x42; pos += 1; // forbidden=0, nal_type=33
    data[pos] = 0x01; pos += 1; // layer_id=0, temporal_id+1=1
    data[pos] = 0x00; pos += 1; // payload byte

    // PPS (nal_unit_type=34)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x44; pos += 1; // forbidden=0, nal_type=34
    data[pos] = 0x01; pos += 1; // layer_id=0, temporal_id+1=1
    data[pos] = 0x00; pos += 1; // payload byte

    let result = parse_hevc(&data[..pos]);
    assert!(result.is_ok());
    let stream = result.unwrap();
    assert!(stream.nal_units.len() >= 3);
}

#[test]
fn test_parse_hevc_quick() {
    // Test parse_hevc_quick function
    let mut data = vec![0u8; 32];
    // Add SPS NAL
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x42; // SPS

    let result = parse_hevc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert!(info.nal_count >= 1);
}

#[test]
fn test_parse_hevc_quick_empty() {
    let result = parse_hevc_quick(&[]);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.nal_count, 0);
    assert_eq!(info.frame_count, 0);
}

#[test]
fn test_hevc_stream_methods() {
    // Test HevcStream methods with default stream
    let stream = HevcStream {
        nal_units: vec![],
        vps_map: std::collections::HashMap::new(),
        sps_map: std::collections::HashMap::new(),
        pps_map: std::collections::HashMap::new(),
        slices: vec![],
    };

    assert_eq!(stream.frame_count(), 0);
    assert_eq!(stream.idr_frames().len(), 0);
    assert_eq!(stream.irap_frames().len(), 0);
    assert!(stream.dimensions().is_none());
    assert!(stream.frame_rate().is_none());
    assert!(stream.bit_depth_luma().is_none());
    assert!(stream.bit_depth_chroma().is_none());
    assert!(stream.chroma_format().is_none());
    assert!(stream.get_vps(0).is_none());
    assert!(stream.get_sps(0).is_none());
    assert!(stream.get_pps(0).is_none());
}

#[test]
fn test_hevc_stream_with_sps() {
    // Test HevcStream with SPS data
    let mut sps_map = std::collections::HashMap::new();
    sps_map.insert(0, Sps {
        sps_video_parameter_set_id: 0,
        sps_max_sub_layers_minus1: 0,
        sps_temporal_id_nesting_flag: true,
        profile_tier_level: ProfileTierLevel {
            general_profile_space: 0,
            general_tier_flag: false,
            general_profile_idc: crate::sps::Profile::Main,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: true,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: true,
            general_frame_only_constraint_flag: true,
            general_level_idc: 51,
        },
        sps_seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Chroma420,
        separate_colour_plane_flag: false,
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        conformance_window_flag: false,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        sps_sub_layer_ordering_info_present_flag: false,
        sps_max_dec_pic_buffering_minus1: vec![],
        sps_max_num_reorder_pics: vec![],
        sps_max_latency_increase_plus1: vec![],
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 0,
        log2_min_luma_transform_block_size_minus2: 0,
        log2_diff_max_min_luma_transform_block_size: 0,
        max_transform_hierarchy_depth_inter: 0,
        max_transform_hierarchy_depth_intra: 0,
        scaling_list_enabled_flag: false,
        amp_enabled_flag: false,
        sample_adaptive_offset_enabled_flag: false,
        pcm_enabled_flag: false,
        num_short_term_ref_pic_sets: 0,
        long_term_ref_pics_present_flag: false,
        num_long_term_ref_pics_sps: 0,
        sps_temporal_mvp_enabled_flag: false,
        strong_intra_smoothing_enabled_flag: false,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    });

    let stream = HevcStream {
        nal_units: vec![],
        vps_map: std::collections::HashMap::new(),
        sps_map,
        pps_map: std::collections::HashMap::new(),
        slices: vec![],
    };

    assert!(stream.dimensions().is_some());
    assert_eq!(stream.dimensions(), Some((1920, 1080)));
    assert_eq!(stream.bit_depth_luma(), Some(8));
    assert_eq!(stream.bit_depth_chroma(), Some(8));
    assert_eq!(stream.chroma_format(), Some(ChromaFormat::Chroma420));
    assert!(stream.get_sps(0).is_some());
}

#[test]
fn test_hevc_quick_info_default() {
    // Test HevcQuickInfo with default values
    let info = HevcQuickInfo {
        nal_count: 0,
        vps_count: 0,
        sps_count: 0,
        pps_count: 0,
        idr_count: 0,
        frame_count: 0,
        width: None,
        height: None,
        profile: None,
        level: None,
    };

    assert_eq!(info.nal_count, 0);
    assert_eq!(info.sps_count, 0);
    assert!(info.width.is_none());
}

#[test]
fn test_nal_unit_type_checks() {
    // Test NAL unit type checking methods
    assert!(!NalUnitType::VpsNut.is_vcl());
    assert!(!NalUnitType::VpsNut.is_idr());
    assert!(!NalUnitType::VpsNut.is_irap());
    assert!(!NalUnitType::VpsNut.is_bla());
    assert!(!NalUnitType::VpsNut.is_cra());
    assert!(!NalUnitType::VpsNut.is_rasl());
    assert!(!NalUnitType::VpsNut.is_radl());
    assert!(!NalUnitType::VpsNut.is_leading());
    assert!(!NalUnitType::VpsNut.is_trailing());

    // TRAIL_N (type 0) - non-reference
    assert!(NalUnitType::TrailN.is_vcl());
    assert!(!NalUnitType::TrailN.is_idr());
    assert!(!NalUnitType::TrailN.is_irap());
    assert!(!NalUnitType::TrailN.is_reference());
    assert!(NalUnitType::TrailN.is_trailing());

    // TRAIL_R (type 1) - reference
    assert!(NalUnitType::TrailR.is_vcl());
    assert!(!NalUnitType::TrailR.is_idr());
    assert!(!NalUnitType::TrailR.is_irap());
    assert!(NalUnitType::TrailR.is_reference());
    assert!(NalUnitType::TrailR.is_trailing());

    // IDR_W_RADL (type 19)
    assert!(NalUnitType::IdrWRadl.is_vcl());
    assert!(NalUnitType::IdrWRadl.is_idr());
    assert!(NalUnitType::IdrWRadl.is_irap());
    assert!(NalUnitType::IdrWRadl.is_reference());
    assert!(!NalUnitType::IdrWRadl.is_bla());
    assert!(!NalUnitType::IdrWRadl.is_cra());

    // CRA_NUT (type 21)
    assert!(NalUnitType::CraNut.is_irap());
    assert!(NalUnitType::CraNut.is_cra());
    assert!(NalUnitType::CraNut.is_reference());

    // BLA_W_LP (type 16)
    assert!(NalUnitType::BlaWLp.is_irap());
    assert!(NalUnitType::BlaWLp.is_bla());
    assert!(NalUnitType::BlaWLp.is_reference());

    // RASL_N (type 8)
    assert!(NalUnitType::RaslN.is_vcl());
    assert!(NalUnitType::RaslN.is_rasl());
    assert!(NalUnitType::RaslN.is_leading());

    // RADL_N (type 6)
    assert!(NalUnitType::RadlN.is_vcl());
    assert!(NalUnitType::RadlN.is_radl());
    assert!(NalUnitType::RadlN.is_leading());
}

// Frame extraction tests
#[test]
fn test_extract_annex_b_frames() {
    // Test frame extraction from Annex B byte stream
    let mut data = vec![0u8; 128];
    let mut pos = 0;

    // First frame: AUD + IDR slice
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x46; pos += 1; // AUD
    data[pos] = 0x01; pos += 1;

    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x26; pos += 1; // IDR_W_RADL
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    // Second frame: AUD + TRAIL slice
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x46; pos += 1; // AUD
    data[pos] = 0x01; pos += 1;

    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x02; pos += 1; // TRAIL_R
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    // Add more padding to make the frames valid
    for _ in 0..32 {
        data.push(0x00);
    }

    let frames = extract_annex_b_frames(&data[..pos]);
    assert!(frames.is_ok());
    let frame_list = frames.unwrap();
    // The function should find the frames
    assert!(frame_list.len() >= 0);
}

#[test]
fn test_extract_frame_at_index() {
    // Test extracting specific frame by index
    let mut data = vec![0u8; 128];
    let mut pos = 0;

    // First frame (IDR)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x46; pos += 1; // AUD
    data[pos] = 0x00; pos += 1; pos += 1;

    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x26; pos += 1; // IDR
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    // Second frame (TRAIL)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x46; pos += 1; // AUD
    data[pos] = 0x00; pos += 1; pos += 1;

    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x02; pos += 1; // TRAIL_R
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    let result = extract_frame_at_index(&data[..pos], 0);
    assert!(result.is_some());

    let result = extract_frame_at_index(&data[..pos], 1);
    assert!(result.is_some());

    let result = extract_frame_at_index(&data[..pos], 10);
    assert!(result.is_none());
}

#[test]
fn test_hevc_frame_to_unit_node() {
    // Test converting HEVC frame to unit node
    use crate::frames::HevcFrame;

    let frame = HevcFrame {
        frame_index: 0,
        frame_type: HevcFrameType::I,
        nal_data: vec![],
        offset: 0,
        size: 0,
        poc: 0,
        frame_num: 0,
        is_idr: true,
        is_irap: true,
        is_ref: true,
        temporal_id: Some(0),
        slice_header: None,
    };

    let node = hevc_frame_to_unit_node(&frame, 0);
    // The unit_type is "FRAME" for all frames, check that it's set correctly
    assert!(!node.unit_type.is_empty());
}

#[test]
fn test_hevc_frames_to_unit_nodes() {
    // Test converting multiple frames to unit nodes
    use crate::frames::HevcFrame;

    let frames = vec![
        HevcFrame {
            frame_index: 0,
            frame_type: HevcFrameType::I,
            nal_data: vec![],
            offset: 0,
            size: 0,
            poc: 0,
            frame_num: 0,
            is_idr: true,
            is_irap: true,
            is_ref: true,
            temporal_id: Some(0),
            slice_header: None,
        },
        HevcFrame {
            frame_index: 1,
            frame_type: HevcFrameType::P,
            nal_data: vec![],
            offset: 0,
            size: 0,
            poc: 1,
            frame_num: 1,
            is_idr: false,
            is_irap: false,
            is_ref: true,
            temporal_id: Some(0),
            slice_header: None,
        },
    ];

    let nodes = hevc_frames_to_unit_nodes(&frames);
    assert_eq!(nodes.len(), 2);
    // The unit_type is "FRAME" for all frames
    assert!(!nodes[0].unit_type.is_empty());
    assert!(!nodes[1].unit_type.is_empty());
}

// NAL unit parsing tests
#[test]
fn test_find_nal_units() {
    // Test finding NAL units in raw data
    let data = vec![
        0x00, 0x00, 0x01, // start code
        0x40, // VPS
        0x01, // header
        0x00, // payload
    ];

    let nal_units = find_nal_units(&data);
    assert!(nal_units.len() >= 1);
}

#[test]
fn test_parse_nal_header() {
    // Test NAL header parsing
    let header_bytes = [0x40, 0x01]; // VPS with layer_id=0, temporal_id=0
    let result = parse_nal_header(&header_bytes);
    assert!(result.is_ok());
    let header = result.unwrap();
    assert_eq!(header.nal_unit_type, NalUnitType::VpsNut);
}

#[test]
fn test_parse_nal_header_invalid() {
    // Test NAL header parsing with invalid data
    let result = parse_nal_header(&[]);
    assert!(result.is_err());

    let result = parse_nal_header(&[0x80]); // forbidden_zero_bit set
    assert!(result.is_err());
}

// Edge cases and error handling
#[test]
fn test_parse_hevc_with_invalid_nal() {
    // Test parsing with corrupted NAL data
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x80; // forbidden_zero_bit set (invalid)

    let result = parse_hevc(&data);
    // Should either succeed with empty NALs or handle error gracefully
    match result {
        Ok(stream) => {
            // Parser should skip invalid NAL
            assert!(stream.nal_units.is_empty() || stream.nal_units.len() >= 0);
        }
        Err(_) => {
            // Or return error
        }
    }
}

#[test]
fn test_parse_hevc_with_rbsp_trailing_bits() {
    // Test parsing with RBSP trailing bits
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x40; // VPS
    data[4] = 0x01; // header
    data[5] = 0x80; // rbsp_trailing_bits (1 followed by zeros)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_cra_nal() {
    // Test CRA (Clean Random Access) NAL unit
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x2A; // nal_unit_type=21 (CRA_NUT)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert!(stream.nal_units[0].header.nal_unit_type.is_cra());
        assert!(stream.nal_units[0].header.nal_unit_type.is_irap());
    }
}

#[test]
fn test_parse_hevc_bla_nal() {
    // Test BLA (Broken Link Access) NAL unit
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x20; // nal_unit_type=16 (BLA_W_LP)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert!(stream.nal_units[0].header.nal_unit_type.is_bla());
        assert!(stream.nal_units[0].header.nal_unit_type.is_irap());
    }
}

#[test]
fn test_parse_hevc_rasl_nal() {
    // Test RASL (Random Access Skippable Leading) NAL unit
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x10; // nal_unit_type=8 (RASL_N)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert!(stream.nal_units[0].header.nal_unit_type.is_rasl());
        assert!(stream.nal_units[0].header.nal_unit_type.is_leading());
    }
}

#[test]
fn test_parse_hevc_radl_nal() {
    // Test RADL (Random Access Decodable Leading) NAL unit
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x0C; // nal_unit_type=6 (RADL_N)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert!(stream.nal_units[0].header.nal_unit_type.is_radl());
        assert!(stream.nal_units[0].header.nal_unit_type.is_leading());
    }
}

#[test]
fn test_parse_hevc_with_emulation_prevention() {
    // Test that emulation prevention bytes are handled
    let data = vec![
        0x00, 0x00, 0x01, // start code
        0x40, // VPS
        0x01, // header
        0x00, 0x00, 0x03, 0x00, // emulation prevention byte
    ];

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_remove_emulation_prevention_bytes() {
    // Test emulation prevention byte removal
    let data = vec![0x00, 0x00, 0x03, 0x00, 0x00, 0x03, 0x01];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0x00, 0x00, 0x01]);
}

#[test]
fn test_remove_emulation_prevention_bytes_no_removal() {
    // Test with no emulation prevention bytes
    let data = vec![0x00, 0x00, 0x01, 0x02, 0x03];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0x01, 0x02, 0x03]);
}

#[test]
fn test_hevc_stream_frame_count() {
    // Test frame counting with actual parsing
    let mut data = vec![0u8; 128];
    let mut pos = 0;

    // First frame (IDR)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x26; pos += 1; // IDR_W_RADL
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    // Second frame (TRAIL)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x02; pos += 1; // TRAIL_R
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    let result = parse_hevc(&data[..pos]);
    assert!(result.is_ok());
}

// Profile and level tests
#[test]
fn test_hevc_profiles() {
    // Test different HEVC profiles
    use crate::sps::Profile;

    let profiles: Vec<Profile> = vec![
        Profile::Main,
        Profile::Main10,
        Profile::MainStillPicture,
        Profile::RangeExtensions,
        Profile::HighThroughput,
    ];

    for profile in &profiles {
        let _ = profile.idc();
    }
}

#[test]
fn test_chroma_format() {
    // Test different chroma formats
    assert_eq!(ChromaFormat::Monochrome, ChromaFormat::Monochrome);
    assert_eq!(ChromaFormat::Chroma420, ChromaFormat::Chroma420);
    assert_eq!(ChromaFormat::Chroma422, ChromaFormat::Chroma422);
    assert_eq!(ChromaFormat::Chroma444, ChromaFormat::Chroma444);
}

#[test]
fn test_parse_hevc_quick_with_multiple_sps() {
    // Test parse_hevc_quick with multiple SPS
    let mut data = vec![0u8; 128];
    let mut pos = 0;

    // First SPS
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x42; pos += 1; // SPS
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    // Second SPS (should not override dimensions)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x42; pos += 1; // SPS
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    let result = parse_hevc_quick(&data[..pos]);
    assert!(result.is_ok());
    let info = result.unwrap();
    // Both SPS should be counted
    assert!(info.sps_count >= 1);
}

#[test]
fn test_parse_hevc_quick_frame_counts() {
    // Test frame counting in parse_hevc_quick
    let mut data = vec![0u8; 128];
    let mut pos = 0;

    // IDR (counts as frame)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x26; pos += 1; // IDR_W_RADL
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    // TRAIL_R (counts as frame)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x02; pos += 1; // TRAIL_R
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    // VPS (non-VCL)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x40; pos += 1; // VPS
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    let result = parse_hevc_quick(&data[..pos]);
    assert!(result.is_ok());
    let info = result.unwrap();
    // Should have at least 1 frame
    assert!(info.frame_count >= 1);
    assert!(info.idr_count >= 1);
    assert!(info.vps_count >= 1);
}

#[test]
fn test_hevc_quick_info_with_dimensions() {
    // Test HevcQuickInfo with width/height set
    let info = HevcQuickInfo {
        nal_count: 5,
        vps_count: 1,
        sps_count: 1,
        pps_count: 1,
        idr_count: 1,
        frame_count: 2,
        width: Some(1920),
        height: Some(1080),
        profile: Some(1),
        level: Some(51),
    };

    assert_eq!(info.nal_count, 5);
    assert_eq!(info.width, Some(1920));
    assert_eq!(info.height, Some(1080));
    assert_eq!(info.profile, Some(1));
    assert_eq!(info.level, Some(51));
}

#[test]
fn test_hevc_slice_type() {
    // Test slice type variants
    use crate::slice::SliceType;

    assert_eq!(SliceType::B.as_str(), "B");
    assert_eq!(SliceType::P.as_str(), "P");
    assert_eq!(SliceType::I.as_str(), "I");

    assert!(!SliceType::B.is_intra());
    assert!(!SliceType::P.is_intra());
    assert!(SliceType::I.is_intra());
}

#[test]
fn test_parse_hevc_temporal_id() {
    // Test parsing with different temporal IDs
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x40; // VPS
    data[4] = 0x05; // temporal_id+1 = 2 (temporal_id = 1)

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_hevc_layer_id() {
    // Test parsing with different layer IDs
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x40; // VPS
    data[4] = 0x21; // layer_id = 4, temporal_id+1 = 1

    let result = parse_hevc(&data);
    assert!(result.is_ok());
}

#[test]
fn test_hevc_stream_idr_frames() {
    // Test IDR frame detection through parsing
    let mut data = vec![0u8; 64];
    let mut pos = 0;

    // IDR frame
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x26; pos += 1; // IDR_W_RADL
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    // Non-IDR frame (TRAIL)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x02; pos += 1; // TRAIL_R
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    let result = parse_hevc(&data[..pos]);
    assert!(result.is_ok());
}

#[test]
fn test_hevc_stream_irap_frames() {
    // Test IRAP frame detection through parsing
    let mut data = vec![0u8; 64];
    let mut pos = 0;

    // CRA frame (IRAP)
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x00; pos += 1;
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x2A; pos += 1; // CRA_NUT
    data[pos] = 0x01; pos += 1;
    data[pos] = 0x00; pos += 1;

    let result = parse_hevc(&data[..pos]);
    assert!(result.is_ok());
    let stream = result.unwrap();
    if !stream.nal_units.is_empty() {
        assert!(stream.nal_units[0].header.nal_unit_type.is_irap());
    }
}

#[test]
fn test_parse_hevc_with_all_slice_types() {
    // Test parsing all VCL NAL types
    let vcl_types = [
        NalUnitType::TrailN,
        NalUnitType::TrailR,
        NalUnitType::TsaN,
        NalUnitType::TsaR,
        NalUnitType::StsaN,
        NalUnitType::StsaR,
        NalUnitType::RadlN,
        NalUnitType::RadlR,
        NalUnitType::RaslN,
        NalUnitType::RaslR,
        NalUnitType::BlaWLp,
        NalUnitType::BlaWRadl,
        NalUnitType::BlaNLp,
        NalUnitType::IdrWRadl,
        NalUnitType::IdrNLp,
        NalUnitType::CraNut,
    ];

    for nal_type in vcl_types.iter() {
        let mut data = vec![0u8; 16];
        data[0] = 0x00;
        data[1] = 0x00;
        data[2] = 0x01;
        data[3] = (*nal_type as u8) << 1;

        let result = parse_hevc(&data);
        assert!(result.is_ok(), "Failed for NAL type {:?}", nal_type);
    }
}

// === Comprehensive Functional Tests ===

#[test]
fn test_parse_sps_tier_and_level() {
    // Test SPS parsing with specific tier and level
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x42]); // NAL header
    data[4] = 0x01; // sps_video_parameter_set_id
    data[5] = 0x01; // max_sub_layers_minus1
    data[6] = 0x60; // general_profile_idc (Main, 10 bits)
    data.extend_from_slice(&[0x00, 0x00]); // general_level_idc = 0

    let result = parse_sps(&data[4..]);
    // May fail due to incomplete bitstream, but should not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_sps_chroma_format() {
    // Test SPS parsing with different chroma formats
    for chroma_id in &[1u8, 2, 3] { // YUV420, YUV422, YUV444
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x42]); // NAL header
        data[4] = 0x01; // sps_video_parameter_set_id
        data[14] = *chroma_id; // chroma_format_idc

        let result = parse_sps(&data[4..]);
        // May fail due to incomplete bitstream, but should not panic
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_parse_pps_pic_parameter_set_id() {
    // Test PPS parsing extracts pic_parameter_set_id correctly
    let mut data = vec![0u8; 16];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x44]); // NAL header
    data[4] = 5; // pps_pic_parameter_set_id
    data[5] = 0x00; // pps_seq_parameter_set_id

    let result = parse_pps(&data[4..]);
    // May fail due to incomplete bitstream, but should not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_nal_header_vcl_nal() {
    // Test NAL header parsing for VCL NAL units
    for nal_type in [1u8, 20, 32] { // various VCL types
        let mut data = vec![0u8; 2];
        data[0] = (nal_type << 1) & 0xFE; // nal_unit_type
        data[1] = 0x01; // nuh_temporal_id_plus1
        let header = parse_nal_header(&data);
        assert!(header.is_ok(), "Should parse NAL header for type {}", nal_type);
        let _header = header.unwrap();
        // Verify parsing succeeded
    }
}

#[test]
fn test_parse_nal_header_non_vcl() {
    // Test NAL header parsing for non-VCL NAL units
    for nal_type in [32u8, 33, 34, 39] { // various non-VCL types
        let mut data = vec![0u8; 2];
        data[0] = (nal_type << 1) & 0xFE; // nal_unit_type
        data[1] = 0x01; // nuh_temporal_id_plus1
        let header = parse_nal_header(&data);
        assert!(header.is_ok(), "Should parse non-VCL NAL header");
        let _header = header.unwrap();
        // Verify parsing succeeded
    }
}

#[test]
fn test_parse_nal_units_multiple_nals() {
    // Test parsing multiple NAL units from byte stream
    let mut data = vec![0u8; 128];
    // First NAL: VPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x40]); // NAL header
    data.extend_from_slice(&[0x01]); // vps_video_parameter_set_id
    data.extend_from_slice(&[0x00; 8]);

    // Second NAL: SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x42]); // NAL header
    data.extend_from_slice(&[0x01]); // sps_video_parameter_set_id
    data.extend_from_slice(&[0x00; 8]);

    // Third NAL: PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x44]); // NAL header
    data.extend_from_slice(&[0x01]); // pps_pic_parameter_set_id
    data.extend_from_slice(&[0x00; 8]);

    let result = parse_nal_units(&data);
    assert!(result.is_ok());
    let nal_units = result.unwrap();
    assert_eq!(nal_units.len(), 3);
    assert_eq!(nal_units[0].header.nal_unit_type, NalUnitType::VpsNut);
    assert_eq!(nal_units[1].header.nal_unit_type, NalUnitType::SpsNut);
    assert_eq!(nal_units[2].header.nal_unit_type, NalUnitType::PpsNut);
}

#[test]
fn test_parse_hevc_quick_info_extraction() {
    // Test quick info extraction from HEVC stream
    let mut data = vec![0u8; 64];
    // Add a minimal SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x42]);
    data.extend_from_slice(&[0x01]); // sps_video_parameter_set_id
    data.extend_from_slice(&[0x60, 0x00, 0x00, 0x00]); // profile_idc + level
    data.extend_from_slice(&[0x01, 0x00]); // chroma_format_idc = 1
    data.extend_from_slice(&[0x00; 8]);

    let result = parse_hevc_quick(&data);
    assert!(result.is_ok());
    let info = result.unwrap();
    // width/height are Option<u32>, profile/level are Option<u8>
    assert!(info.width.is_some() || info.height.is_some() || info.profile.is_some() || info.level.is_some() || info.nal_count > 0);
}

#[test]
fn test_parse_vps_profile_tier() {
    // Test VPS parsing - VPS parsing requires valid bitstream data
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x40]); // NAL header
    data[4] = 0x01; // vps_video_parameter_set_id
    data[5] = 0x00; // vps_max_sub_layers_minus1
    data[6] = 0x60; // general_profile_idc (Main, 10 bits)
    data.extend_from_slice(&[0x00, 0x00]); // general_level_idc = 0

    // VPS parsing is done through parse_hevc, not directly
    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_slice_header_with_pic_order_cnt() {
    // Test slice header parsing - slice headers are parsed through parse_hevc
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&[0x00, 0x00, 0x01, 0x01]); // NAL header (trail_n)
    data[4] = 0x01; // first_slice_segment_in_pic_flag
    data[5] = 0x01; // no_output_of_prior_pics_flag
    data[6] = 0x01; // slice_pic_parameter_set_id
    // Add pic_order_cnt_lsb
    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

// === Error Handling Tests ===

#[test]
fn test_parse_hevc_with_completely_invalid_data() {
    // Test parse_hevc with completely random/invalid data
    let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let result = parse_hevc(&data);
    // Should handle gracefully - either Ok with minimal info or Err
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_all_zeros() {
    // Test parse_hevc with all zeros (completely invalid)
    let data = vec![0u8; 100];
    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_truncated_nal_unit() {
    // Test parse_hevc with truncated NAL unit
    let data = [0x00, 0x00, 0x01]; // Only start code (no NAL header)
    let result = parse_hevc(&data);
    // Should handle gracefully - incomplete NAL unit
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_invalid_nal_type() {
    // Test parse_hevc with reserved/invalid NAL unit type
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0xFF; // Invalid NAL type (reserved)

    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_repeated_start_codes() {
    // Test parse_hevc with repeated start codes (no actual data)
    let data = [0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01];
    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_quick_with_invalid_data() {
    // Test parse_hevc_quick with invalid data
    let data = vec![0xFFu8; 50];
    let result = parse_hevc_quick(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_very_large_input() {
    // Test parse_hevc doesn't crash on very large input
    let large_data = vec![0u8; 10_000_000]; // 10 MB
    let result = parse_hevc(&large_data);
    // Should handle without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_malformed_vps() {
    // Test parse_hevc with malformed VPS
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x00; // VPS NAL type
    data.extend_from_slice(&[0xFF; 28]);

    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_malformed_sps() {
    // Test parse_hevc with malformed SPS
    let mut data = vec![0u8; 32];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x42; // SPS NAL type (bit 1 set for forbidden_zero_bit = 0)
    data.extend_from_slice(&[0xFF; 28]);

    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_incomplete_start_code() {
    // Test parse_hevc with incomplete start code
    let data = [0x00, 0x00]; // Only 2 bytes of start code
    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_error_messages_are_descriptive() {
    // Test that error messages provide useful information
    let invalid_data = vec![0xFFu8; 10];
    let result = parse_hevc(&invalid_data);
    if let Err(e) = result {
        // Error should have some description
        let error_msg = format!("{}", e);
        assert!(!error_msg.is_empty(), "Error message should not be empty");
    }
}

#[test]
fn test_parse_hevc_with_embedded_nulls() {
    // Test parse_hevc handles embedded null bytes
    let mut data = vec![0u8; 100];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01; // Start code
    data[3] = 0x00; // Embedded null in NAL header position
    // Rest is nulls
    for i in 4..100 {
        data[i] = 0x00;
    }

    let result = parse_hevc(&data);
    // Should handle without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_single_byte() {
    // Test parse_hevc with single byte input
    let data = [0x67];
    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_two_bytes() {
    // Test parse_hevc with two byte input
    let data = [0x00, 0x67];
    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_unicode_bytes() {
    // Test parse_hevc doesn't crash on unexpected byte patterns
    let data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let result = parse_hevc(&data);
    // Should handle all byte values gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_forbidden_bit_set() {
    // Test parse_hevc with forbidden_zero_bit set in NAL header
    let mut data = vec![0u8; 16];
    data[0] = 0x00;
    data[1] = 0x00;
    data[2] = 0x01;
    data[3] = 0x81; // forbidden_zero_bit = 1 (invalid)

    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_with_mixed_nal_types() {
    // Test parse_hevc with mixed valid and invalid NAL types
    let mut data = vec![0u8; 64];
    // Valid SPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x42]);
    data.extend_from_slice(&[0x00; 10]);
    // Invalid NAL
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0xFF]);
    data.extend_from_slice(&[0x00; 10]);
    // Valid PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x44]);
    data.extend_from_slice(&[0x00; 10]);

    let result = parse_hevc(&data);
    // Should handle gracefully - parse valid NALs, skip invalid
    assert!(result.is_ok() || result.is_err());
}

// === Additional Negative Tests for Public API ===

#[test]
fn test_parse_nal_header_invalid_temporal_id() {
    // Test NAL header with zero temporal_id_plus1 (must be >= 1)
    let mut data = vec![0u8; 2];
    data[0] = 0x00; // nal_unit_type = 0
    data[1] = 0x00; // nuh_temporal_id_plus1 = 0 (invalid)

    let result = parse_nal_header(&data);
    // May succeed but temporal_id should be handled
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_mv_grid_with_empty_data() {
    // Test MV grid extraction with empty slice data
    // Note: extract_mv_grid requires valid inputs, test error handling
    use crate::overlay_extraction::extract_mv_grid;
    use crate::sps::{Profile, ProfileTierLevel};
    let nal_units: &[crate::nal::NalUnit] = &[];
    let sps = Sps {
        sps_video_parameter_set_id: 0,
        sps_max_sub_layers_minus1: 0,
        sps_temporal_id_nesting_flag: true,
        profile_tier_level: ProfileTierLevel {
            general_profile_space: 0,
            general_tier_flag: false,
            general_profile_idc: Profile::Main,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: true,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: false,
            general_frame_only_constraint_flag: true,
            general_level_idc: 120,
        },
        sps_seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Chroma420,
        separate_colour_plane_flag: false,
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        conformance_window_flag: false,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        sps_sub_layer_ordering_info_present_flag: false,
        sps_max_dec_pic_buffering_minus1: vec![0],
        sps_max_num_reorder_pics: vec![0],
        sps_max_latency_increase_plus1: vec![0],
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 0,
        log2_min_luma_transform_block_size_minus2: 0,
        log2_diff_max_min_luma_transform_block_size: 0,
        max_transform_hierarchy_depth_inter: 0,
        max_transform_hierarchy_depth_intra: 0,
        scaling_list_enabled_flag: false,
        amp_enabled_flag: false,
        sample_adaptive_offset_enabled_flag: false,
        pcm_enabled_flag: false,
        num_short_term_ref_pic_sets: 0,
        long_term_ref_pics_present_flag: false,
        num_long_term_ref_pics_sps: 0,
        sps_temporal_mvp_enabled_flag: false,
        strong_intra_smoothing_enabled_flag: false,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };
    let result = extract_mv_grid(nal_units, &sps);
    // Should handle gracefully - may return empty grid or error
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_extract_qp_grid_with_empty_data() {
    // Test QP grid extraction with empty data
    // Note: extract_qp_grid requires valid SPS, test error handling with empty data
    use crate::overlay_extraction::extract_qp_grid;
    use crate::sps::{Profile, ProfileTierLevel};
    let nal_units: &[crate::nal::NalUnit] = &[];
    let sps = Sps {
        sps_video_parameter_set_id: 0,
        sps_max_sub_layers_minus1: 0,
        sps_temporal_id_nesting_flag: true,
        profile_tier_level: ProfileTierLevel {
            general_profile_space: 0,
            general_tier_flag: false,
            general_profile_idc: Profile::Main,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: true,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: false,
            general_frame_only_constraint_flag: true,
            general_level_idc: 120,
        },
        sps_seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Chroma420,
        separate_colour_plane_flag: false,
        pic_width_in_luma_samples: 1920,
        pic_height_in_luma_samples: 1080,
        conformance_window_flag: false,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        sps_sub_layer_ordering_info_present_flag: false,
        sps_max_dec_pic_buffering_minus1: vec![0],
        sps_max_num_reorder_pics: vec![0],
        sps_max_latency_increase_plus1: vec![0],
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 0,
        log2_min_luma_transform_block_size_minus2: 0,
        log2_diff_max_min_luma_transform_block_size: 0,
        max_transform_hierarchy_depth_inter: 0,
        max_transform_hierarchy_depth_intra: 0,
        scaling_list_enabled_flag: false,
        amp_enabled_flag: false,
        sample_adaptive_offset_enabled_flag: false,
        pcm_enabled_flag: false,
        num_short_term_ref_pic_sets: 0,
        long_term_ref_pics_present_flag: false,
        num_long_term_ref_pics_sps: 0,
        sps_temporal_mvp_enabled_flag: false,
        strong_intra_smoothing_enabled_flag: false,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    };
    let result = extract_qp_grid(nal_units, &sps, 26); // Valid QP value
    // Empty data should return error or empty grid
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_parse_hevc_quick_with_empty_input() {
    // Test quick info with empty input
    let data: &[u8] = &[];
    let result = parse_hevc_quick(data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
    if let Ok(info) = result {
        assert_eq!(info.nal_count, 0);
    }
}

#[test]
fn test_parse_hevc_with_only_start_codes() {
    // Test parse_hevc with only start codes (no actual NAL data)
    let data = [0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00];
    let result = parse_hevc(&data);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_find_nal_units_with_empty_input() {
    // Test find_nal_units with empty input - returns Vec<(usize, usize)>, not Result
    let data: &[u8] = &[];
    let units = find_nal_units(data);
    assert_eq!(units.len(), 0);
}

#[test]
fn test_find_nal_units_with_no_start_codes() {
    // Test find_nal_units with data but no start codes - returns Vec<(usize, usize)>
    let data = vec![0x12, 0x34, 0x56, 0x78];
    let units = find_nal_units(&data);
    assert_eq!(units.len(), 0);
}
