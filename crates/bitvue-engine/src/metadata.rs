//! Metadata Inspector - Feature Parity: HDR/SEI Metadata
//!
//! Per COMPETITOR_PARITY_STATUS.md §4.3:
//! - Metadata inspector (HDR/SEI) - provides UI for viewing stream metadata
//!
//! Supports:
//! - HDR metadata (SMPTE ST 2086, CLL, HDR10+, Dolby Vision)
//! - SEI messages (H.264/HEVC/VVC)
//! - OBU metadata (AV1)
//! - VP9 frame metadata

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// HDR Metadata Types
// ═══════════════════════════════════════════════════════════════════════════

/// Mastering Display Color Volume (SMPTE ST 2086)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringDisplayMetadata {
    /// Display primaries - Red (x, y) in 0.00002 units
    pub red_x: u16,
    pub red_y: u16,
    /// Display primaries - Green (x, y)
    pub green_x: u16,
    pub green_y: u16,
    /// Display primaries - Blue (x, y)
    pub blue_x: u16,
    pub blue_y: u16,
    /// White point (x, y)
    pub white_point_x: u16,
    pub white_point_y: u16,
    /// Max luminance in 0.0001 cd/m² units
    pub max_luminance: u32,
    /// Min luminance in 0.0001 cd/m² units
    pub min_luminance: u32,
}

impl MasteringDisplayMetadata {
    /// Get red primary as normalized (0.0-1.0) coordinates
    pub fn red_normalized(&self) -> (f32, f32) {
        (self.red_x as f32 * 0.00002, self.red_y as f32 * 0.00002)
    }

    /// Get green primary as normalized coordinates
    pub fn green_normalized(&self) -> (f32, f32) {
        (self.green_x as f32 * 0.00002, self.green_y as f32 * 0.00002)
    }

    /// Get blue primary as normalized coordinates
    pub fn blue_normalized(&self) -> (f32, f32) {
        (self.blue_x as f32 * 0.00002, self.blue_y as f32 * 0.00002)
    }

    /// Get white point as normalized coordinates
    pub fn white_point_normalized(&self) -> (f32, f32) {
        (
            self.white_point_x as f32 * 0.00002,
            self.white_point_y as f32 * 0.00002,
        )
    }

    /// Get max luminance in cd/m²
    pub fn max_luminance_cdm2(&self) -> f32 {
        self.max_luminance as f32 * 0.0001
    }

    /// Get min luminance in cd/m²
    pub fn min_luminance_cdm2(&self) -> f32 {
        self.min_luminance as f32 * 0.0001
    }

    /// Format as display text
    pub fn format_display(&self) -> String {
        format!(
            "Mastering Display:\n  Primaries: R({:.4},{:.4}) G({:.4},{:.4}) B({:.4},{:.4})\n  White Point: ({:.4},{:.4})\n  Luminance: {:.1} - {:.4} cd/m²",
            self.red_normalized().0, self.red_normalized().1,
            self.green_normalized().0, self.green_normalized().1,
            self.blue_normalized().0, self.blue_normalized().1,
            self.white_point_normalized().0, self.white_point_normalized().1,
            self.max_luminance_cdm2(), self.min_luminance_cdm2()
        )
    }
}

/// Content Light Level (CLL) metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentLightLevel {
    /// Maximum Content Light Level (cd/m²)
    pub max_cll: u16,
    /// Maximum Frame-Average Light Level (cd/m²)
    pub max_fall: u16,
}

impl ContentLightLevel {
    pub fn new(max_cll: u16, max_fall: u16) -> Self {
        Self { max_cll, max_fall }
    }

    /// Format as display text
    pub fn format_display(&self) -> String {
        format!(
            "Content Light Level:\n  MaxCLL: {} cd/m²\n  MaxFALL: {} cd/m²",
            self.max_cll, self.max_fall
        )
    }
}

/// HDR format type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HdrFormat {
    /// SDR (no HDR metadata)
    Sdr,
    /// HDR10 (static metadata)
    Hdr10,
    /// HDR10+ (dynamic metadata)
    Hdr10Plus,
    /// Dolby Vision
    DolbyVision,
    /// HLG (Hybrid Log-Gamma)
    Hlg,
    /// PQ (Perceptual Quantizer) without metadata
    Pq,
}

impl HdrFormat {
    pub fn name(&self) -> &'static str {
        match self {
            HdrFormat::Sdr => "SDR",
            HdrFormat::Hdr10 => "HDR10",
            HdrFormat::Hdr10Plus => "HDR10+",
            HdrFormat::DolbyVision => "Dolby Vision",
            HdrFormat::Hlg => "HLG",
            HdrFormat::Pq => "PQ",
        }
    }

    pub fn is_hdr(&self) -> bool {
        !matches!(self, HdrFormat::Sdr)
    }
}

/// Color primaries (ITU-T H.273)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorPrimaries {
    Bt709,
    Bt470M,
    Bt470Bg,
    Smpte170M,
    Smpte240M,
    Film,
    Bt2020,
    Xyz,
    Smpte431,
    Smpte432,
    Ebu3213,
    Unknown(u8),
}

impl ColorPrimaries {
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => ColorPrimaries::Bt709,
            4 => ColorPrimaries::Bt470M,
            5 => ColorPrimaries::Bt470Bg,
            6 => ColorPrimaries::Smpte170M,
            7 => ColorPrimaries::Smpte240M,
            8 => ColorPrimaries::Film,
            9 => ColorPrimaries::Bt2020,
            10 => ColorPrimaries::Xyz,
            11 => ColorPrimaries::Smpte431,
            12 => ColorPrimaries::Smpte432,
            22 => ColorPrimaries::Ebu3213,
            _ => ColorPrimaries::Unknown(code),
        }
    }

    pub fn name(&self) -> String {
        match self {
            ColorPrimaries::Bt709 => "BT.709".to_string(),
            ColorPrimaries::Bt470M => "BT.470M".to_string(),
            ColorPrimaries::Bt470Bg => "BT.470BG".to_string(),
            ColorPrimaries::Smpte170M => "SMPTE 170M".to_string(),
            ColorPrimaries::Smpte240M => "SMPTE 240M".to_string(),
            ColorPrimaries::Film => "Film".to_string(),
            ColorPrimaries::Bt2020 => "BT.2020".to_string(),
            ColorPrimaries::Xyz => "XYZ".to_string(),
            ColorPrimaries::Smpte431 => "SMPTE 431 (DCI-P3)".to_string(),
            ColorPrimaries::Smpte432 => "SMPTE 432 (Display P3)".to_string(),
            ColorPrimaries::Ebu3213 => "EBU 3213".to_string(),
            ColorPrimaries::Unknown(c) => format!("Unknown ({})", c),
        }
    }
}

/// Transfer characteristics (ITU-T H.273)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferCharacteristics {
    Bt709,
    Bt470M,
    Bt470Bg,
    Smpte170M,
    Smpte240M,
    Linear,
    Log100,
    Log316,
    Iec61966,
    Bt1361,
    Srgb,
    Bt2020_10bit,
    Bt2020_12bit,
    Pq,
    Smpte428,
    Hlg,
    Unknown(u8),
}

impl TransferCharacteristics {
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => TransferCharacteristics::Bt709,
            4 => TransferCharacteristics::Bt470M,
            5 => TransferCharacteristics::Bt470Bg,
            6 => TransferCharacteristics::Smpte170M,
            7 => TransferCharacteristics::Smpte240M,
            8 => TransferCharacteristics::Linear,
            9 => TransferCharacteristics::Log100,
            10 => TransferCharacteristics::Log316,
            11 => TransferCharacteristics::Iec61966,
            12 => TransferCharacteristics::Bt1361,
            13 => TransferCharacteristics::Srgb,
            14 => TransferCharacteristics::Bt2020_10bit,
            15 => TransferCharacteristics::Bt2020_12bit,
            16 => TransferCharacteristics::Pq,
            17 => TransferCharacteristics::Smpte428,
            18 => TransferCharacteristics::Hlg,
            _ => TransferCharacteristics::Unknown(code),
        }
    }

    pub fn name(&self) -> String {
        match self {
            TransferCharacteristics::Bt709 => "BT.709".to_string(),
            TransferCharacteristics::Bt470M => "BT.470M".to_string(),
            TransferCharacteristics::Bt470Bg => "BT.470BG".to_string(),
            TransferCharacteristics::Smpte170M => "SMPTE 170M".to_string(),
            TransferCharacteristics::Smpte240M => "SMPTE 240M".to_string(),
            TransferCharacteristics::Linear => "Linear".to_string(),
            TransferCharacteristics::Log100 => "Log 100:1".to_string(),
            TransferCharacteristics::Log316 => "Log 316:1".to_string(),
            TransferCharacteristics::Iec61966 => "IEC 61966".to_string(),
            TransferCharacteristics::Bt1361 => "BT.1361".to_string(),
            TransferCharacteristics::Srgb => "sRGB".to_string(),
            TransferCharacteristics::Bt2020_10bit => "BT.2020 10-bit".to_string(),
            TransferCharacteristics::Bt2020_12bit => "BT.2020 12-bit".to_string(),
            TransferCharacteristics::Pq => "PQ (SMPTE ST 2084)".to_string(),
            TransferCharacteristics::Smpte428 => "SMPTE 428".to_string(),
            TransferCharacteristics::Hlg => "HLG (ARIB STD-B67)".to_string(),
            TransferCharacteristics::Unknown(c) => format!("Unknown ({})", c),
        }
    }

    pub fn is_hdr(&self) -> bool {
        matches!(
            self,
            TransferCharacteristics::Pq | TransferCharacteristics::Hlg
        )
    }
}

/// Matrix coefficients (ITU-T H.273)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatrixCoefficients {
    Identity,
    Bt709,
    Fcc,
    Bt470Bg,
    Smpte170M,
    Smpte240M,
    YCgCo,
    Bt2020Ncl,
    Bt2020Cl,
    Smpte2085,
    ChromaDerivedNcl,
    ChromaDerivedCl,
    ICtCp,
    Unknown(u8),
}

impl MatrixCoefficients {
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => MatrixCoefficients::Identity,
            1 => MatrixCoefficients::Bt709,
            4 => MatrixCoefficients::Fcc,
            5 => MatrixCoefficients::Bt470Bg,
            6 => MatrixCoefficients::Smpte170M,
            7 => MatrixCoefficients::Smpte240M,
            8 => MatrixCoefficients::YCgCo,
            9 => MatrixCoefficients::Bt2020Ncl,
            10 => MatrixCoefficients::Bt2020Cl,
            11 => MatrixCoefficients::Smpte2085,
            12 => MatrixCoefficients::ChromaDerivedNcl,
            13 => MatrixCoefficients::ChromaDerivedCl,
            14 => MatrixCoefficients::ICtCp,
            _ => MatrixCoefficients::Unknown(code),
        }
    }

    pub fn name(&self) -> String {
        match self {
            MatrixCoefficients::Identity => "Identity (RGB)".to_string(),
            MatrixCoefficients::Bt709 => "BT.709".to_string(),
            MatrixCoefficients::Fcc => "FCC".to_string(),
            MatrixCoefficients::Bt470Bg => "BT.470BG".to_string(),
            MatrixCoefficients::Smpte170M => "SMPTE 170M".to_string(),
            MatrixCoefficients::Smpte240M => "SMPTE 240M".to_string(),
            MatrixCoefficients::YCgCo => "YCgCo".to_string(),
            MatrixCoefficients::Bt2020Ncl => "BT.2020 NCL".to_string(),
            MatrixCoefficients::Bt2020Cl => "BT.2020 CL".to_string(),
            MatrixCoefficients::Smpte2085 => "SMPTE 2085".to_string(),
            MatrixCoefficients::ChromaDerivedNcl => "Chroma-Derived NCL".to_string(),
            MatrixCoefficients::ChromaDerivedCl => "Chroma-Derived CL".to_string(),
            MatrixCoefficients::ICtCp => "ICtCp".to_string(),
            MatrixCoefficients::Unknown(c) => format!("Unknown ({})", c),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SEI Message Types
// ═══════════════════════════════════════════════════════════════════════════

/// SEI message type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeiMessageType {
    // Common SEI types
    BufferingPeriod,
    PicTiming,
    PanScanRect,
    FillerPayload,
    UserDataRegistered,
    UserDataUnregistered,
    RecoveryPoint,
    DecRefPicMarkingRepetition,
    SparePic,
    SceneInfo,
    SubSeqInfo,
    SubSeqLayerCharacteristics,
    SubSeqCharacteristics,
    FullFrameFreeze,
    FullFrameFreezeRelease,
    FullFrameSnapshot,
    ProgressiveRefinementSegmentStart,
    ProgressiveRefinementSegmentEnd,
    MotionConstrainedSliceGroupSet,
    FilmGrainCharacteristics,
    DeblockingFilterDisplayPreference,
    StereoVideoInfo,
    PostFilterHint,
    ToneMappingInfo,
    ScalabilityInfo,
    SubPicScalableLayer,
    NonRequiredLayerRep,
    PriorityLayerInfo,
    LayersNotPresent,
    LayerDependencyChange,
    ScalableNesting,
    BaseLayerTemporalHrd,
    QualityLayerIntegrityCheck,
    RedundantPicProperty,
    Tl0DepRepIndex,
    TlSwitchingPoint,
    ParallelDecodingInfo,
    MvcScalableNesting,
    ViewScalabilityInfo,
    MultiviewSceneInfo,
    MultiviewAcquisitionInfo,
    NonRequiredViewComponent,
    ViewDependencyChange,
    OperationPointsNotPresent,
    BaseViewTemporalHrd,
    FramePackingArrangement,
    MultiviewViewPosition,
    DisplayOrientation,
    MvcdScalableNesting,
    MvcdViewScalabilityInfo,
    DepthRepresentationInfo,
    ThreeDimensionalReferenceDisplaysInfo,
    DepthTiming,
    DepthSamplingInfo,
    ConstrainedDepthParameterSetIdentifier,
    GreenMetadata,
    MasteringDisplayColourVolume,
    ColourRemappingInfo,
    AlternativeTransferCharacteristics,
    AmbientViewingEnvironment,
    ContentLightLevelInfo,
    AlternativeDepthInfo,
    Unknown(u32),
}

impl SeiMessageType {
    pub fn name(&self) -> &'static str {
        match self {
            SeiMessageType::BufferingPeriod => "Buffering Period",
            SeiMessageType::PicTiming => "Picture Timing",
            SeiMessageType::UserDataRegistered => "User Data Registered",
            SeiMessageType::UserDataUnregistered => "User Data Unregistered",
            SeiMessageType::RecoveryPoint => "Recovery Point",
            SeiMessageType::FilmGrainCharacteristics => "Film Grain Characteristics",
            SeiMessageType::MasteringDisplayColourVolume => "Mastering Display Colour Volume",
            SeiMessageType::ContentLightLevelInfo => "Content Light Level Info",
            SeiMessageType::AlternativeTransferCharacteristics => {
                "Alternative Transfer Characteristics"
            }
            SeiMessageType::FramePackingArrangement => "Frame Packing Arrangement",
            SeiMessageType::DisplayOrientation => "Display Orientation",
            SeiMessageType::GreenMetadata => "Green Metadata",
            _ => "Other",
        }
    }

    pub fn is_hdr_related(&self) -> bool {
        matches!(
            self,
            SeiMessageType::MasteringDisplayColourVolume
                | SeiMessageType::ContentLightLevelInfo
                | SeiMessageType::AlternativeTransferCharacteristics
        )
    }
}

/// SEI message data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeiMessage {
    /// Message type
    pub message_type: SeiMessageType,
    /// Raw payload size
    pub payload_size: usize,
    /// Byte offset in stream
    pub byte_offset: u64,
    /// Frame index (display order)
    pub frame_idx: Option<usize>,
    /// Parsed data (type-specific)
    pub data: SeiData,
}

/// Parsed SEI data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeiData {
    /// Mastering display colour volume
    MasteringDisplay(MasteringDisplayMetadata),
    /// Content light level
    ContentLightLevel(ContentLightLevel),
    /// User data with UUID
    UserDataUnregistered { uuid: [u8; 16], data: Vec<u8> },
    /// User data with ITU-T T.35 code
    UserDataRegistered { country_code: u8, data: Vec<u8> },
    /// Recovery point
    RecoveryPoint { recovery_poc_cnt: i32 },
    /// Picture timing
    PicTiming {
        cpb_removal_delay: Option<u32>,
        dpb_output_delay: Option<u32>,
    },
    /// Film grain characteristics
    FilmGrain { present: bool },
    /// Raw/unparsed data
    Raw(Vec<u8>),
}

// ═══════════════════════════════════════════════════════════════════════════
// Metadata Collection
// ═══════════════════════════════════════════════════════════════════════════

/// Complete stream metadata collection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamMetadata {
    /// HDR format detected
    pub hdr_format: Option<HdrFormat>,
    /// Color primaries
    pub color_primaries: Option<ColorPrimaries>,
    /// Transfer characteristics
    pub transfer_characteristics: Option<TransferCharacteristics>,
    /// Matrix coefficients
    pub matrix_coefficients: Option<MatrixCoefficients>,
    /// Video full range flag
    pub video_full_range: Option<bool>,
    /// Bit depth (luma)
    pub bit_depth: Option<u8>,
    /// Chroma subsampling (e.g., "4:2:0", "4:2:2", "4:4:4")
    pub chroma_subsampling: Option<String>,
    /// Mastering display metadata (if present)
    pub mastering_display: Option<MasteringDisplayMetadata>,
    /// Content light level (if present)
    pub content_light_level: Option<ContentLightLevel>,
    /// SEI messages collected
    pub sei_messages: Vec<SeiMessage>,
    /// Additional key-value metadata
    pub custom: HashMap<String, String>,
}

impl StreamMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if stream has HDR metadata
    pub fn has_hdr(&self) -> bool {
        self.hdr_format.map(|f| f.is_hdr()).unwrap_or(false)
            || self
                .transfer_characteristics
                .map(|t| t.is_hdr())
                .unwrap_or(false)
            || self.mastering_display.is_some()
            || self.content_light_level.is_some()
    }

    /// Get HDR-related SEI messages
    pub fn hdr_sei_messages(&self) -> Vec<&SeiMessage> {
        self.sei_messages
            .iter()
            .filter(|m| m.message_type.is_hdr_related())
            .collect()
    }

    /// Get SEI message count by type
    pub fn sei_count_by_type(&self) -> HashMap<SeiMessageType, usize> {
        let mut counts = HashMap::new();
        for msg in &self.sei_messages {
            *counts.entry(msg.message_type).or_insert(0) += 1;
        }
        counts
    }

    /// Format color info as display string
    pub fn color_info_display(&self) -> String {
        let mut lines = Vec::new();

        if let Some(cp) = &self.color_primaries {
            lines.push(format!("Color Primaries: {}", cp.name()));
        }
        if let Some(tc) = &self.transfer_characteristics {
            lines.push(format!("Transfer: {}", tc.name()));
        }
        if let Some(mc) = &self.matrix_coefficients {
            lines.push(format!("Matrix: {}", mc.name()));
        }
        if let Some(range) = self.video_full_range {
            lines.push(format!("Range: {}", if range { "Full" } else { "Limited" }));
        }
        if let Some(depth) = self.bit_depth {
            lines.push(format!("Bit Depth: {}", depth));
        }
        if let Some(chroma) = &self.chroma_subsampling {
            lines.push(format!("Chroma: {}", chroma));
        }

        if lines.is_empty() {
            "No color info available".to_string()
        } else {
            lines.join("\n")
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Metadata Inspector Panel
// ═══════════════════════════════════════════════════════════════════════════

/// Metadata inspector filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataFilter {
    /// Show HDR metadata
    pub show_hdr: bool,
    /// Show SEI messages
    pub show_sei: bool,
    /// Show color info
    pub show_color: bool,
    /// Filter SEI by type
    pub sei_type_filter: Option<SeiMessageType>,
    /// Text search in metadata
    pub text_search: Option<String>,
}

impl MetadataFilter {
    pub fn all() -> Self {
        Self {
            show_hdr: true,
            show_sei: true,
            show_color: true,
            sei_type_filter: None,
            text_search: None,
        }
    }

    pub fn hdr_only() -> Self {
        Self {
            show_hdr: true,
            show_sei: false,
            show_color: false,
            sei_type_filter: None,
            text_search: None,
        }
    }
}

/// Metadata inspector panel state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataInspector {
    /// Current filter
    pub filter: MetadataFilter,
    /// Selected SEI message index
    pub selected_sei_idx: Option<usize>,
    /// Expanded sections
    pub expanded_hdr: bool,
    pub expanded_color: bool,
    pub expanded_sei: bool,
    /// Sort SEI by
    pub sei_sort: SeiSortColumn,
    pub sei_sort_ascending: bool,
}

impl Default for MetadataInspector {
    fn default() -> Self {
        Self {
            filter: MetadataFilter::all(),
            selected_sei_idx: None,
            expanded_hdr: true,
            expanded_color: true,
            expanded_sei: true,
            sei_sort: SeiSortColumn::ByteOffset,
            sei_sort_ascending: true,
        }
    }
}

/// SEI message sort column
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeiSortColumn {
    ByteOffset,
    FrameIdx,
    Type,
    Size,
}

impl MetadataInspector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle HDR section
    pub fn toggle_hdr(&mut self) {
        self.expanded_hdr = !self.expanded_hdr;
    }

    /// Toggle color section
    pub fn toggle_color(&mut self) {
        self.expanded_color = !self.expanded_color;
    }

    /// Toggle SEI section
    pub fn toggle_sei(&mut self) {
        self.expanded_sei = !self.expanded_sei;
    }

    /// Select SEI message
    pub fn select_sei(&mut self, idx: Option<usize>) {
        self.selected_sei_idx = idx;
    }

    /// Get filtered SEI messages
    pub fn filtered_sei<'a>(&self, metadata: &'a StreamMetadata) -> Vec<&'a SeiMessage> {
        metadata
            .sei_messages
            .iter()
            .filter(|m| {
                // Type filter
                if let Some(type_filter) = &self.filter.sei_type_filter {
                    if m.message_type != *type_filter {
                        return false;
                    }
                }

                // Text search (search in type name)
                if let Some(search) = &self.filter.text_search {
                    let search_lower = search.to_lowercase();
                    if !m.message_type.name().to_lowercase().contains(&search_lower) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Sort SEI messages
    pub fn sort_sei(&self, messages: &mut Vec<&SeiMessage>) {
        match self.sei_sort {
            SeiSortColumn::ByteOffset => {
                messages.sort_by_key(|m| m.byte_offset);
            }
            SeiSortColumn::FrameIdx => {
                messages.sort_by_key(|m| m.frame_idx.unwrap_or(usize::MAX));
            }
            SeiSortColumn::Type => {
                messages.sort_by_key(|m| format!("{:?}", m.message_type));
            }
            SeiSortColumn::Size => {
                messages.sort_by_key(|m| m.payload_size);
            }
        }

        if !self.sei_sort_ascending {
            messages.reverse();
        }
    }

    /// Generate summary text
    pub fn summary_text(&self, metadata: &StreamMetadata) -> String {
        let mut parts = Vec::new();

        if metadata.has_hdr() {
            if let Some(format) = &metadata.hdr_format {
                parts.push(format!("HDR: {}", format.name()));
            } else {
                parts.push("HDR: Yes".to_string());
            }
        }

        let sei_count = metadata.sei_messages.len();
        if sei_count > 0 {
            parts.push(format!("SEI: {} messages", sei_count));
        }

        if parts.is_empty() {
            "No metadata".to_string()
        } else {
            parts.join(" | ")
        }
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("metadata_test.rs");
