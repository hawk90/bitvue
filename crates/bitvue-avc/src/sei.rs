//! H.264/AVC Supplemental Enhancement Information (SEI) parsing.

use crate::bitreader::BitReader;
use crate::error::{AvcError, Result};
use serde::{Deserialize, Serialize};

/// SEI payload types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum SeiPayloadType {
    /// Buffering period
    BufferingPeriod = 0,
    /// Picture timing
    PicTiming = 1,
    /// Pan-scan rectangle
    PanScanRect = 2,
    /// Filler payload
    FillerPayload = 3,
    /// User data registered by ITU-T Rec. T.35
    UserDataRegisteredItuTT35 = 4,
    /// User data unregistered
    UserDataUnregistered = 5,
    /// Recovery point
    RecoveryPoint = 6,
    /// Decoded reference picture marking repetition
    DecRefPicMarkingRepetition = 7,
    /// Spare picture
    SparePic = 8,
    /// Scene information
    SceneInfo = 9,
    /// Sub-sequence information
    SubSeqInfo = 10,
    /// Sub-sequence layer characteristics
    SubSeqLayerCharacteristics = 11,
    /// Sub-sequence characteristics
    SubSeqCharacteristics = 12,
    /// Full-frame freeze
    FullFrameFreeze = 13,
    /// Full-frame freeze release
    FullFrameFreezeRelease = 14,
    /// Full-frame snapshot
    FullFrameSnapshot = 15,
    /// Progressive refinement segment start
    ProgressiveRefinementSegmentStart = 16,
    /// Progressive refinement segment end
    ProgressiveRefinementSegmentEnd = 17,
    /// Motion-constrained slice group set
    MotionConstrainedSliceGroupSet = 18,
    /// Film grain characteristics
    FilmGrainCharacteristics = 19,
    /// Deblocking filter display preference
    DeblockingFilterDisplayPreference = 20,
    /// Stereo video information
    StereoVideoInfo = 21,
    /// Post-filter hint
    PostFilterHint = 22,
    /// Tone mapping information
    ToneMappingInfo = 23,
    /// Scalability information
    ScalabilityInfo = 24,
    /// Sub-picture scalable layer
    SubPicScalableLayer = 25,
    /// Non-required layer representation
    NonRequiredLayerRep = 26,
    /// Priority layer information
    PriorityLayerInfo = 27,
    /// Layers not present
    LayersNotPresent = 28,
    /// Layer dependency change
    LayerDependencyChange = 29,
    /// Scalable nesting
    ScalableNesting = 30,
    /// Base layer temporal HRD
    BaseLayerTemporalHrd = 31,
    /// Quality layer integrity check
    QualityLayerIntegrityCheck = 32,
    /// Redundant picture property
    RedundantPicProperty = 33,
    /// TL0 dependency representation index
    Tl0DepRepIndex = 34,
    /// TL switching point
    TlSwitchingPoint = 35,
    /// Parallel decoding information
    ParallelDecodingInfo = 36,
    /// MVC scalable nesting
    MvcScalableNesting = 37,
    /// View scalability information
    ViewScalabilityInfo = 38,
    /// Multiview scene information
    MultiviewSceneInfo = 39,
    /// Multiview acquisition information
    MultiviewAcquisitionInfo = 40,
    /// Non-required view component
    NonRequiredViewComponent = 41,
    /// View dependency change
    ViewDependencyChange = 42,
    /// Operation points not present
    OperationPointsNotPresent = 43,
    /// Base view temporal HRD
    BaseViewTemporalHrd = 44,
    /// Frame packing arrangement
    FramePackingArrangement = 45,
    /// Multiview view position
    MultiviewViewPosition = 46,
    /// Display orientation
    DisplayOrientation = 47,
    /// MVCD scalable nesting
    MvcdScalableNesting = 48,
    /// MVCD view scalability information
    MvcdViewScalabilityInfo = 49,
    /// Depth representation information
    DepthRepresentationInfo = 50,
    /// 3D reference displays information
    ThreeDRefDisplaysInfo = 51,
    /// Depth timing
    DepthTiming = 52,
    /// Depth sampling information
    DepthSamplingInfo = 53,
    /// Constrained depth parameter set identifier
    ConstrainedDepthParameterSetId = 54,
    /// Mastering display colour volume
    MasteringDisplayColourVolume = 137,
    /// Content light level information
    ContentLightLevelInfo = 144,
    /// Alternative transfer characteristics
    AlternativeTransferCharacteristics = 147,
    /// Unknown
    Unknown = 255,
}

impl SeiPayloadType {
    /// Create from raw value.
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => SeiPayloadType::BufferingPeriod,
            1 => SeiPayloadType::PicTiming,
            2 => SeiPayloadType::PanScanRect,
            3 => SeiPayloadType::FillerPayload,
            4 => SeiPayloadType::UserDataRegisteredItuTT35,
            5 => SeiPayloadType::UserDataUnregistered,
            6 => SeiPayloadType::RecoveryPoint,
            7 => SeiPayloadType::DecRefPicMarkingRepetition,
            19 => SeiPayloadType::FilmGrainCharacteristics,
            45 => SeiPayloadType::FramePackingArrangement,
            47 => SeiPayloadType::DisplayOrientation,
            137 => SeiPayloadType::MasteringDisplayColourVolume,
            144 => SeiPayloadType::ContentLightLevelInfo,
            147 => SeiPayloadType::AlternativeTransferCharacteristics,
            _ => SeiPayloadType::Unknown,
        }
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            SeiPayloadType::BufferingPeriod => "Buffering Period",
            SeiPayloadType::PicTiming => "Picture Timing",
            SeiPayloadType::PanScanRect => "Pan-Scan Rectangle",
            SeiPayloadType::FillerPayload => "Filler Payload",
            SeiPayloadType::UserDataRegisteredItuTT35 => "User Data (ITU-T T.35)",
            SeiPayloadType::UserDataUnregistered => "User Data (Unregistered)",
            SeiPayloadType::RecoveryPoint => "Recovery Point",
            SeiPayloadType::DecRefPicMarkingRepetition => "Dec Ref Pic Marking Repetition",
            SeiPayloadType::FilmGrainCharacteristics => "Film Grain Characteristics",
            SeiPayloadType::FramePackingArrangement => "Frame Packing Arrangement",
            SeiPayloadType::DisplayOrientation => "Display Orientation",
            SeiPayloadType::MasteringDisplayColourVolume => "Mastering Display Colour Volume",
            SeiPayloadType::ContentLightLevelInfo => "Content Light Level Info",
            SeiPayloadType::AlternativeTransferCharacteristics => {
                "Alternative Transfer Characteristics"
            }
            _ => "Unknown",
        }
    }
}

/// SEI message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeiMessage {
    /// Payload type.
    pub payload_type: SeiPayloadType,
    /// Raw payload type value.
    pub payload_type_raw: u32,
    /// Payload size.
    pub payload_size: u32,
    /// Raw payload data.
    pub payload: Vec<u8>,
    /// Parsed data (for known types).
    pub parsed: Option<SeiParsedData>,
}

/// Parsed SEI data for known types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeiParsedData {
    /// Recovery point SEI.
    RecoveryPoint {
        recovery_frame_cnt: u32,
        exact_match_flag: bool,
        broken_link_flag: bool,
        changing_slice_group_idc: u8,
    },
    /// User data unregistered.
    UserDataUnregistered { uuid: [u8; 16], data: Vec<u8> },
    /// Mastering display colour volume.
    MasteringDisplayColourVolume {
        display_primaries_x: [u16; 3],
        display_primaries_y: [u16; 3],
        white_point_x: u16,
        white_point_y: u16,
        max_display_mastering_luminance: u32,
        min_display_mastering_luminance: u32,
    },
    /// Content light level info.
    ContentLightLevelInfo {
        max_content_light_level: u16,
        max_pic_average_light_level: u16,
    },
}

/// Parse SEI messages from NAL unit payload.
pub fn parse_sei(data: &[u8]) -> Result<Vec<SeiMessage>> {
    // SECURITY: Limit iterations to prevent DoS attacks.
    // According to H.264/AVC spec, each SEI type should only appear once per NAL.
    const MAX_SEI_TYPE_ITERATIONS: u8 = 1;
    const MAX_SEI_SIZE_ITERATIONS: u8 = 1;

    let mut messages = Vec::new();
    let mut offset = 0;

    // SECURITY: Reject obviously malicious inputs early to prevent DoS.
    // Consecutive 0xFF bytes indicate state machine confusion attack.
    // Check for runs of 5+ consecutive 0xFF bytes (emulation prevention pattern).
    let mut consecutive_ff = 0;
    for byte in data.iter().take(200) {
        if *byte == 0xFF {
            consecutive_ff += 1;
            if consecutive_ff > 5 {
                return Err(AvcError::InvalidSei(
                    "SEI payload contains suspicious consecutive emulation prevention bytes".to_string(),
                ));
            }
        } else {
            consecutive_ff = 0;
        }
    }

    while offset < data.len() {
        // Read payload type
        let mut payload_type: u32 = 0;
        let mut type_iterations = 0;
        while offset < data.len() && data[offset] == 0xFF {
            if type_iterations >= MAX_SEI_TYPE_ITERATIONS {
                return Err(AvcError::InvalidSei(
                    "SEI payload type extension exceeds maximum iterations".to_string(),
                ));
            }
            payload_type += 255;
            offset += 1;
            type_iterations += 1;
        }
        if offset >= data.len() {
            break;
        }
        payload_type += data[offset] as u32;
        offset += 1;

        // Read payload size
        let mut payload_size: u32 = 0;
        let mut size_iterations = 0;
        while offset < data.len() && data[offset] == 0xFF {
            if size_iterations >= MAX_SEI_SIZE_ITERATIONS {
                return Err(AvcError::InvalidSei(
                    "SEI payload size extension exceeds maximum iterations".to_string(),
                ));
            }
            payload_size += 255;
            offset += 1;
            size_iterations += 1;
        }
        if offset >= data.len() {
            break;
        }
        payload_size += data[offset] as u32;
        offset += 1;

        // Extract payload
        let end = (offset + payload_size as usize).min(data.len());
        let payload = data[offset..end].to_vec();
        offset = end;

        let sei_type = SeiPayloadType::from_u32(payload_type);
        let parsed = parse_sei_payload(sei_type, &payload);

        messages.push(SeiMessage {
            payload_type: sei_type,
            payload_type_raw: payload_type,
            payload_size,
            payload,
            parsed,
        });

        // Check for RBSP trailing bits / stop
        if offset < data.len() && data[offset] == 0x80 {
            break;
        }
    }

    Ok(messages)
}

/// Parse specific SEI payload.
fn parse_sei_payload(payload_type: SeiPayloadType, data: &[u8]) -> Option<SeiParsedData> {
    match payload_type {
        SeiPayloadType::RecoveryPoint => parse_recovery_point(data),
        SeiPayloadType::UserDataUnregistered => parse_user_data_unregistered(data),
        SeiPayloadType::MasteringDisplayColourVolume => parse_mastering_display(data),
        SeiPayloadType::ContentLightLevelInfo => parse_content_light_level(data),
        _ => None,
    }
}

/// Parse recovery point SEI.
fn parse_recovery_point(data: &[u8]) -> Option<SeiParsedData> {
    let mut reader = BitReader::new(data);

    let recovery_frame_cnt = reader.read_ue().ok()?;
    let exact_match_flag = reader.read_flag().ok()?;
    let broken_link_flag = reader.read_flag().ok()?;
    let changing_slice_group_idc = reader.read_bits(2).ok()? as u8;

    Some(SeiParsedData::RecoveryPoint {
        recovery_frame_cnt,
        exact_match_flag,
        broken_link_flag,
        changing_slice_group_idc,
    })
}

/// Parse user data unregistered SEI.
fn parse_user_data_unregistered(data: &[u8]) -> Option<SeiParsedData> {
    if data.len() < 16 {
        return None;
    }

    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&data[0..16]);

    let user_data = data[16..].to_vec();

    Some(SeiParsedData::UserDataUnregistered {
        uuid,
        data: user_data,
    })
}

/// Parse mastering display colour volume SEI.
fn parse_mastering_display(data: &[u8]) -> Option<SeiParsedData> {
    if data.len() < 24 {
        return None;
    }

    let mut display_primaries_x = [0u16; 3];
    let mut display_primaries_y = [0u16; 3];

    for i in 0..3 {
        // SECURITY: Use get() for bounds-checked access
        let base = i * 4;
        display_primaries_x[i] = u16::from_be_bytes([
            data.get(base).copied().unwrap_or(0),
            data.get(base + 1).copied().unwrap_or(0),
        ]);
        display_primaries_y[i] = u16::from_be_bytes([
            data.get(base + 2).copied().unwrap_or(0),
            data.get(base + 3).copied().unwrap_or(0),
        ]);
    }

    let white_point_x = u16::from_be_bytes([
        data.get(12).copied().unwrap_or(0),
        data.get(13).copied().unwrap_or(0),
    ]);
    let white_point_y = u16::from_be_bytes([
        data.get(14).copied().unwrap_or(0),
        data.get(15).copied().unwrap_or(0),
    ]);
    let max_display_mastering_luminance = u32::from_be_bytes([
        data.get(16).copied().unwrap_or(0),
        data.get(17).copied().unwrap_or(0),
        data.get(18).copied().unwrap_or(0),
        data.get(19).copied().unwrap_or(0),
    ]);
    let min_display_mastering_luminance = u32::from_be_bytes([
        data.get(20).copied().unwrap_or(0),
        data.get(21).copied().unwrap_or(0),
        data.get(22).copied().unwrap_or(0),
        data.get(23).copied().unwrap_or(0),
    ]);

    Some(SeiParsedData::MasteringDisplayColourVolume {
        display_primaries_x,
        display_primaries_y,
        white_point_x,
        white_point_y,
        max_display_mastering_luminance,
        min_display_mastering_luminance,
    })
}

/// Parse content light level info SEI.
fn parse_content_light_level(data: &[u8]) -> Option<SeiParsedData> {
    if data.len() < 4 {
        return None;
    }

    let max_content_light_level = u16::from_be_bytes([data[0], data[1]]);
    let max_pic_average_light_level = u16::from_be_bytes([data[2], data[3]]);

    Some(SeiParsedData::ContentLightLevelInfo {
        max_content_light_level,
        max_pic_average_light_level,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sei_payload_type() {
        assert_eq!(SeiPayloadType::from_u32(0), SeiPayloadType::BufferingPeriod);
        assert_eq!(SeiPayloadType::from_u32(6), SeiPayloadType::RecoveryPoint);
        assert_eq!(
            SeiPayloadType::from_u32(137),
            SeiPayloadType::MasteringDisplayColourVolume
        );
    }
}
