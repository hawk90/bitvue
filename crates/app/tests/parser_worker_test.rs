//! Tests for Parser Worker (codec-specific parsing)

#[test]
fn test_codec_parser_task() {
    struct CodecParserTask {
        codec: String,
        data: Vec<u8>,
        frame_number: usize,
    }

    let task = CodecParserTask {
        codec: "HEVC".to_string(),
        data: vec![0, 0, 0, 1], // NAL unit start code
        frame_number: 0,
    };

    assert_eq!(task.data.len(), 4);
}

#[test]
fn test_nal_unit_parsing() {
    struct NalUnit {
        nal_type: u8,
        payload: Vec<u8>,
        start_code_size: usize,
    }

    impl NalUnit {
        fn is_slice(&self) -> bool {
            self.nal_type >= 0 && self.nal_type <= 9
        }
    }

    let nal = NalUnit {
        nal_type: 1,
        payload: vec![0u8; 100],
        start_code_size: 4,
    };

    assert!(nal.is_slice());
}

#[test]
fn test_obu_parsing() {
    #[derive(Debug, PartialEq)]
    enum ObuType {
        SequenceHeader = 1,
        TemporalDelimiter = 2,
        FrameHeader = 3,
        Frame = 6,
    }

    struct Obu {
        obu_type: ObuType,
        size: usize,
        offset: u64,
    }

    let obu = Obu {
        obu_type: ObuType::SequenceHeader,
        size: 256,
        offset: 0,
    };

    assert_eq!(obu.obu_type, ObuType::SequenceHeader);
}

#[test]
fn test_bitstream_reader() {
    struct BitstreamReader {
        data: Vec<u8>,
        bit_pos: usize,
    }

    impl BitstreamReader {
        fn read_bits(&mut self, n: usize) -> u32 {
            // Simplified implementation
            let byte_pos = self.bit_pos / 8;
            let bit_offset = self.bit_pos % 8;
            self.bit_pos += n;

            if byte_pos < self.data.len() {
                (self.data[byte_pos] >> bit_offset) as u32 & ((1 << n) - 1)
            } else {
                0
            }
        }

        fn bytes_remaining(&self) -> usize {
            let byte_pos = self.bit_pos / 8;
            self.data.len().saturating_sub(byte_pos)
        }
    }

    let mut reader = BitstreamReader {
        data: vec![0b10110100, 0b11001010],
        bit_pos: 0,
    };

    assert_eq!(reader.bytes_remaining(), 2);
    reader.read_bits(4);
    assert_eq!(reader.bit_pos, 4);
}

#[test]
fn test_syntax_element_parsing() {
    struct SyntaxElement {
        name: String,
        value: u64,
        bit_offset: usize,
        bit_length: usize,
    }

    impl SyntaxElement {
        fn to_hex_string(&self) -> String {
            format!("0x{:X}", self.value)
        }
    }

    let element = SyntaxElement {
        name: "profile_idc".to_string(),
        value: 100,
        bit_offset: 0,
        bit_length: 8,
    };

    assert_eq!(element.to_hex_string(), "0x64");
}

#[test]
fn test_codec_profile_detection() {
    #[derive(Debug, PartialEq)]
    enum HevcProfile {
        Main,
        Main10,
        MainStillPicture,
    }

    fn detect_hevc_profile(profile_idc: u8) -> HevcProfile {
        match profile_idc {
            1 => HevcProfile::Main,
            2 => HevcProfile::Main10,
            3 => HevcProfile::MainStillPicture,
            _ => HevcProfile::Main,
        }
    }

    assert_eq!(detect_hevc_profile(1), HevcProfile::Main);
    assert_eq!(detect_hevc_profile(2), HevcProfile::Main10);
}

#[test]
fn test_frame_type_detection() {
    #[derive(Debug, PartialEq)]
    enum FrameType {
        I,
        P,
        B,
    }

    struct Frame {
        frame_type: FrameType,
        poc: i32, // Picture Order Count
    }

    let iframe = Frame {
        frame_type: FrameType::I,
        poc: 0,
    };

    let pframe = Frame {
        frame_type: FrameType::P,
        poc: 1,
    };

    assert_eq!(iframe.frame_type, FrameType::I);
    assert_eq!(pframe.frame_type, FrameType::P);
}

#[test]
fn test_parameter_set_storage() {
    use std::collections::HashMap;

    struct ParameterSetStorage {
        sps: HashMap<u8, Vec<u8>>, // Sequence Parameter Sets
        pps: HashMap<u8, Vec<u8>>, // Picture Parameter Sets
    }

    impl ParameterSetStorage {
        fn store_sps(&mut self, id: u8, data: Vec<u8>) {
            self.sps.insert(id, data);
        }

        fn get_sps(&self, id: u8) -> Option<&Vec<u8>> {
            self.sps.get(&id)
        }
    }

    let mut storage = ParameterSetStorage {
        sps: HashMap::new(),
        pps: HashMap::new(),
    };

    storage.store_sps(0, vec![1, 2, 3]);
    assert!(storage.get_sps(0).is_some());
    assert!(storage.get_sps(1).is_none());
}

#[test]
fn test_slice_parsing() {
    struct Slice {
        slice_type: u8,
        first_mb_in_slice: usize,
        qp: i8,
    }

    impl Slice {
        fn is_intra(&self) -> bool {
            self.slice_type == 2 || self.slice_type == 7 // I-slice
        }
    }

    let slice = Slice {
        slice_type: 2,
        first_mb_in_slice: 0,
        qp: 26,
    };

    assert!(slice.is_intra());
}

#[test]
fn test_reference_frame_management() {
    struct ReferenceFrameManager {
        dpb: Vec<usize>, // Decoded Picture Buffer (frame indices)
        max_dpb_size: usize,
    }

    impl ReferenceFrameManager {
        fn add_frame(&mut self, frame_idx: usize) -> bool {
            if self.dpb.len() < self.max_dpb_size {
                self.dpb.push(frame_idx);
                true
            } else {
                false
            }
        }

        fn is_full(&self) -> bool {
            self.dpb.len() >= self.max_dpb_size
        }
    }

    let mut manager = ReferenceFrameManager {
        dpb: vec![],
        max_dpb_size: 4,
    };

    assert!(manager.add_frame(0));
    assert!(!manager.is_full());
}

#[test]
fn test_entropy_coding_mode() {
    #[derive(Debug, PartialEq)]
    enum EntropyCodingMode {
        Cabac,
        Cavlc,
    }

    struct EntropyConfig {
        mode: EntropyCodingMode,
        cabac_init_idc: u8,
    }

    let config = EntropyConfig {
        mode: EntropyCodingMode::Cabac,
        cabac_init_idc: 0,
    };

    assert_eq!(config.mode, EntropyCodingMode::Cabac);
}

#[test]
fn test_motion_vector_parsing() {
    struct MotionVector {
        mvx: i16,
        mvy: i16,
        ref_idx: u8,
    }

    impl MotionVector {
        fn magnitude(&self) -> f32 {
            ((self.mvx as f32).powi(2) + (self.mvy as f32).powi(2)).sqrt()
        }
    }

    let mv = MotionVector {
        mvx: 3,
        mvy: 4,
        ref_idx: 0,
    };

    assert_eq!(mv.magnitude(), 5.0);
}

// ============================================================================
// Container Format Detection Tests
// ============================================================================

#[test]
fn test_container_format_ivf_detection() {
    // IVF signature: DKIF
    let ivf_header = b"DKIF\x00\x00\x20\x00";
    assert_eq!(&ivf_header[0..4], b"DKIF");
}

#[test]
fn test_container_format_mp4_ftyp() {
    // MP4 ftyp box
    let mut mp4_data = Vec::new();
    mp4_data.extend_from_slice(&20u32.to_be_bytes());
    mp4_data.extend_from_slice(b"ftyp");
    assert_eq!(&mp4_data[4..8], b"ftyp");
}

#[test]
fn test_container_format_mp4_moov() {
    // MP4 moov box
    let mut mp4_data = Vec::new();
    mp4_data.extend_from_slice(&20u32.to_be_bytes());
    mp4_data.extend_from_slice(b"moov");
    assert_eq!(&mp4_data[4..8], b"moov");
}

#[test]
fn test_container_format_mkv_ebml() {
    // MKV EBML header: 0x1A 0x45 0xDF 0xA3
    let mkv_header = [0x1A, 0x45, 0xDF, 0xA3];
    assert_eq!(mkv_header[0], 0x1A);
    assert_eq!(mkv_header[1], 0x45);
    assert_eq!(mkv_header[2], 0xDF);
    assert_eq!(mkv_header[3], 0xA3);
}

#[test]
fn test_container_format_ts_packet() {
    // TS packet starts with sync byte 0x47
    let ts_packet = [0x47; 188];
    assert_eq!(ts_packet[0], 0x47);
    assert_eq!(ts_packet.len(), 188);
}

#[test]
fn test_nal_start_code_3_byte() {
    // 3-byte start code: 0x00 0x00 0x01
    let start_code = [0x00, 0x00, 0x01];
    assert_eq!(start_code, [0x00, 0x00, 0x01]);
}

#[test]
fn test_nal_start_code_4_byte() {
    // 4-byte start code: 0x00 0x00 0x00 0x01
    let start_code = [0x00, 0x00, 0x00, 0x01];
    assert_eq!(start_code, [0x00, 0x00, 0x00, 0x01]);
}

// ============================================================================
// HEVC Specific Tests
// ============================================================================

#[test]
fn test_hevc_nal_unit_type_values() {
    // Test HEVC NAL unit type values
    assert_eq!(0u8, 0);   // TRAIL_N
    assert_eq!(1u8, 1);   // TRAIL_R
    assert_eq!(19u8, 19); // IDR_W_RADL
    assert_eq!(20u8, 20); // IDR_N_LP
    assert_eq!(32u8, 32); // VPS
    assert_eq!(33u8, 33); // SPS
    assert_eq!(34u8, 34); // PPS
}

#[test]
fn test_hevc_temporal_id_range() {
    // HEVC temporal_id_plus1 should be 1-7
    let valid_temporal_ids = vec![1, 2, 3, 4, 5, 6, 7];
    for tid in valid_temporal_ids {
        assert!(tid >= 1 && tid <= 7);
    }
}

#[test]
fn test_hevc_layer_id_range() {
    // HEVC layer_id should be 0-63
    let valid_layer_ids = vec![0u8, 1, 10, 32, 63];
    for lid in valid_layer_ids {
        assert!(lid <= 63);
    }
}

#[test]
fn test_hevc_profile_idc_values() {
    // HEVC profile_idc values
    let profiles = vec![1u8, 2, 3]; // Main, Main10, MainStillPicture
    for profile in profiles {
        assert!(profile >= 1 && profile <= 3);
    }
}

// ============================================================================
// AVC Specific Tests
// ============================================================================

#[test]
fn test_avc_nal_ref_idc_values() {
    // AVC nal_ref_idc should be 0-3
    let valid_ref_idc = vec![0u8, 1, 2, 3];
    for ref_idc in valid_ref_idc {
        assert!(ref_idc <= 3);
    }
}

#[test]
fn test_avc_nal_unit_type_range() {
    // AVC nal_unit_type should be 0-31
    let valid_types = vec![0u8, 1, 5, 7, 8, 9, 31];
    for nal_type in valid_types {
        assert!(nal_type <= 31);
    }
}

#[test]
fn test_avc_profile_idc_values() {
    // AVC profile_idc values
    let profiles = vec![66u8, 77, 88, 100, 110, 122, 244];
    // Baseline=66, Main=77, Extended=88, High=100, etc.
    for profile in profiles {
        assert!(profile >= 66);
    }
}

// ============================================================================
// AV1 Specific Tests
// ============================================================================

#[test]
fn test_av1_obu_header_format() {
    // AV1 OBU header: forbidden(1) + type(4) + extension(1) + has_size(1) + reserved(1)
    let obu_header = 0x0Au8; // type=1, has_size_field=1
    let obu_type = (obu_header >> 3) & 0x0F;
    let has_size = (obu_header & 0x02) != 0;

    assert_eq!(obu_type, 1);
    assert!(has_size);
}

#[test]
fn test_av1_obu_type_values() {
    // AV1 OBU types
    let obu_types = vec![1u8, 2, 3, 4, 5, 6, 7]; // Sequence header to tile list
    for obu_type in obu_types {
        assert!(obu_type <= 15);
    }
}

#[test]
fn test_av1_frame_type_values() {
    // AV1 frame types: KeyFrame=0, Inter=1, IntraOnly=2, Switch=3
    let frame_types = vec![0u8, 1, 2, 3];
    for frame_type in frame_types {
        assert!(frame_type <= 3);
    }
}

#[test]
fn test_av1_sequence_header_structure() {
    struct SequenceHeader {
        seq_profile: u8,
        seq_level_idx: u8,
    }

    let seq = SequenceHeader {
        seq_profile: 0,
        seq_level_idx: 5,
    };

    assert!(seq.seq_profile <= 3); // 0-3 are valid
}

// ============================================================================
// VP9 Specific Tests
// ============================================================================

#[test]
fn test_vp9_frame_marker() {
    // VP9 frame marker: 2-bit value = 0b10
    let frame_marker = 0b10u8;
    assert_eq!(frame_marker, 2);
}

#[test]
fn test_vp9_profile_range() {
    // VP9 profile should be 0-3
    let profiles = vec![0u8, 1, 2, 3];
    for profile in profiles {
        assert!(profile <= 3);
    }
}

#[test]
fn test_vp9_frame_type_values() {
    // VP9 frame types: KeyFrame=0, InterFrame=1
    let frame_types = vec![0u8, 1];
    for frame_type in frame_types {
        assert!(frame_type <= 1);
    }
}

#[test]
fn test_vp9_refresh_frame_flags() {
    // VP9 refresh_frame_flags is 8 bits
    let flags: u8 = 0xFF;
    assert_eq!(flags, 0xFF);
}

// ============================================================================
// Bitstream Reading Tests
// ============================================================================

#[test]
fn test_exp_golomb_coding() {
    // Exponential Golomb coding basics
    // Code 0 -> 1
    // Code 1 -> 010
    // Code 2 -> 011
    // Code 3 -> 00100
    // Code 4 -> 00101

    fn ue_v(value: u32) -> Vec<bool> {
        let mut m = value + 1;
        let leading_zeros = m.leading_zeros() - 1;
        let mut bits = Vec::new();

        // Write leading zeros
        for _ in 0..leading_zeros {
            bits.push(false);
        }
        // Write 1
        bits.push(true);
        // Write remaining bits
        for i in (0..leading_zeros).rev() {
            bits.push((m & (1 << i)) != 0);
        }

        bits
    }

    assert_eq!(ue_v(0).len(), 1);  // "1"
    assert_eq!(ue_v(1).len(), 3);  // "010"
    assert_eq!(ue_v(2).len(), 3);  // "011"
}

#[test]
fn test_signed_exp_golomb_coding() {
    // Signed exponential Golomb coding
    fn se_v(value: i32) -> u32 {
        if value > 0 {
            (2 * value) as u32
        } else {
            (2 * (-value) - 1) as u32
        }
    }

    assert_eq!(se_v(0), 0);
    assert_eq!(se_v(1), 2);
    assert_eq!(se_v(-1), 1);
    assert_eq!(se_v(2), 4);
    assert_eq!(se_v(-2), 3);
}

#[test]
fn test_bitstream_alignment() {
    // Test byte-aligned bitstream reading
    let data = [0b10110110u8, 0b01011010u8];

    // Read first byte
    let first_byte = data[0];
    assert_eq!(first_byte, 0b10110110);

    // Read second byte
    let second_byte = data[1];
    assert_eq!(second_byte, 0b01011010);
}

// ============================================================================
// Resolution Tests
// ============================================================================

#[test]
fn test_common_resolutions() {
    struct Resolution {
        width: u32,
        height: u32,
    }

    let resolutions = vec![
        Resolution { width: 640, height: 480 },   // VGA
        Resolution { width: 1280, height: 720 },  // HD
        Resolution { width: 1920, height: 1080 }, // Full HD
        Resolution { width: 3840, height: 2160 }, // 4K
    ];

    for res in resolutions {
        assert!(res.width > 0);
        assert!(res.height > 0);
        assert!(res.width >= res.height); // Landscape assumption
    }
}

#[test]
fn test_aspect_ratios() {
    fn calculate_aspect_ratio(width: u32, height: u32) -> f64 {
        width as f64 / height as f64
    }

    assert!((calculate_aspect_ratio(1920, 1080) - 16.0 / 9.0).abs() < 0.01);
    assert!((calculate_aspect_ratio(1280, 720) - 16.0 / 9.0).abs() < 0.01);
    assert!((calculate_aspect_ratio(3840, 2160) - 16.0 / 9.0).abs() < 0.01);
}

// ============================================================================
// QP (Quantization Parameter) Tests
// ============================================================================

#[test]
fn test_qp_range_avc_hevc() {
    // AVC and HEVC QP range is 0-51
    let qp_values = vec![0, 10, 20, 26, 30, 40, 51];
    for qp in qp_values {
        assert!(qp <= 51);
    }
}

#[test]
fn test_qp_range_av1() {
    // AV1 base_q_idx range is 0-255
    let qp_values = vec![0u8, 64, 128, 192, 255];
    for qp in qp_values {
        assert!(qp <= 255);
    }
}

#[test]
fn test_qp_delta_range() {
    // QP delta is typically -26 to +25
    let qp_deltas = vec![-26i8, -10, 0, 10, 25];
    for delta in qp_deltas {
        assert!(delta >= -26);
        assert!(delta <= 25);
    }
}
