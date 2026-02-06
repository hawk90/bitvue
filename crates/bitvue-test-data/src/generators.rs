//! Video data generators for testing
//!
//! Provides functions to generate valid and invalid video data
//! for various codecs and containers.

use std::io::Write;

/// IVF header structure
#[derive(Debug, Clone)]
pub struct IvfHeader {
    pub signature: [u8; 4],  // "DKIF"
    pub version: u16,         // Usually 0
    pub header_size: u16,     // Usually 32
    pub fourcc: [u8; 4],      // e.g., "AV01"
    pub width: u16,
    pub height: u16,
    pub timebase_den: u32,
    pub timebase_num: u32,
    pub num_frames: u32,
    pub reserved: [u8; 4],
}

impl Default for IvfHeader {
    fn default() -> Self {
        Self {
            signature: *b"DKIF",
            version: 0,
            header_size: 32,
            fourcc: *b"AV01",
            width: 320,
            height: 240,
            timebase_den: 30,
            timebase_num: 1,
            num_frames: 1,
            reserved: [0; 4],
        }
    }
}

impl IvfHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);

        bytes.write_all(&self.signature).unwrap();
        bytes.write_all(&self.version.to_le_bytes()).unwrap();
        bytes.write_all(&self.header_size.to_le_bytes()).unwrap();
        bytes.write_all(&self.fourcc).unwrap();
        bytes.write_all(&self.width.to_le_bytes()).unwrap();
        bytes.write_all(&self.height.to_le_bytes()).unwrap();
        bytes.write_all(&self.timebase_den.to_le_bytes()).unwrap();
        bytes.write_all(&self.timebase_num.to_le_bytes()).unwrap();
        bytes.write_all(&self.num_frames.to_le_bytes()).unwrap();
        bytes.write_all(&self.reserved).unwrap();

        bytes
    }
}

/// Create a minimal IVF file header
pub fn create_ivf_header() -> Vec<u8> {
    IvfHeader::default().to_bytes()
}

/// Create an IVF file with specified parameters
pub fn create_ivf_with_params(
    width: u16,
    height: u16,
    num_frames: u32,
    frame_data_size: usize,
) -> Vec<u8> {
    let header = IvfHeader {
        width,
        height,
        num_frames,
        ..Default::default()
    };

    let mut data = header.to_bytes();

    for i in 0..num_frames {
        // IVF frame header
        data.write_all(&(frame_data_size as u32).to_le_bytes()).unwrap();
        data.write_all(&(i as u64).to_le_bytes()).unwrap(); // Timestamp

        // Frame data (dummy)
        data.extend(vec![0u8; frame_data_size]);
    }

    data
}

/// Create a minimal IVF file with one frame
pub fn create_minimal_ivf() -> Vec<u8> {
    create_ivf_with_params(320, 240, 1, 1000)
}

/// Create an IVF file with N frames
pub fn create_ivf_with_n_frames(num_frames: usize, width: u16, height: u16) -> Vec<u8> {
    create_ivf_with_params(width, height, num_frames as u32, 1000)
}

/// Create an IVF file with a specific frame size
pub fn create_ivf_header_with_frame_size(frame_size: u32) -> Vec<u8> {
    let header = IvfHeader {
        num_frames: 1,
        ..Default::default()
    };

    let mut data = header.to_bytes();

    // IVF frame header
    data.write_all(&frame_size.to_le_bytes()).unwrap();
    data.write_all(&0u64.to_le_bytes()).unwrap(); // Timestamp

    data
}

/// AV1 OBU header structure
pub struct ObuHeader {
    pub obu_type: u8,
    pub has_extension: bool,
    pub has_size_field: bool,
    pub forbidden: bool,
}

impl ObuHeader {
    pub fn to_bytes(&self, size: Option<u32>) -> Vec<u8> {
        let mut byte = 0u8;

        byte |= self.obu_type & 0x0F;
        if self.forbidden {
            byte |= 0x80;
        }
        if self.has_size_field {
            byte |= 0x20;
        }
        if self.has_extension {
            byte |= 0x10;
        }

        let mut bytes = vec![byte];

        // Add size field if present
        if self.has_size_field {
            if let Some(size) = size {
                // Write LEB128 size
                let mut size = size;
                loop {
                    let mut byte = (size & 0x7F) as u8;
                    size >>= 7;
                    if size > 0 {
                        byte |= 0x80;
                    }
                    bytes.push(byte);
                    if size == 0 {
                        break;
                    }
                }
            }
        }

        bytes
    }
}

/// Create a minimal OBU frame
pub fn create_minimal_obu_frame() -> Vec<u8> {
    let header = ObuHeader {
        obu_type: 1, // Sequence header
        has_extension: false,
        has_size_field: true,
        forbidden: false,
    };

    let mut data = header.to_bytes(Some(10));
    // Add some OBU payload
    data.extend_from_slice(&[0u8; 10]);

    data
}

/// Create an OBU with specified type
pub fn create_obu_of_type(obu_type: u8) -> Vec<u8> {
    let header = ObuHeader {
        obu_type,
        has_extension: false,
        has_size_field: true,
        forbidden: false,
    };

    let mut data = header.to_bytes(Some(10));
    data.extend_from_slice(&[0u8; 10]);

    data
}

/// Create an OBU with Annex B start code
pub fn create_obu_with_annex_b_start() -> Vec<u8> {
    let mut data = vec![0x00, 0x00, 0x01]; // Annex B start code
    data.extend(create_minimal_obu_frame());

    data
}

/// Create an AV1 sequence header with specified parameters
pub fn create_seq_header_with_profile_level(profile: u8, level: u8) -> Vec<u8> {
    let mut data = create_obu_of_type(1); // Sequence header

    // Sequence header OBU payload (simplified)
    let payload_start = data.len();
    data.push((profile << 5) | (1 << 3)); // seq_profile, still_picture
    data.push(level); // seq_level_idx

    // Add OBU payload size
    let payload_size = data.len() - payload_start;
    // Update OBU header size (simplified)

    data
}

/// Create AV1 sequence header with specific frame rate
pub fn create_seq_header_with_framerate(framerate: u32) -> Vec<u8> {
    let mut data = create_seq_header_with_profile_level(0, 0);

    // Timing info (simplified)
    data.extend_from_slice(&1u32.to_le_bytes()); // num_units_in_display_tick
    data.extend_from_slice(&framerate.to_le_bytes()); // time_scale

    data
}

/// Create AV1 with color configuration
pub fn create_av1_with_color_config(
    bit_depth: u8,
    _yuv_range: YuvRange,
    _color_primaries: ColorPrimaries,
    _transfer_characteristics: TransferCharacteristics,
    _matrix_coefficients: MatrixCoefficients,
) -> Vec<u8> {
    let mut data = create_seq_header_with_profile_level(0, 0);

    // Color config (simplified)
    data.push(bit_depth); // Bit depth
    data.push(0); // Mono chrome
    // ... rest of color config

    data
}

#[derive(Debug, Clone, Copy)]
pub enum YuvRange {
    Limited,
    Full,
}

#[derive(Debug, Clone, Copy)]
pub enum ColorPrimaries {
    Bt709,
    Bt2020,
}

#[derive(Debug, Clone, Copy)]
pub enum TransferCharacteristics {
    Bt709,
    SmpteSt2084,
}

#[derive(Debug, Clone, Copy)]
pub enum MatrixCoefficients {
    Bt709,
    Bt2020Nc,
    Identity,
}

/// Create AV1 with invalid tiles
pub fn create_av1_with_invalid_tiles(tile_cols: u32, tile_rows: u32) -> Vec<u8> {
    let mut data = create_seq_header_with_profile_level(0, 0);

    // Tile info (simplified - would be more complex in reality)
    // In real AV1, tile_cols is encoded as (tile_cols - 1)
    // Max is usually much smaller than 256
    data.push((tile_cols - 1) as u8);
    data.push((tile_rows - 1) as u8);

    data
}

/// Create AV1 with film grain
pub fn create_av1_with_film_grain(
    grain_seed: u64,
    num_y_points: u8,
    num_cb_points: u8,
    num_cr_points: u8,
) -> Vec<u8> {
    let mut data = create_seq_header_with_profile_level(0, 0);

    // Film grain params (simplified)
    data.push(1); // apply_grain = true
    data.extend_from_slice(&grain_seed.to_le_bytes());
    data.push(num_y_points);
    data.push(num_cb_points);
    data.push(num_cr_points);

    data
}

/// Create H.264 NAL unit with type
pub fn create_nal_with_type(nal_type: u8) -> Vec<u8> {
    let mut data = vec![0x00, 0x00, 0x01]; // NAL start code
    data.push(nal_type << 1); // NAL header (ref_idc = 0)

    // Add some payload
    data.extend_from_slice(&[0u8; 100]);

    data
}

/// Create H.264 SPS with profile
pub fn create_sps_with_params(profile: u8, constraint_set: u8) -> Vec<u8> {
    let mut data = vec![0x00, 0x00, 0x01]; // NAL start code
    data.push(0x07); // NAL type = SPS

    // SPS payload (simplified)
    data.push(profile);
    data.push(constraint_set);

    data
}

/// Create H.264 stream with CABAC
pub fn create_h264_stream_with_cabac() -> Vec<u8> {
    let mut data = create_sps_with_params(77, 0); // High profile

    // Add PPS
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x08]); // PPS

    // Add slice header with CABAC
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x05]); // IDR slice

    data
}

/// Create H.264 stream with CAVLC
pub fn create_h264_stream_with_cavlc() -> Vec<u8> {
    let mut data = create_sps_with_params(66, 0); // Baseline profile (CAVLC)

    data
}

/// Create H.264 with mixed slice types
pub fn create_h264_with_mixed_slices() -> Vec<u8> {
    let mut data = create_sps_with_params(77, 0);

    // I-slice
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x05]); // IDR
    data.extend_from_slice(&[0u8; 100]);

    // P-slice
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x01]); // Non-IDR
    data.extend_from_slice(&[0u8; 100]);

    // B-slice
    data.extend_from_slice(&[0x00, 0x00, 0x01, 0x01]);
    data.extend_from_slice(&[0u8; 100]);

    data
}

/// Create interlaced H.264 stream
pub fn create_interlaced_h264_stream() -> Vec<u8> {
    let mut data = create_sps_with_params(77, 0);

    // Set frame_mbs_only_flag = 0 in SPS payload
    // (simplified)

    data
}

/// Create H.264 with MBAFF
pub fn create_h264_with_mbff() -> Vec<u8> {
    let mut data = create_sps_with_params(77, 0);

    // Set mb_adaptive_frame_field_flag in SPS
    // (simplified)

    data
}

/// Create HEVC NAL unit
pub fn create_hevc_nal_with_type(nal_type: u8) -> Vec<u8> {
    let mut data = vec![0x00, 0x00, 0x01]; // NAL start code

    // HEVC NAL header (2 bytes)
    let forbidden_zero_bit = 0;
    let nal_unit_type = nal_type;
    let nuh_layer_id = 0;
    let nuh_temporal_id_plus1 = 1;

    let header = (forbidden_zero_bit << 7)
        | (nal_unit_type << 1)
        | (nuh_layer_id >> 5);
    let header2 = ((nuh_layer_id & 0x1F) << 3) | (nuh_temporal_id_plus1 & 0x07);

    data.push(header);
    data.push(header2);

    data.extend_from_slice(&[0u8; 100]);

    data
}

/// Create HEVC with parameters
pub fn create_hevc_with_params(profile: u8, tier: u8, level: u8) -> Vec<u8> {
    let mut data = create_hevc_nal_with_type(32); // VPS

    // Add SPS
    data.extend(create_hevc_nal_with_type(33));

    data
}

/// Create VP9 frame
pub fn create_vp9_frame() -> Vec<u8> {
    let mut data = vec![0x82, 0x49, 0x83]; // VP9 frame header

    // Sync code
    data.extend_from_slice(&[0x49, 0x83, 0x42]);

    data.extend_from_slice(&[0u8; 1000]);

    data
}

/// Create VP9 superframe
pub fn create_vp9_superframe(frames: Vec<Vec<u8>>) -> Vec<u8> {
    let mut data = Vec::new();

    // Add all frames
    for frame in &frames {
        data.extend_from_slice(frame);
    }

    // Add superframe index
    let mut index = Vec::new();
    index.extend_from_slice(b"\x00\x00\x00"); // Superframe magic

    // Frame sizes
    for frame in &frames {
        index.extend_from_slice(&(frame.len() as u32).to_le_bytes()[..3]);
    }

    // CRC
    index.push(0); // Dummy CRC

    data.extend_from_slice(&index);

    data
}

/// Create VP9 with profile
pub fn create_vp9_with_profile(profile: u8, bit_depth: u8, format: YuvFormat) -> Vec<u8> {
    let mut data = vec![0x82, 0x49, 0x83]; // VP9 frame header

    // Profile in frame header
    data[0] = (profile << 4) | (bit_depth - 8);

    // Color config based on format
    match format {
        YuvFormat::Yuv420 => {
            data[0] |= 0x00;
        }
        YuvFormat::Yuv444 => {
            data[0] |= 0x10;
        }
    }

    data.extend_from_slice(&[0u8; 1000]);

    data
}

#[derive(Debug, Clone, Copy)]
pub enum YuvFormat {
    Yuv420,
    Yuv444,
}

/// Create MP4 file type box
pub fn create_mp4_ftyp(brand: &[u8; 4]) -> Vec<u8> {
    let mut data = Vec::new();

    // Box header
    data.extend_from_slice(&24u32.to_be_bytes()); // Box size
    data.extend_from_slice(b"ftyp");

    // Major brand
    data.extend_from_slice(brand);

    // Minor version
    data.extend_from_slice(&0u32.to_be_bytes());

    // Compatible brands
    data.extend_from_slice(b"isom");
    data.extend_from_slice(b"av01");

    data
}

/// Create MP4 with invalid ftyp
pub fn create_mp4_with_invalid_ftyp(brand: &[u8; 4]) -> Vec<u8> {
    let mut data = create_mp4_ftyp(brand);

    // Add moov (empty, just for structure)
    data.extend_from_slice(&32u32.to_be_bytes());
    data.extend_from_slice(b"moov");

    data
}

/// Create MP4 without moov
pub fn create_mp4_without_moov() -> Vec<u8> {
    let mut data = create_mp4_ftyp(b"isom");

    // Add mdat (no moov)
    data.extend_from_slice(&16u32.to_be_bytes());
    data.extend_from_slice(b"mdat");

    data
}

/// Create fragmented MP4
pub fn create_fragmented_mp4(num_fragments: usize, frames_per_fragment: usize) -> Vec<u8> {
    let mut data = create_mp4_ftyp(b"iso5");

    // ftyp + moov + moof for each fragment + mdat

    data
}

/// Create MKV with EBML header
pub fn create_mkv_with_ebml() -> Vec<u8> {
    let mut data = Vec::new();

    // EBML header
    data.extend_from_slice(&[0x1A, 0x45, 0xDF, 0xA3]); // EBML ID
    data.extend_from_slice(&[0x80]); // Data size (unknown)

    data
}

/// Create MKV with invalid EBML
pub fn create_mkv_with_invalid_ebml() -> Vec<u8> {
    let mut data = Vec::new();

    // Invalid EBML ID (unknown)
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

    data
}

/// Create MKV with deep nesting
pub fn create_mkv_with_nesting_depth(depth: usize) -> Vec<u8> {
    let mut data = create_mkv_with_ebml();

    // Add nested elements
    for _ in 0..depth {
        data.extend_from_slice(&[0xAE]); // Unknown element ID
        data.extend_from_slice(&[0x80]); // Unknown size
    }

    data
}

/// Create MKV with unknown elements
pub fn create_mkv_with_unknown_elements() -> Vec<u8> {
    let mut data = create_mkv_with_ebml();

    // Unknown/skippable elements
    data.extend_from_slice(&[0xEC, 0x80]); // Void element
    data.extend_from_slice(&[0xBF, 0x80]); // Unknown

    data
}

/// Create MKV with CRC element
pub fn create_mkv_with_crc() -> Vec<u8> {
    let mut data = create_mkv_with_ebml();

    // CRC-32 element
    data.extend_from_slice(&[0x3F, 0x85]); // CRC-32 ID
    data.extend_from_slice(&[0x84]); // Size (4 bytes)
    data.extend_from_slice(&[0u8; 4]); // CRC value

    data
}
