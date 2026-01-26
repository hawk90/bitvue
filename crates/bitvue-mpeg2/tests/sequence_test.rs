//! MPEG-2 Sequence Tests
//!
//! Tests for MPEG-2 sequence header parsing to improve coverage.

use bitvue_mpeg2::sequence;

#[test]
fn test_sequence_header_creation() {
    let seq = sequence::SequenceHeader {
        horizontal_size_value: 720,
        vertical_size_value: 480,
        aspect_ratio_information: 1,
        frame_rate_code: 4,
        bit_rate_value: 4000000,
        vbv_buffer_size_value: 112,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
    };

    assert_eq!(seq.horizontal_size_value, 720);
    assert_eq!(seq.vertical_size_value, 480);
}

#[test]
fn test_sequence_header_default() {
    let seq = sequence::SequenceHeader {
        horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
    };

    assert!(!seq.constrained_parameters_flag);
    assert!(!seq.load_intra_quantiser_matrix);
    assert!(!seq.load_non_intra_quantiser_matrix);
}

#[test]
fn test_frame_rate_method() {
    let seq = sequence::SequenceHeader {
        frame_rate_code: 4, // 29.97 fps
        horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
    };

    assert!((seq.frame_rate() - 29.97).abs() < 0.01);
    assert_eq!(seq.frame_rate_string(), "29.97 (30000/1001)");
}

#[test]
fn test_aspect_ratio_string() {
    let seq = sequence::SequenceHeader {
        aspect_ratio_information: 3, // 16:9
        horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
    };

    assert_eq!(seq.aspect_ratio_string(), "16:9");
}

#[test]
fn test_various_resolutions() {
    let resolutions = vec![
        (352, 240),   // CIF
        (352, 288),   // CIF
        (640, 480),   // VGA
        (704, 480),   // D1
        (720, 480),   // NTSC D1
        (720, 576),   // PAL D1
        (1280, 720),  // HD 720p
        (1920, 1080), // HD 1080p
    ];

    for (width, height) in resolutions {
        let seq = sequence::SequenceHeader {
            horizontal_size_value: width,
            vertical_size_value: height,
            horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
        };
        assert_eq!(seq.horizontal_size_value, width);
        assert_eq!(seq.vertical_size_value, height);
    }
}

#[test]
fn test_constrained_parameters_flag() {
    let constrained = sequence::SequenceHeader {
        constrained_parameters_flag: true,
        horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
    };
    assert!(constrained.constrained_parameters_flag);

    let unconstrained = sequence::SequenceHeader {
        constrained_parameters_flag: false,
        horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
    };
    assert!(!unconstrained.constrained_parameters_flag);
}

#[test]
fn test_quantiser_matrix_flags() {
    let intra_matrix = vec![1u8; 64];
    let non_intra_matrix = vec![2u8; 64];

    let with_matrices = sequence::SequenceHeader {
        load_intra_quantiser_matrix: true,
        load_non_intra_quantiser_matrix: true,
        intra_quantiser_matrix: Some(intra_matrix),
        non_intra_quantiser_matrix: Some(non_intra_matrix),
        horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
    };

    assert!(with_matrices.load_intra_quantiser_matrix);
    assert!(with_matrices.load_non_intra_quantiser_matrix);
    assert!(with_matrices.intra_quantiser_matrix.is_some());
    assert!(with_matrices.non_intra_quantiser_matrix.is_some());
}

#[test]
fn test_all_field_codes() {
    // Test all frame rate codes
    for code in 1..=8u8 {
        let seq = sequence::SequenceHeader {
            frame_rate_code: code,
            horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
        };
        assert_eq!(seq.frame_rate_code, code);
        assert!(seq.frame_rate() > 0.0);
    }
}

#[test]
fn test_chroma_format_from_u8() {
    assert_eq!(sequence::ChromaFormat::from_u8(1), sequence::ChromaFormat::Yuv420);
    assert_eq!(sequence::ChromaFormat::from_u8(2), sequence::ChromaFormat::Yuv422);
    assert_eq!(sequence::ChromaFormat::from_u8(3), sequence::ChromaFormat::Yuv444);
    assert_eq!(sequence::ChromaFormat::from_u8(0), sequence::ChromaFormat::Reserved);
    assert_eq!(sequence::ChromaFormat::from_u8(99), sequence::ChromaFormat::Yuv420); // Invalid defaults to 420
}

#[test]
fn test_bit_rate_values() {
    let bit_rates = vec![1000000, 4000000, 8000000, 15000000, 20000000];

    for bit_rate in bit_rates {
        let seq = sequence::SequenceHeader {
            bit_rate_value: bit_rate,
            horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
        };
        assert_eq!(seq.bit_rate_value, bit_rate);
    }
}

#[test]
fn test_vbv_buffer_values() {
    let buffer_sizes = vec![0, 112, 224, 448, 896];

    for size in buffer_sizes {
        let seq = sequence::SequenceHeader {
            vbv_buffer_size_value: size,
            horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
        };
        assert_eq!(seq.vbv_buffer_size_value, size);
    }
}

#[test]
fn test_aspect_ratio_codes() {
    let ratios = vec![
        (1, "1:1 (Square)"),
        (2, "4:3"),
        (3, "16:9"),
        (4, "2.21:1"),
    ];

    for (code, expected) in ratios {
        let seq = sequence::SequenceHeader {
            aspect_ratio_information: code,
            horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
        };
        assert_eq!(seq.aspect_ratio_string(), expected);
    }
}

#[test]
fn test_matrix_default_none() {
    let seq = sequence::SequenceHeader {
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        horizontal_size_value: 0,
        vertical_size_value: 0,
        aspect_ratio_information: 1,
        frame_rate_code: 1,
        bit_rate_value: 0,
        vbv_buffer_size_value: 0,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: false,
        load_non_intra_quantiser_matrix: false,
        intra_quantiser_matrix: None,
        non_intra_quantiser_matrix: None,
    };

    assert!(!seq.load_intra_quantiser_matrix);
    assert!(!seq.load_non_intra_quantiser_matrix);
    assert!(seq.intra_quantiser_matrix.is_none());
    assert!(seq.non_intra_quantiser_matrix.is_none());
}

#[test]
fn test_complete_sequence_header() {
    let intra = vec![10u8; 64];
    let non_intra = vec![16u8; 64];

    let seq = sequence::SequenceHeader {
        horizontal_size_value: 1920,
        vertical_size_value: 1080,
        aspect_ratio_information: 3,
        frame_rate_code: 5,
        bit_rate_value: 20000000,
        vbv_buffer_size_value: 448,
        constrained_parameters_flag: false,
        load_intra_quantiser_matrix: true,
        load_non_intra_quantiser_matrix: true,
        intra_quantiser_matrix: Some(intra),
        non_intra_quantiser_matrix: Some(non_intra),
    };

    assert_eq!(seq.horizontal_size_value, 1920);
    assert_eq!(seq.vertical_size_value, 1080);
    assert_eq!(seq.aspect_ratio_information, 3);
    assert_eq!(seq.frame_rate_code, 5);
    assert_eq!(seq.bit_rate_value, 20000000);
    assert_eq!(seq.vbv_buffer_size_value, 448);
    assert!(seq.load_intra_quantiser_matrix);
    assert!(seq.load_non_intra_quantiser_matrix);
    assert!(seq.intra_quantiser_matrix.is_some());
    assert!(seq.non_intra_quantiser_matrix.is_some());
}
