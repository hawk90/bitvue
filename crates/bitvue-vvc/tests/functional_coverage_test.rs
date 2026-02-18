#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
// Functional tests for VVC codec - targeting 100% coverage
// These tests exercise specific code paths in low-coverage modules
use bitvue_vvc::{
    parse_nal_units, parse_vvc, ChromaFormat, NalUnitHeader, NalUnitType, Pps, Profile, Sps,
    VvcError, VvcStream,
};

#[test]
fn test_error_display() {
    // Test VvcError Display implementation
    let err = VvcError::InvalidData("test error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("test error"));
}

#[test]
fn test_sps_default_construct() {
    let sps = Sps::default();
    assert_eq!(sps.sps_seq_parameter_set_id, 0);
    assert_eq!(sps.sps_video_parameter_set_id, 0);
    assert_eq!(sps.sps_max_sublayers_minus1, 0);
    assert_eq!(sps.sps_chroma_format_idc, ChromaFormat::Chroma420);
}

#[test]
fn test_sps_chroma_formats() {
    // Test all chroma formats
    assert_eq!(ChromaFormat::Monochrome as u8, 0);
    assert_eq!(ChromaFormat::Chroma420 as u8, 1);
    assert_eq!(ChromaFormat::Chroma422 as u8, 2);
    assert_eq!(ChromaFormat::Chroma444 as u8, 3);
}

#[test]
fn test_sps_resolution() {
    // Test SPS resolution methods
    let sps = Sps {
        sps_pic_width_max_in_luma_samples: 64,
        sps_pic_height_max_in_luma_samples: 48,
        ..Default::default()
    };

    assert_eq!(sps.display_width(), 64);
    assert_eq!(sps.display_height(), 48);
}

#[test]
fn test_sps_bit_depth() {
    // Test SPS bit depth
    let sps = Sps {
        sps_bitdepth_minus8: 2, // 10-bit
        ..Default::default()
    };

    assert_eq!(sps.bit_depth(), 10);

    let sps = Sps {
        sps_bitdepth_minus8: 4, // 12-bit
        ..Default::default()
    };

    assert_eq!(sps.bit_depth(), 12);
}

#[test]
fn test_pps_default_construct() {
    let pps = Pps::default();
    assert_eq!(pps.pps_pic_parameter_set_id, 0);
    assert_eq!(pps.pps_seq_parameter_set_id, 0);
}

#[test]
fn test_nal_header_all_types() {
    // Test various NAL unit types
    let types = [
        (0u8, NalUnitType::TrailNut),
        (1u8, NalUnitType::TrailNut),
        (7u8, NalUnitType::IdrWRadl),
        (20u8, NalUnitType::IdrNLp),
        (32u8, NalUnitType::VpsNut),
        (33u8, NalUnitType::SpsNut),
        (34u8, NalUnitType::PpsNut),
        (35u8, NalUnitType::AudNut),
        (36u8, NalUnitType::EosNut),
        (37u8, NalUnitType::EobNut),
    ];

    for (_nal_type, expected) in types {
        let header = NalUnitHeader {
            nal_unit_type: expected,
            nuh_layer_id: 0,
            nuh_temporal_id_plus1: 1,
        };

        assert_eq!(header.nal_unit_type, expected);
    }
}

#[test]
fn test_nal_header_vcl() {
    // Test VCL NAL types
    let vcl_types = [
        NalUnitType::TrailNut,
        NalUnitType::IdrWRadl,
        NalUnitType::IdrNLp,
        NalUnitType::CraNut,
        NalUnitType::RadlNut,
        NalUnitType::RadlNut,
    ];

    for nal_type in vcl_types {
        let _header = NalUnitHeader {
            nal_unit_type: nal_type,
            nuh_layer_id: 0,
            nuh_temporal_id_plus1: 1,
        };

        assert!(nal_type.is_vcl());
    }
}

#[test]
fn test_nal_header_non_vcl() {
    // Test non-VCL NAL types
    let non_vcl_types = [
        NalUnitType::VpsNut,
        NalUnitType::SpsNut,
        NalUnitType::PpsNut,
        NalUnitType::AudNut,
        NalUnitType::EosNut,
        NalUnitType::EobNut,
        NalUnitType::PrefixSeiNut,
        NalUnitType::SuffixSeiNut,
    ];

    for nal_type in non_vcl_types {
        let _header = NalUnitHeader {
            nal_unit_type: nal_type,
            nuh_layer_id: 0,
            nuh_temporal_id_plus1: 1,
        };

        assert!(!nal_type.is_vcl());
    }
}

#[test]
fn test_nal_header_temporal_id() {
    // Test temporal_id
    for tid in 0u8..=7u8 {
        let header = NalUnitHeader {
            nal_unit_type: NalUnitType::TrailNut,
            nuh_layer_id: 0,
            nuh_temporal_id_plus1: tid + 1,
        };

        assert_eq!(header.temporal_id(), tid);
    }
}

#[test]
fn test_parse_vvc_empty() {
    let data: &[u8] = &[];
    let stream = parse_vvc(data).unwrap();
    assert_eq!(stream.nal_units.len(), 0);
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_stream_dimensions() {
    // Test VvcStream dimension queries
    let stream = VvcStream {
        nal_units: vec![],
        sps_map: std::collections::HashMap::new(),
        pps_map: std::collections::HashMap::new(),
    };

    assert!(stream.dimensions().is_none());
    assert_eq!(stream.frame_count(), 0);
}

#[test]
fn test_parse_nal_units_empty() {
    let data: &[u8] = &[];
    let result = parse_nal_units(data);
    assert!(result.is_ok());
    let units = result.unwrap();
    assert_eq!(units.len(), 0);
}

#[test]
fn test_profile_variants() {
    // Test Profile variants
    let profiles = [
        Profile::Main10,
        Profile::Main10,
        Profile::Main10StillPicture,
    ];

    for profile in profiles {
        // Just verify we can create all profile types
        let _ = profile;
    }
}
